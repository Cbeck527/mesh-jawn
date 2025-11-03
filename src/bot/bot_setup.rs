use super::node_tracker::NodeTracker;
use anyhow::{Context, Result};
use async_nats::{Client, jetstream};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Bot state initialized from NATS
pub struct BotState {
    pub client: Client,
    pub jetstream: jetstream::Context,
    pub my_node_num: u32,
    pub tracker: Arc<RwLock<NodeTracker>>,
}

/// Connect to NATS and initialize bot state
pub async fn connect_nats(nats_url: &str) -> Result<BotState> {
    // Connect to NATS
    let client = async_nats::connect(nats_url)
        .await
        .context("Failed to connect to NATS")?;

    let jetstream = jetstream::new(client.clone());

    tracing::info!("Connected to NATS at {}", nats_url);

    // Read my_node_num from KV bucket
    let kv = jetstream
        .get_key_value("cbmesh_nodes")
        .await
        .context("Failed to get cbmesh_nodes KV bucket")?;

    let self_entry = kv
        .get("self")
        .await
        .context("Failed to get 'self' from KV bucket")?
        .context("'self' key not found in KV bucket - is relay running?")?;

    let my_info: meshtastic::protobufs::MyNodeInfo =
        serde_json::from_slice(&self_entry).context("Failed to deserialize MyNodeInfo from KV")?;

    let my_node_num = my_info.my_node_num;
    tracing::info!("Bot node ID: !{:08x}", my_node_num);

    // Initialize NodeTracker from KV bucket
    let tracker = Arc::new(RwLock::new(NodeTracker::new()));
    load_nodes_from_kv(&kv, &tracker).await?;

    Ok(BotState {
        client,
        jetstream,
        my_node_num,
        tracker,
    })
}

/// Load all nodes from KV bucket into tracker
async fn load_nodes_from_kv(
    kv: &jetstream::kv::Store,
    tracker: &Arc<RwLock<NodeTracker>>,
) -> Result<()> {
    // Get all keys from bucket
    let mut keys = kv.keys().await?.boxed();

    let mut node_count = 0;
    while let Some(key) = keys.next().await {
        let key = key?;

        // Skip 'self' key
        if key == "self" {
            continue;
        }

        // Keys that are 8 hex digits are node IDs
        if key.len() != 8 || !key.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }

        // Get value
        if let Some(entry) = kv.get(&key).await?
            && let Ok(node_info) = serde_json::from_slice::<meshtastic::protobufs::NodeInfo>(&entry)
        {
            tracker.write().await.update_node(node_info);
            node_count += 1;
        }
    }

    tracing::info!("Loaded {} nodes from KV bucket", node_count);
    Ok(())
}

/// Publish outbound message to NATS
pub async fn send_message(
    client: &Client,
    to: u32,
    channel: u8,
    text: String,
    is_dm: bool,
) -> Result<()> {
    let outbound = crate::relay::OutboundMessage {
        to,
        channel,
        text,
        is_dm,
    };

    let payload = serde_json::to_vec(&outbound).context("Failed to serialize outbound message")?;

    client
        .publish("cbmesh.outbound", payload.into())
        .await
        .context("Failed to publish to cbmesh.outbound")?;

    Ok(())
}
