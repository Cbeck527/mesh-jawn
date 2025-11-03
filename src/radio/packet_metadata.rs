use meshtastic::protobufs::MeshPacket;

#[derive(Debug, Clone)]
pub struct PacketMetadata {
    pub from_id: u32,
    pub to_id: u32,
    pub message_id: Option<u32>,
    pub channel_index: u32,
    pub is_dm: bool,
    /// Signal-to-noise ratio of the received packet
    pub rx_snr: f32,
    /// Number of hops this packet traveled (hop_start - hop_limit)
    pub hops_traveled: u32,
}

impl PacketMetadata {
    pub fn from_packet(packet: &MeshPacket, my_node_num: u32) -> Self {
        let from_id = packet.from;
        let to_id = packet.to;
        let message_id = Some(packet.id);
        let channel_index = packet.channel;
        let is_dm = to_id == my_node_num;

        // Extract signal quality and hop information
        let rx_snr = packet.rx_snr;
        // Calculate how many hops this packet traveled
        // hop_start is the initial hop limit, hop_limit is what remains
        let hops_traveled = packet.hop_start.saturating_sub(packet.hop_limit);

        Self {
            from_id,
            to_id,
            message_id,
            channel_index,
            is_dm,
            rx_snr,
            hops_traveled,
        }
    }

    pub fn from_id_hex(&self) -> String {
        format!("!{:08x}", self.from_id)
    }

    pub fn to_id_hex(&self) -> String {
        format!("!{:08x}", self.to_id)
    }
}
