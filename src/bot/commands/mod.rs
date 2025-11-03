pub mod help;
pub mod info;
pub mod ping;
pub mod welcome;
pub mod whoami;

use super::node_tracker::NodeTracker;
use super::replies::CommandResponse;
use crate::radio::packet_metadata::PacketMetadata;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::LazyLock;

pub type CommandHandler = fn(&PacketMetadata, &str, &NodeTracker) -> Result<CommandResponse>;

pub struct CommandMetadata {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    pub handler: CommandHandler,
}

pub static COMMAND_REGISTRY: LazyLock<HashMap<&'static str, &'static CommandMetadata>> =
    LazyLock::new(|| {
        let mut registry = HashMap::new();

        for cmd in &[
            &help::HELP_COMMAND,
            &ping::PING_COMMAND,
            &welcome::WELCOME_COMMAND,
            &whoami::WHOAMI_COMMAND,
            &info::INFO_COMMAND,
        ] {
            registry.insert(cmd.name, *cmd);
            for alias in cmd.aliases {
                registry.insert(alias, *cmd);
            }
        }

        registry
    });

pub fn get_command(name: &str) -> Option<&'static CommandMetadata> {
    COMMAND_REGISTRY.get(name).copied()
}

pub fn all_commands() -> Vec<&'static CommandMetadata> {
    let mut commands: Vec<_> = COMMAND_REGISTRY.values().copied().collect();

    commands.sort_by_key(|cmd| cmd.name);
    commands.dedup_by_key(|cmd| cmd.name);

    commands
}
