use anyhow::Result;
use meshtastic::api::{ConnectedStreamApi, StreamApi};
use meshtastic::protobufs::{FromRadio, MyNodeInfo, from_radio};
use meshtastic::utils;
use tokio::sync::mpsc::UnboundedReceiver;

/// Radio connection information
pub struct RadioConnection {
    pub my_node_num: u32,
    pub my_node_info: MyNodeInfo,
    pub stream_api: ConnectedStreamApi,
    pub decoded_listener: UnboundedReceiver<FromRadio>,
}

/// Establishes TCP connection to Meshtastic device and waits for node info
pub async fn connect_to_radio(tcp_address: String) -> Result<RadioConnection> {
    tracing::info!(address = %tcp_address, "Connecting to Meshtastic device");

    let stream_api = StreamApi::new();
    let tcp_stream = utils::stream::build_tcp_stream(tcp_address.clone()).await?;
    let (mut decoded_listener, stream_api) = stream_api.connect(tcp_stream).await;

    let config_id = utils::generate_rand_id();
    let stream_api = stream_api.configure(config_id).await?;

    tracing::info!(
        interface = "tcp",
        address = %tcp_address,
        "Connected to Meshtastic device"
    );

    // Wait for MyInfo to get node number
    let (my_node_num, my_node_info) = loop {
        if let Some(decoded) = decoded_listener.recv().await
            && let Some(from_radio::PayloadVariant::MyInfo(info)) = decoded.payload_variant
        {
            let node_num = info.my_node_num;
            tracing::info!(
                node_id = format!("!{:08x}", node_num),
                "Received node information"
            );
            break (node_num, info);
        }
    };

    Ok(RadioConnection {
        my_node_num,
        my_node_info,
        stream_api,
        decoded_listener,
    })
}
