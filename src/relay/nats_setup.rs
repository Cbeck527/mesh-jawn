use anyhow::{Context, Result};

/// Connect to NATS server
pub async fn connect_nats(nats_url: &str, read_only: bool) -> Result<Option<async_nats::Client>> {
    if read_only {
        tracing::info!("READ-ONLY: Skipping NATS connection");
        Ok(None)
    } else {
        tracing::info!(url = %nats_url, "Connecting to NATS server");
        let client = async_nats::connect(nats_url)
            .await
            .context("Failed to connect to NATS server")?;
        tracing::info!("Connected to NATS server");
        Ok(Some(client))
    }
}

/// Ensure JetStream stream exists, creating if necessary
pub async fn ensure_stream(client: &async_nats::Client) -> Result<()> {
    let jetstream = async_nats::jetstream::new(client.clone());

    // Check if stream exists, create if not
    match jetstream.get_stream("cbmesh").await {
        Ok(_) => {
            tracing::info!("JetStream stream 'cbmesh' already exists");
        }
        Err(_) => {
            tracing::info!("Creating JetStream stream 'cbmesh'");

            use async_nats::jetstream::stream::{
                Config as StreamConfig, DiscardPolicy, RetentionPolicy, StorageType,
            };

            let stream_config = StreamConfig {
                name: "cbmesh".to_string(),
                subjects: vec!["cbmesh.>".to_string()],
                max_age: std::time::Duration::from_secs(30 * 24 * 60 * 60), // 30 days
                storage: StorageType::File,
                retention: RetentionPolicy::Limits,
                discard: DiscardPolicy::Old,
                ..Default::default()
            };

            jetstream
                .create_stream(stream_config)
                .await
                .context("Failed to create JetStream stream")?;

            tracing::info!("JetStream stream 'cbmesh' created successfully");
        }
    }

    Ok(())
}

/// Ensure KV bucket exists, creating if necessary
/// Returns the KV store handle
pub async fn ensure_kv_bucket(
    client: &async_nats::Client,
) -> Result<async_nats::jetstream::kv::Store> {
    let jetstream = async_nats::jetstream::new(client.clone());

    // Get or create KV bucket
    let store = match jetstream.get_key_value("cbmesh_nodes").await {
        Ok(store) => {
            tracing::info!("KV bucket 'cbmesh_nodes' already exists");
            store
        }
        Err(_) => {
            tracing::info!("Creating KV bucket 'cbmesh_nodes'");

            jetstream
                .create_key_value(async_nats::jetstream::kv::Config {
                    bucket: "cbmesh_nodes".to_string(),
                    ..Default::default()
                })
                .await
                .context("Failed to create KV bucket 'cbmesh_nodes'")?
        }
    };

    tracing::info!("KV bucket 'cbmesh_nodes' ready");
    Ok(store)
}

/// Initialize MyNodeInfo in KV store
pub async fn initialize_my_node_info(
    kv_store: Option<&async_nats::jetstream::kv::Store>,
    my_node_info: &meshtastic::protobufs::MyNodeInfo,
    read_only: bool,
) -> Result<()> {
    if let Some(store) = kv_store {
        match serde_json::to_string(my_node_info) {
            Ok(json) => {
                tracing::info!(
                    my_node_num = my_node_info.my_node_num,
                    key = "self",
                    "Writing MyNodeInfo to KV store"
                );

                store
                    .put("self", json.into())
                    .await
                    .context("Failed to write MyNodeInfo to KV store")?;
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to serialize MyNodeInfo to JSON, skipping KV write"
                );
            }
        }
    } else if read_only {
        match serde_json::to_string(my_node_info) {
            Ok(json) => {
                tracing::info!(
                    key = "self",
                    json = %json,
                    "READ-ONLY: Would write MyNodeInfo to KV store"
                );
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to serialize MyNodeInfo to JSON"
                );
            }
        }
    }

    Ok(())
}

/// Publish JSON to NATS with standard headers
pub async fn publish_to_nats(
    client: &async_nats::Client,
    subject: String,
    json: String,
    my_node_num: u32,
    packet_timestamp: Option<u32>,
) -> Result<()> {
    let mut headers = async_nats::HeaderMap::new();

    // Add publish timestamp (when relay published to NATS)
    let publish_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    headers.insert("publish_timestamp", publish_timestamp.to_string().as_str());

    // Add relay node ID
    headers.insert("relay_node_id", format!("!{:08x}", my_node_num).as_str());

    // Add packet timestamp if available
    if let Some(packet_ts) = packet_timestamp {
        headers.insert("packet_timestamp", packet_ts.to_string().as_str());
    }

    // Publish with headers
    client
        .publish_with_headers(subject.clone(), headers, json.into())
        .await
        .context(format!("Failed to publish to NATS subject: {}", subject))
}
