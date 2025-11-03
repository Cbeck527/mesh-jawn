# Architecture

This document describes the system architecture, data flows, and design decisions in mesh-jawn.

## System Overview

mesh-jawn implements a bidirectional bridge between Meshtastic mesh networks and NATS JetStream, with an extensible command bot built on top. The system is designed around three core principles:

1. **Separation of Concerns**: Radio connectivity (relay) is isolated from application logic (bot)
2. **Message Persistence**: All mesh packets are stored in JetStream for 30 days
3. **Multi-Language Integration**: NATS enables consumers in any language

### Components

```
    Meshtastic Radio (TCP)
            ↕
      Relay Process
            ↕
      NATS JetStream
            ↕
External Consumers (bots, etc)
```

The relay maintains a single TCP connection to the radio and handles all packet serialization. The bot and any external consumers interact only with NATS, allowing multiple concurrent instances without radio contention.

## Core Components

### Relay (`src/relay/`)

The relay is a bidirectional bridge that handles all radio communication.

**Responsibilities:**

- Establish and maintain TCP connection to Meshtastic device
- Receive `FromRadio` packets (MeshPacket, NodeInfo, MyNodeInfo)
- Serialize packets to JSON and publish to NATS
- Store node information in NATS KV bucket
- Subscribe to outbound messages and transmit to radio

**Connection Flow:**

1. Connect to radio via TCP (`radio/connection.rs`)
2. Wait for MyNodeInfo packet to get node number
3. Connect to NATS server
4. Create/verify JetStream stream `cbmesh`
5. Create/verify KV bucket `cbmesh_nodes`
6. Write MyNodeInfo to KV bucket key `self`
7. Subscribe to `cbmesh.outbound` for TX
8. Enter event loop processing RX and TX concurrently

**Read-Only Mode:**

The `--read-only` flag enables testing without transmitting to the radio. In this mode:

- RX path still publishes to NATS
- TX path logs messages instead of transmitting

### Bot (`src/bot/`)

The bot processes commands sent to it via DM.

**Responsibilities:**

- Connect to NATS (no radio connection)
- Subscribe to `cbmesh.packet.TextMessageApp` for text messages
- Parse and route commands starting with `.` prefix
- Execute command handlers
- Publish replies to `cbmesh.outbound` for TX

