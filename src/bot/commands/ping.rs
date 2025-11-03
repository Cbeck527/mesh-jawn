use super::super::replies::CommandResponse;
use super::CommandMetadata;
use crate::radio::packet_metadata::PacketMetadata;
use anyhow::Result;

pub static PING_COMMAND: CommandMetadata = CommandMetadata {
    name: "ping",
    aliases: &[],
    description: "Ping pong the bot",
    handler: ping_command,
};

fn ping_command(
    _coords: &PacketMetadata,
    _argument: &str,
    _tracker: &crate::bot::node_tracker::NodeTracker,
) -> Result<CommandResponse> {
    Ok("pong".into())
}
