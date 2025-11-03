use serde::{Deserialize, Serialize};

/// Message format for cbmesh.outbound subject.
/// Simplified TX format that relay translates to radio API calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    /// Target node number (u32)
    pub to: u32,

    /// Channel index (typically 0)
    pub channel: u8,

    /// Message text content
    pub text: String,

    /// Whether this is a direct message (vs broadcast)
    pub is_dm: bool,
}
