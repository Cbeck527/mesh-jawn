use super::super::replies::CommandResponse;
use super::CommandMetadata;
use crate::radio::packet_metadata::PacketMetadata;
use anyhow::Result;

pub static WELCOME_COMMAND: CommandMetadata = CommandMetadata {
    name: "welcome",
    aliases: &["hello", "login"],
    description: "Display welcome message",
    handler: welcome_command,
};

fn welcome_command(
    _coords: &PacketMetadata,
    _argument: &str,
    _tracker: &crate::bot::node_tracker::NodeTracker,
) -> Result<CommandResponse> {
    let welcome_text = concat! {
        "WELCOME TO MESH JAWN v0.0.1",
        "\n\n",
        "🤖 I'm a bot on the Philly Meshtastic network written in Rust. ",
        "Check me out on Github: cbeck527/mesh-jawn",
        "\n\n",
        "Send .help for a list of commands!"
    };

    Ok(welcome_text.into())
}
