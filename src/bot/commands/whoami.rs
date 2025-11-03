use super::super::replies::CommandResponse;
use super::CommandMetadata;
use crate::radio::packet_metadata::PacketMetadata;
use anyhow::Result;

pub static WHOAMI_COMMAND: CommandMetadata = CommandMetadata {
    name: "whoami",
    aliases: &[],
    description: "Respond with your node info",
    handler: whoami_command,
};

fn whoami_command(
    coords: &PacketMetadata,
    _argument: &str,
    _tracker: &crate::bot::node_tracker::NodeTracker,
) -> Result<CommandResponse> {
    let mut components = Vec::new();

    components.push(format!("node id: {}", coords.from_id_hex()));
    components.push(format!("snr: {:.2}", coords.rx_snr));
    components.push(format!("hops: {}", coords.hops_traveled));

    let response = components.join("\n");

    Ok(response.into())
}
