use super::super::replies::CommandResponse;
use super::{CommandMetadata, all_commands};
use crate::radio::packet_metadata::PacketMetadata;
use anyhow::Result;

pub static HELP_COMMAND: CommandMetadata = CommandMetadata {
    name: "help",
    aliases: &["?"],
    description: "Show help menu",
    handler: help_command,
};

fn help_command(
    _coords: &PacketMetadata,
    _argument: &str,
    _tracker: &crate::bot::node_tracker::NodeTracker,
) -> Result<CommandResponse> {
    let mut help_text = String::from("Available commands:\n\n");

    for cmd in all_commands() {
        let aliases = if cmd.aliases.is_empty() {
            String::new()
        } else {
            format!(", .{}", cmd.aliases.join(", ."))
        };

        help_text.push_str(&format!(".{}{} - {}\n", cmd.name, aliases, cmd.description));
    }

    Ok(help_text.into())
}
