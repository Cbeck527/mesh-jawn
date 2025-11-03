use meshtastic::protobufs::PortNum;

/// Convert PortNum enum to subject name
/// Returns subject like "TextMessageApp" or "UNKNOWN_APP" for unrecognized values
pub fn portnum_to_subject(portnum: i32) -> String {
    match PortNum::try_from(portnum) {
        Ok(port) => format!("{:?}", port), // Uses Debug trait: TextMessageApp, NodeinfoApp, etc.
        Err(_) => "UNKNOWN_APP".to_string(),
    }
}

/// Build NATS subject for MeshPacket based on payload variant
/// Returns subject like "cbmesh.packet.TextMessageApp" or "cbmesh.packet.ENCRYPTED"
pub fn mesh_packet_subject(mesh_packet: &meshtastic::protobufs::MeshPacket) -> String {
    use meshtastic::protobufs::mesh_packet::PayloadVariant;

    match &mesh_packet.payload_variant {
        Some(PayloadVariant::Decoded(data)) => {
            let portnum_name = portnum_to_subject(data.portnum);
            format!("cbmesh.packet.{}", portnum_name)
        }
        Some(PayloadVariant::Encrypted(_)) => "cbmesh.packet.ENCRYPTED".to_string(),
        None => "cbmesh.packet.UNKNOWN".to_string(),
    }
}

/// Extract packet timestamp from MeshPacket if available
/// Returns Unix timestamp in seconds, or None if not present
/// rx_time is a u32 where 0 means unset (never sent over radio)
pub fn extract_packet_timestamp(mesh_packet: &meshtastic::protobufs::MeshPacket) -> Option<u32> {
    if mesh_packet.rx_time != 0 {
        Some(mesh_packet.rx_time)
    } else {
        None
    }
}
