# Bot

This guide covers all available bot commands and how to add new ones.

## Using Commands

Commands are sent via **direct messages (DMs)** to the bot's node on your Meshtastic device.

### Command Format

```
.command [arguments]
```

- **Prefix:** `.` (required)
- **Command:** Case-insensitive (`.help`, `.HELP`, `.Help` all work)
- **Arguments:** Optional, everything after command name
- **Whitespace:** Leading/trailing whitespace ignored

## Available Commands

```
.help        - List available commands
.ping        - Test connectivity
.whoami      - Show your node information
.welcome     - Welcome message with bot info
.info        - Current mesh network status
```

---

### help

Shows list of available commands with descriptions.

```
> .help

Available commands:
.help (?) - Show available commands
.ping - Test connectivity
.whoami - Show your node info
.welcome (hello, login) - Welcome message
.info (status) - Network status
```

**Response:** Multi-line message listing all commands with aliases and descriptions.

---

### ping

Simple connectivity test. Returns "pong" to verify bot is responsive.

```
> .ping
pong
```

**Response:** Single word "pong"

---

### whoami

Shows information about the sender's node including signal quality and hop count.

```
> .whoami

You are: MyNodeName (!12345678)
Signal: 9.5 dB SNR
Hops: 2
```

**Response:**
- Long name and node ID in hex format
- Signal-to-noise ratio (SNR) in dB
- Number of hops packet traveled

(SNR and hop count from most recent message, not real-time.)

---

### welcome

Displays welcome message with bot version and project information.

```
> .welcome

Welcome to mesh-jawn!
v0.0.1 - Meshtastic bot
Send .help for commands
GitHub: github.com/yourusername/mesh-jawn
```

**Response:** Multi-line welcome banner with:
- Bot name
- Version number
- Link to help
- Project URL

---

### info

Shows current mesh network status including time, date, node counts, and weather.

**Aliases:** `status`

**Response:**

- Current time and timezone
- Number of online nodes (heard within 2 hours)
- Total discovered nodes
- Current weather for Philadelphia

**Data sources:**

- Time: Bot's system clock
- Nodes: From NATS KV bucket `cbmesh_nodes`
- Weather: Open-Meteo API (https://open-meteo.com)

```
> .info

Current Time: 2025-01-15 14:32:15 EST
Mesh Status:
- Online Nodes: 12
- Total Nodes: 45

Weather (Philadelphia):
72°F, Partly Cloudy
```

---

## Adding New Commands

Follow these steps to add a new command to the bot.

### 1. Create Command File

Create `src/bot/commands/yourcommand.rs`:

```rust
use super::CommandMetadata;
use super::super::replies::CommandResponse;
use crate::radio::packet_metadata::PacketMetadata;
use crate::bot::node_tracker::NodeTracker;
use anyhow::Result;

pub static YOUR_COMMAND: CommandMetadata = CommandMetadata {
    name: "yourcommand",
    aliases: &["alias1", "alias2"],
    description: "Brief description of what your command does",
    handler: your_command_handler,
};

fn your_command_handler(
    coords: &PacketMetadata,
    argument: &str,
    tracker: &NodeTracker,
) -> Result<CommandResponse> {
    // Your command logic here
    let response = format!("Hello from {}", coords.from_id_hex());
    Ok(response.into())
}
```

### 2. Add Module Declaration

Edit `src/bot/commands/mod.rs` and add:

```rust
pub mod yourcommand;
```

### 3. Register Command

In `src/bot/commands/mod.rs`, add to `COMMAND_REGISTRY`:

```rust
lazy_static! {
    pub static ref COMMAND_REGISTRY: HashMap<&'static str, &'static CommandMetadata> = {
        let mut m = HashMap::new();
        m.insert(PING_COMMAND.name, &PING_COMMAND);
        m.insert(HELP_COMMAND.name, &HELP_COMMAND);
        // ... other commands ...
        m.insert(yourcommand::YOUR_COMMAND.name, &yourcommand::YOUR_COMMAND); // Add this line
        m
    };
}
```

### 4. Build and Test

```bash
# Build
cargo build

# Run bot
cargo run -- --verbose bot

# Test command
# Send DM to bot: .yourcommand
```
