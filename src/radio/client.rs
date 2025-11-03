use super::packet_metadata::PacketMetadata;
use crate::utils::bounded_message::truncate_message;
use anyhow::Result;
use meshtastic::packet::{PacketDestination, PacketRouter};
use meshtastic::protobufs::{FromRadio, MeshPacket};
use meshtastic::types::NodeId;
use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;

// Simple error type for EchoRouter
#[derive(Debug)]
struct EchoError;

impl fmt::Display for EchoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Echo router error")
    }
}

impl std::error::Error for EchoError {}

// EchoRouter handles message echo callbacks from the meshtastic library.
// When a message is sent, the library echoes it back through handle_mesh_packet()
// for confirmation. This router logs the echo for debugging and visibility.
struct EchoRouter {
    node_id: NodeId,
}

impl EchoRouter {
    fn new(node_id: u32) -> Self {
        Self {
            node_id: node_id.into(),
        }
    }
}

impl PacketRouter<(), EchoError> for EchoRouter {
    fn handle_packet_from_radio(&mut self, _packet: FromRadio) -> Result<(), EchoError> {
        Ok(())
    }

    fn handle_mesh_packet(&mut self, packet: MeshPacket) -> Result<(), EchoError> {
        tracing::debug!(
            packet_id = packet.id,
            from = packet.from,
            to = packet.to,
            "Echoed sent message"
        );
        Ok(())
    }

    fn source_node_id(&self) -> NodeId {
        self.node_id
    }
}

pub struct Client {
    stream_api: Arc<Mutex<meshtastic::api::ConnectedStreamApi>>,
    my_node_num: u32,
    read_only: bool,
}

impl Client {
    pub fn new(
        stream_api: meshtastic::api::ConnectedStreamApi,
        my_node_num: u32,
        read_only: bool,
    ) -> Self {
        Self {
            stream_api: Arc::new(Mutex::new(stream_api)),
            my_node_num,
            read_only,
        }
    }

    pub fn my_node_num(&self) -> u32 {
        self.my_node_num
    }

    pub async fn send_message(&self, message: &str, coords: &PacketMetadata) -> Result<()> {
        // Truncate message to fit within Meshtastic's 200-byte limit
        let message = truncate_message(message);

        if self.read_only {
            tracing::info!(
                to_id = coords.to_id_hex(),
                message = message,
                is_dm = coords.is_dm,
                channel = coords.channel_index,
                "Dry-run: would send message"
            );
            return Ok(());
        }

        let destination = if coords.is_dm {
            PacketDestination::Node(coords.to_id.into())
        } else {
            PacketDestination::Broadcast
        };

        let mut api = self.stream_api.lock().await;
        let mut router = EchoRouter::new(self.my_node_num);

        api.send_text::<(), EchoError, EchoRouter>(
            &mut router,
            message.clone(),
            destination,
            true,
            coords.channel_index.into(),
        )
        .await?;

        tracing::info!(
            to_id = coords.to_id_hex(),
            message = message,
            is_dm = coords.is_dm,
            channel = coords.channel_index,
            "Message sent"
        );

        Ok(())
    }
}
