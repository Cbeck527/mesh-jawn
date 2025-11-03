use anyhow::{Context, Result};
use futures::StreamExt;
use meshtastic::api::ConnectedStreamApi;
use meshtastic::protobufs::FromRadio;
use tokio::sync::mpsc::UnboundedReceiver;

mod config;
mod handlers;
mod nats_setup;
mod outbound_message;
mod subjects;

pub use config::RelayConfig;
pub use outbound_message::OutboundMessage;

/// Main relay event loop
/// Receives radio packets and publishes to NATS JetStream
pub async fn run_relay(
    stream_api: ConnectedStreamApi,
    my_node_num: u32,
    my_node_info: meshtastic::protobufs::MyNodeInfo,
    mut decoded_listener: UnboundedReceiver<FromRadio>,
    config: RelayConfig,
) -> Result<()> {
    tracing::info!(
        my_node_id = format!("!{:08x}", my_node_num),
        nats_url = %config.nats_url,
        read_only = config.read_only,
        "Starting NATS JetStream relay"
    );

    // TODO: Handle radio connection drops/reconnection (currently exits on disconnect)
    // TODO: Add NATS authentication support (credentials file, token, TLS)
    // TODO: Consider summary stats logging instead of per-packet logs (add --quiet flag)

    // Create client for sending messages back to radio
    let client = crate::radio::client::Client::new(stream_api, my_node_num, config.read_only);

    // Connect to NATS
    let nats_client = nats_setup::connect_nats(&config.nats_url, config.read_only).await?;

    // Setup JetStream and KV bucket
    let kv_store = if let Some(ref client) = nats_client {
        nats_setup::ensure_stream(client).await?;
        let store = nats_setup::ensure_kv_bucket(client).await?;
        Some(store)
    } else {
        None
    };

    // Initialize MyNodeInfo in KV store
    nats_setup::initialize_my_node_info(kv_store.as_ref(), &my_node_info, config.read_only).await?;

    // Subscribe to outbound messages for TX
    let mut outbound_sub = if let Some(ref client) = nats_client {
        let sub = client
            .subscribe("cbmesh.outbound")
            .await
            .context("Failed to subscribe to cbmesh.outbound")?;
        tracing::info!("Subscribed to cbmesh.outbound for TX");
        Some(sub)
    } else {
        None
    };

    // Process incoming packets
    loop {
        tokio::select! {
            // RX path: receive from radio and publish to NATS
            result = decoded_listener.recv() => {
                let from_radio = match result {
                    Some(fr) => fr,
                    None => {
                        tracing::info!("Radio disconnected, relay shutting down");
                        break;
                    }
                };

                use meshtastic::protobufs::from_radio::PayloadVariant;

                match from_radio.payload_variant {
                    Some(PayloadVariant::Packet(mesh_packet)) => {
                        handlers::handle_mesh_packet(
                            mesh_packet,
                            nats_client.as_ref(),
                            my_node_num,
                            config.read_only,
                        )
                        .await?;
                    }
                    Some(PayloadVariant::MyInfo(my_info)) => {
                        tracing::warn!(
                            my_node_num = my_info.my_node_num,
                            "Received unexpected MyNodeInfo in event loop (should only happen at connection)"
                        );
                    }
                    Some(PayloadVariant::NodeInfo(node_info)) => {
                        handlers::handle_node_info(node_info, kv_store.as_ref(), config.read_only)
                            .await?;
                    }
                    Some(variant) => {
                        tracing::debug!(?variant, "Ignoring FromRadio variant");
                    }
                    None => {
                        tracing::debug!("Received FromRadio with no payload_variant");
                    }
                }
            }

            // TX path: receive from NATS and send to radio
            Some(nats_msg) = async {
                match outbound_sub.as_mut() {
                    Some(sub) => sub.next().await,
                    None => None,
                }
            } => {
                handlers::handle_outbound_message(nats_msg, &client, my_node_num, config.read_only).await;
            }
        }
    }

    Ok(())
}
