use crate::radio::packet_metadata::PacketMetadata;
use crate::utils::bounded_message::truncate_message;
use anyhow::{Context, Result};
use futures::StreamExt;

mod bot_setup;
pub mod commands;
mod node_tracker;
pub mod replies;

use bot_setup::BotState;
use commands::get_command;

/// Command prefix for bot commands
const COMMAND_PREFIX: char = '.';

/// Extract text from MeshPacket if it's a TextMessageApp
fn extract_text_from_packet(packet: &meshtastic::protobufs::MeshPacket) -> Option<String> {
    use meshtastic::protobufs::mesh_packet::PayloadVariant;

    match &packet.payload_variant {
        Some(PayloadVariant::Decoded(data)) => {
            if data.portnum() == meshtastic::protobufs::PortNum::TextMessageApp {
                String::from_utf8(data.payload.clone()).ok()
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Handle incoming message and route to command handler
async fn handle_message(state: &BotState, coords: &PacketMetadata, message: &str) -> Result<()> {
    // ONLY respond to DMs right now
    if !coords.is_dm {
        return Ok(());
    }

    let parts: Vec<&str> = message.splitn(2, ' ').collect();
    let command_text = parts[0];
    if !command_text.starts_with(COMMAND_PREFIX) {
        return Ok(());
    }

    let command_name = &command_text[1..];
    let cmd = match get_command(command_name.to_lowercase().as_str()) {
        Some(cmd) => cmd,
        None => return Ok(()),
    };

    let argument = parts.get(1).unwrap_or(&"");

    tracing::info!(
        command = cmd.name,
        from_id = coords.from_id_hex(),
        argument = argument,
        "Command received"
    );

    let tracker_guard = state.tracker.read().await;
    let response = (cmd.handler)(coords, argument, &tracker_guard)?;

    // Send response
    for msg in response.messages {
        let text = truncate_message(&msg);
        let is_dm = coords.is_dm;

        if let Err(e) = bot_setup::send_message(
            &state.client,
            coords.from_id, // Reply to sender
            coords.channel_index as u8,
            text,
            is_dm,
        )
        .await
        {
            tracing::error!("Failed to publish reply: {}", e);
        }
    }

    Ok(())
}

/// Main bot event loop
/// Connects to NATS, subscribes to text messages, and processes commands
pub async fn run_bot(nats_url: String) -> Result<()> {
    // Connect to NATS and initialize bot state
    let state = bot_setup::connect_nats(&nats_url).await?;

    // Subscribe to TEXT_MESSAGE_APP packets
    let consumer = state
        .jetstream
        .get_stream("cbmesh")
        .await
        .context("Failed to get cbmesh stream - is relay running?")?
        .create_consumer(async_nats::jetstream::consumer::pull::Config {
            filter_subject: "cbmesh.packet.TextMessageApp".to_string(),
            deliver_policy: async_nats::jetstream::consumer::DeliverPolicy::New,
            ..Default::default()
        })
        .await
        .context("Failed to create consumer for TextMessageApp")?;

    let mut messages = consumer.messages().await?;

    tracing::info!("Bot started, listening for commands...");

    // Process messages
    while let Some(msg) = messages.next().await {
        let msg = msg?;

        // Deserialize MeshPacket
        let packet: meshtastic::protobufs::MeshPacket = match serde_json::from_slice(&msg.payload) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Failed to deserialize MeshPacket: {}", e);
                msg.ack()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to ack message: {}", e))?;
                continue;
            }
        };

        // Extract coordinates
        let coords = PacketMetadata::from_packet(&packet, state.my_node_num);

        // Only process DMs
        if !coords.is_dm {
            tracing::trace!("Skipping non-DM message");
            msg.ack()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to ack message: {}", e))?;
            continue;
        }

        // Extract text
        let text = match extract_text_from_packet(&packet) {
            Some(t) => t,
            None => {
                msg.ack()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to ack message: {}", e))?;
                continue;
            }
        };

        tracing::info!("Received DM from !{:08x}: {}", coords.from_id, text);

        // Handle message (spawn to avoid blocking message loop)
        tokio::spawn({
            let state_clone = BotState {
                client: state.client.clone(),
                jetstream: state.jetstream.clone(),
                my_node_num: state.my_node_num,
                tracker: state.tracker.clone(),
            };
            let coords_clone = coords.clone();
            let text_clone = text.clone();
            async move {
                if let Err(e) = handle_message(&state_clone, &coords_clone, &text_clone).await {
                    tracing::error!("Error handling message: {}", e);
                }
            }
        });

        // Ack message
        msg.ack()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to ack message: {}", e))?;
    }

    Ok(())
}
