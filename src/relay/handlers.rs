use super::nats_setup::publish_to_nats;
use super::outbound_message::OutboundMessage;
use super::subjects::{extract_packet_timestamp, mesh_packet_subject};
use anyhow::{Context, Result};

/// Handle outbound message from NATS (TX path: NATS → radio)
pub async fn handle_outbound_message(
    nats_msg: async_nats::Message,
    client: &crate::radio::client::Client,
    my_node_num: u32,
    _read_only: bool,
) {
    // Deserialize
    let outbound: OutboundMessage = match serde_json::from_slice(&nats_msg.payload) {
        Ok(msg) => msg,
        Err(e) => {
            tracing::error!("Failed to deserialize outbound message: {}", e);
            return;
        }
    };

    tracing::debug!(
        "Outbound message: to={:08x}, channel={}, is_dm={}, text_len={}",
        outbound.to,
        outbound.channel,
        outbound.is_dm,
        outbound.text.len()
    );

    // Create PacketMetadata for routing
    let coords = crate::radio::packet_metadata::PacketMetadata {
        from_id: my_node_num,
        to_id: outbound.to,
        message_id: None, // Not relevant for TX
        channel_index: outbound.channel as u32,
        is_dm: outbound.is_dm,
        rx_snr: 0.0,
        hops_traveled: 0,
    };

    // Send via radio client
    if let Err(e) = client.send_message(&outbound.text, &coords).await {
        tracing::error!("Failed to send message to radio: {}", e);
    } else {
        tracing::info!(
            "Sent message to !{:08x} on channel {} ({}): {}",
            outbound.to,
            outbound.channel,
            if outbound.is_dm { "DM" } else { "broadcast" },
            outbound.text
        );
    }
}

/// Handle MeshPacket from radio (RX path: radio → NATS)
pub async fn handle_mesh_packet(
    mesh_packet: meshtastic::protobufs::MeshPacket,
    nats_client: Option<&async_nats::Client>,
    my_node_num: u32,
    read_only: bool,
) -> Result<()> {
    let subject = mesh_packet_subject(&mesh_packet);

    // Serialize MeshPacket to JSON
    match serde_json::to_string(&mesh_packet) {
        Ok(json) => {
            tracing::info!(
                from_id = format!("!{:08x}", mesh_packet.from),
                to_id = format!("!{:08x}", mesh_packet.to),
                subject = %subject,
                "Relaying MeshPacket to NATS"
            );

            if read_only {
                tracing::info!(
                    subject = %subject,
                    json = %json,
                    "READ-ONLY: Would publish MeshPacket to NATS"
                );
            } else if let Some(client) = nats_client {
                let packet_timestamp = extract_packet_timestamp(&mesh_packet);

                publish_to_nats(client, subject.clone(), json, my_node_num, packet_timestamp)
                    .await
                    .map_err(|e| {
                        tracing::error!(
                            error = %e,
                            subject = %subject,
                            "Failed to publish MeshPacket to NATS"
                        );
                        e
                    })?;
            }
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                packet_id = mesh_packet.id,
                "Failed to serialize MeshPacket to JSON, skipping"
            );
        }
    }

    Ok(())
}

/// Handle NodeInfo from radio (RX path: radio → NATS KV)
pub async fn handle_node_info(
    node_info: meshtastic::protobufs::NodeInfo,
    kv_store: Option<&async_nats::jetstream::kv::Store>,
    read_only: bool,
) -> Result<()> {
    let key = format!("{:08x}", node_info.num);

    // Serialize NodeInfo to JSON
    match serde_json::to_string(&node_info) {
        Ok(json) => {
            tracing::info!(
                node_num = node_info.num,
                node_id = %key,
                "Writing NodeInfo to KV store"
            );

            if read_only {
                tracing::info!(
                    key = %key,
                    json = %json,
                    "READ-ONLY: Would write NodeInfo to KV store"
                );
            } else if let Some(store) = kv_store {
                store
                    .put(&key, json.into())
                    .await
                    .context(format!(
                        "Failed to write NodeInfo to KV store for key: {}",
                        key
                    ))
                    .map_err(|e| {
                        tracing::error!(
                            error = %e,
                            key = %key,
                            "Failed to write NodeInfo to KV store"
                        );
                        e
                    })?;
            }
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                node_num = node_info.num,
                "Failed to serialize NodeInfo to JSON, skipping"
            );
        }
    }

    Ok(())
}