**Startup Flow:**
1. Connect to NATS
2. Get JetStream context
3. Read MyNodeInfo from KV bucket key `self` (to get bot's node ID)
4. Load all nodes from KV bucket into NodeTracker
5. Create consumer for `cbmesh.packet.TextMessageApp` subject
6. Enter event loop processing messages

**Command Processing:**
1. Receive TextMessageApp packet from JetStream consumer
2. Extract PacketMetadata (from/to IDs, channel, SNR, hops)
3. Check if message is a DM (to_id matches bot's node number)
4. If DM and starts with `.`, parse command name
5. Look up command in COMMAND_REGISTRY
6. Spawn async task to execute handler (non-blocking)
7. Handler returns CommandResponse with reply text
8. Publish OutboundMessage to `cbmesh.outbound`
9. Ack JetStream message

### Radio Layer (`src/radio/`)

Shared abstractions for radio communication used by both relay and bot.

**Modules:**

- `connection.rs`: TCP connection setup, waits for MyNodeInfo
- `client.rs`: Message sending wrapper with DM/broadcast routing
- `packet_metadata.rs`: Extracts metadata from MeshPackets (coordinates, SNR, hops)

**Connection Abstraction:**
The `connect_to_radio()` function returns a tuple:
```rust
(my_node_num, stream_api, decoded_listener)
```

This provides:

- Node number for determining DM vs broadcast
- ConnectedStreamApi for sending messages
- UnboundedReceiver for receiving decoded packets

**Client Abstraction:**
The `Client` type wraps ConnectedStreamApi with Arc<Mutex<>> for thread-safe async sharing:

- Handles routing: DMs use `send_text()`, broadcasts use different logic
- Supports read-only mode (logs instead of transmitting)
- All sends are logged with metadata

### Utilities (`src/utils/`)

**bounded_message.rs:**
Truncates text messages to 200 bytes while preserving UTF-8 validity. Meshtastic has strict size limits, so this prevents packet fragmentation and transmission failures.

**logging.rs:**
Configures tracing-subscriber with two output formats:

1. Console format: Human-readable for terminal (default for TTY)
2. JSON format: Structured for log aggregation (default for non-TTY stderr)

Logging levels:

- `INFO`: Default, shows relay/bot activity and command execution
- `DEBUG`: Enabled with `--verbose`, shows packet details and internal state

## Data Flow

### RX Path: Radio → NATS

The receive path ingests packets from the radio and publishes to NATS.

**Step-by-step:**

1. **Radio sends FromRadio packet** via TCP connection
   - Packet types: MeshPacket, NodeInfo, MyNodeInfo, Config, etc.

2. **Relay receives via decoded_listener** (UnboundedReceiver)
   - Packets are pre-decoded by meshtastic-rust library

3. **Relay routes packet by type:**

   **MeshPacket** → `handle_mesh_packet()`:
   - Extract portnum (application type)
   - Construct subject: `cbmesh.packet.<PORTNUM>` or `cbmesh.packet.ENCRYPTED`
   - Serialize entire MeshPacket to JSON
   - Add NATS headers: publish_timestamp, relay_node_id, packet_timestamp
   - Publish to JetStream subject
   - Example subjects:
     - `cbmesh.packet.TextMessageApp` (portnum 1)
     - `cbmesh.packet.PositionApp` (portnum 3)
     - `cbmesh.packet.NodeinfoApp` (portnum 4)
     - `cbmesh.packet.ENCRYPTED` (encrypted packets)

   **NodeInfo** → `handle_node_info()`:
   - Extract node number
   - Format key as 8-character hex: `{node_num:08x}`
   - Serialize NodeInfo to JSON
   - Write to KV bucket `cbmesh_nodes` with hex key
   - Example: node 305419896 → key `1234abcd`

   **MyNodeInfo** (startup only):
   - Written to KV bucket key `self`
   - Contains relay's node number and metadata

4. **Bot (and consumers) subscribe to subjects:**
   - Bot subscribes to `cbmesh.packet.TextMessageApp` only
   - Other consumers can subscribe to any subject pattern
   - JetStream provides historical replay capability

**Message Headers:**
All published messages include NATS headers:
- `publish_timestamp`: Unix timestamp when relay published (seconds)
- `relay_node_id`: Relay's node ID in `!xxxxxxxx` format
- `packet_timestamp`: Original packet timestamp (MeshPackets only, if present)

**Performance:**
The relay is designed to be lightweight and fast:
- No message processing beyond serialization
- No database writes (NATS handles persistence)
- Async handling prevents blocking on NATS publish
- Exits immediately on NATS errors (fail-fast)

### TX Path: NATS → Radio

The transmit path allows external systems to send messages via the mesh.

**Step-by-step:**

1. **Consumer publishes OutboundMessage to `cbmesh.outbound`:**
   ```json
   {
     "to": 305419896,
     "channel": 0,
     "text": "Hello from NATS!",
     "is_dm": true
   }
   ```

2. **Relay receives message** via subscribed stream
   - Relay subscribes to `cbmesh.outbound` subject on startup
   - Messages received via async-nats subscriber

3. **Relay deserializes JSON** → OutboundMessage struct
   - Validates required fields (to, channel, text, is_dm)
   - Logs error and continues if deserialization fails

4. **Relay converts to PacketMetadata:**
   - from_id: relay's node number
   - to_id: target node from message
   - channel_index: from message
   - is_dm: from message (determines routing)

5. **Client sends via radio:**
   - If is_dm=true: send_text() with specific destination
   - If is_dm=false: broadcast to channel
   - Read-only mode: logs instead of transmitting

6. **Radio transmits** to mesh network
   - Meshtastic handles encryption and routing
   - No acknowledgment returned (fire-and-forget)

**Error Handling:**
- Deserialization errors: logged, message skipped
- Transmission errors: logged, message skipped
- No retries or acknowledgments (by design)

**Use Cases:**
- Bot replying to commands
- External systems sending notifications
- Automated messages from monitoring tools
- API gateways forwarding requests to mesh

## Message Routing

### Determining Message Type

**DM (Direct Message):**
A message is a DM if `to_id == my_node_num`. The bot only processes DMs.

**Broadcast:**
A message is broadcast if `to_id == 0xFFFFFFFF` (4294967295)

**Channel-specific:**
Non-DM messages are routed to specific channels (0-7). The bot ignores these.

### Bot Message Filtering

The bot applies multiple filters before processing:

1. **Subject filter**: Only subscribes to `cbmesh.packet.TextMessageApp`
2. **DM filter**: Checks `is_dm` flag (to_id == my_node_num)
3. **Prefix filter**: Only processes messages starting with `.` (COMMAND_PREFIX)
4. **Command lookup**: Verifies command exists in registry

Non-matching messages are acknowledged and skipped without processing.

### Packet Metadata

The `PacketMetadata` struct contains routing information:

```rust
pub struct PacketMetadata {
    pub from_id: u32,        // Sender node number
    pub to_id: u32,          // Recipient node number
    pub message_id: Option<u32>,  // Message ID
    pub channel_index: u32,  // Channel (0-7)
    pub is_dm: bool,         // DM flag
    pub rx_snr: f32,         // Signal quality
    pub hops_traveled: u32,  // Hop count
}
```

This metadata is:

- Extracted from MeshPacket by relay, serialized to JSON, and sent to NATS
- Deserialized by bot for command context
- Passed to command handlers for response routing

## Node Tracking

mesh-jawn maintains node information for mesh network awareness.

### KV Bucket Structure

**Bucket Name:** `cbmesh_nodes`

**Keys:**

- `self`: Relay's MyNodeInfo (JSON)
- `{node_num:08x}`: NodeInfo for each discovered node (JSON)

**Example:**
```
self         → MyNodeInfo (relay's own info)
12ab34cd     → NodeInfo for node 0x12ab34cd
deadbeef     → NodeInfo for node 0xdeadbeef
```

### NodeInfo Storage

When relay receives NodeInfo packet:

1. Extract node number
2. Format as 8-character lowercase hex
3. Serialize NodeInfo to JSON
4. Write to KV bucket with hex key

KV bucket provides:

- Persistent storage across restarts
- Revision history for debugging
- Real-time updates via watch API

### NodeTracker (Bot)

The bot maintains an in-memory tracker for fast lookups:

**Structure:**
```rust
pub struct NodeTracker {
    nodes: HashMap<u32, NodeInfo>,
}
```

**Initialization:**

On startup, bot loads all nodes from KV bucket:

1. Query all keys in `cbmesh_nodes`
2. Skip `self` key (relay info)
3. Parse 8-character hex keys as node numbers
4. Deserialize NodeInfo from JSON
5. Insert into HashMap

**Online Status:**
The tracker determines if a node is online based on `last_heard` timestamp (threshold: 2 hours)

**Usage:**
Command handlers receive `&NodeTracker` and can:

- Look up node information by ID
- Count online nodes
- Get node names and metadata
- Check signal quality history

### Updates

Node information updates automatically:
- Relay writes NodeInfo to KV on receipt
- Bot can reload from KV or maintain in-memory state
- KV watch API can enable real-time synchronization

## Error Handling/Exiting

### Relay

**Fatal errors (exit immediately):**

- Cannot connect to radio
- Cannot connect to NATS
- NATS publish fails
- Radio disconnects

**Non-fatal errors (log and continue):**

- Packet serialization fails (skip packet)
- Unknown packet type (log and ignore)


### Bot

**Fatal errors (exit immediately):**

- Cannot connect to NATS
- Cannot read MyNodeInfo from KV
- NATS consumer fails

**Non-fatal errors (log and continue):**

- Command handler returns error (log, no reply)
- Invalid message format (skip message)
- Non-DM message (ignore)
