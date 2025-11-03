# NATS Integration

This guide covers NATS JetStream integration with mesh-jawn, including subjects, message formats, and basic consumer examples.

## Overview

mesh-jawn uses NATS JetStream for message persistence and delivery. All mesh packets are published to subjects and node information is stored in a KV bucket.

**Benefits of NATS integration:**

- 30-day message retention (configurable)
- Historical replay of missed messages
- Multi-language client support
- Pub/sub with filtering
- Persistent node information

**Components:**

- **Stream:** `cbmesh` - stores all mesh packets
- **KV Bucket:** `cbmesh_nodes` - stores node information
- **Subjects:** `cbmesh.packet.*` - packets by type
- **Outbound:** `cbmesh.outbound` - messages to transmit

## JetStream Subjects

### Subject Structure

All mesh packets are published to subjects following this pattern:

```
cbmesh.packet.<PORTNUM>
```

Where `<PORTNUM>` is the Meshtastic application type:

- `TextMessageApp` - Text messages (portnum 1)
- `PositionApp` - Location updates (portnum 3)
- `NodeinfoApp` - Node metadata (portnum 4)
- `RoutingApp` - Routing information (portnum 5)
- `AdminApp` - Admin commands (portnum 6)
- `ENCRYPTED` - Encrypted packets (unknown portnum)

### Common Subjects

**Text Messages:**
```
cbmesh.packet.TextMessageApp
```
All text messages sent on the mesh. The bot subscribes to this subject and looks for DMs.

**Position Updates:**
```
cbmesh.packet.PositionApp
```
GPS coordinates and location information from nodes.

**Node Information:**
```
cbmesh.packet.NodeinfoApp
```
Node metadata updates (name, hardware, firmware version).

**All Packets:**
```
cbmesh.packet.*
```
Wildcard subscription receives all packet types.

## KV Bucket Structure

**Bucket name:** `cbmesh_nodes`

This bucket stores node information for discovered mesh nodes.

### Key Format

**Relay's own node info:**
```
self
```
Contains MyNodeInfo with relay's node number and details.

**Other nodes:**
```
{node_num:08x}
```
8-character lowercase hexadecimal node number.

**Examples:**
```
self         → MyNodeInfo for relay
12ab34cd     → NodeInfo for node 0x12ab34cd (305419981)
deadbeef     → NodeInfo for node 0xdeadbeef (3735928559)
00000001     → NodeInfo for node 0x00000001 (1)
```

### Value Format

All values are JSON-serialized protobuf messages:

**MyNodeInfo (key: self):**
```json
{
  "my_node_num": 305419896,
  "reboot_count": 42,
  "min_app_version": 20300
}
```

**NodeInfo (key: hex node number):**
```json
{
  "num": 305419896,
  "user": {
    "id": "!12345678",
    "long_name": "My Node",
    "short_name": "MYN",
    "hw_model": 31
  },
  "position": {
    "latitude_i": 399631105,
    "longitude_i": -751577344,
    "altitude": 45,
    "time": 1730649600
  },
  "snr": 9.5,
  "last_heard": 1730649600
}
```

### Accessing KV Store via CLI

**List all keys:**
```bash
nats kv ls cbmesh_nodes
```

**Get relay's info:**
```bash
nats kv get cbmesh_nodes self
```

**Get specific node:**
```bash
# Convert node number to hex first
# Example: 305419896 = 0x12345678
nats kv get cbmesh_nodes 12345678
```

**Watch for updates:**
```bash
nats kv watch cbmesh_nodes
```

**Get revision history:**
```bash
nats kv history cbmesh_nodes 12345678
```

## Message Formats

### MeshPacket

Published to `cbmesh.packet.*` subjects.

**Complete example:**
```json
{
  "from": 305419896,
  "to": 4294967295,
  "channel": 0,
  "id": 123456789,
  "rx_time": 1730649600,
  "rx_snr": 9.5,
  "hop_limit": 3,
  "hop_start": 3,
  "want_ack": false,
  "priority": 1,
  "payload_variant": {
    "decoded": {
      "portnum": 1,
      "payload": [72, 101, 108, 108, 111],
      "want_response": false,
      "dest": 0,
      "source": 0,
      "request_id": 0
    }
  }
}
```

**Encrypted packet example:**
```json
{
  "from": 305419896,
  "to": 305419897,
  "channel": 0,
  "id": 123456790,
  "payload_variant": {
    "encrypted": [15, 234, 108, ...]
  }
}
```

**Key fields:**

- `from`: Sender node number (32-bit unsigned int)
- `to`: Recipient node number (or 0xFFFFFFFF for broadcast)
- `channel`: Channel index (0-7)
- `id`: Unique message ID
- `rx_time`: Receive timestamp (Unix seconds)
- `rx_snr`: Signal-to-noise ratio (dB)
- `hop_limit`: Remaining hops
- `hop_start`: Initial hop limit
- `payload_variant`: Either `decoded` or `encrypted`

### OutboundMessage

Published to `cbmesh.outbound` subject for the relay to send to the radio.

**Format:**
```json
{
  "to": 305419896,
  "channel": 0,
  "text": "Hello from NATS!",
  "is_dm": true
}
```

**Fields:**

- `to`: Recipient node number (or 0xFFFFFFFF for broadcast)
- `channel`: Channel index (0-7)
- `text`: Message text (UTF-8 string, max 200 bytes)
- `is_dm`: True for direct message, false for channel broadcast

**Examples:**

Direct message:
```json
{
  "to": 305419896,
  "channel": 0,
  "text": "Direct message to specific node",
  "is_dm": true
}
```

Broadcast:
```json
{
  "to": 4294967295,
  "channel": 0,
  "text": "Message to everyone on channel 0",
  "is_dm": false
}
```

### NATS Headers

Messages in the stream include metadata in NATS headers.

**Headers:**

- `publish_timestamp`: Unix timestamp when relay published (seconds)
- `relay_node_id`: Relay's node ID in `!xxxxxxxx` format
- `packet_timestamp`: Original packet timestamp (MeshPackets only, if present)

**Access headers with NATS CLI:**
```bash
nats sub "cbmesh.packet.*" --headers
```

**Example output:**
```
Subject: cbmesh.packet.TextMessageApp
Headers:
  publish_timestamp: 1730649600
  relay_node_id: !12345678
  packet_timestamp: 1730649595
Payload: {"from":305419896,"to":305419897,...}
```

## Playing around with the NATS CLI

Basic subscription:
```bash
# All packets
nats sub "cbmesh.packet.*"

# Text messages only
nats sub "cbmesh.packet.TextMessageApp"

# Multiple subjects (separate terminals)
nats sub "cbmesh.packet.TextMessageApp"
nats sub "cbmesh.packet.PositionApp"
```

### Consumer Options

**Show headers:**
```bash
nats sub "cbmesh.packet.*" --headers
```

**Count messages:**
```bash
nats sub "cbmesh.packet.*" --count
```

**Delivery policy:**
```bash
# Only new messages (default)
nats sub "cbmesh.packet.*" --deliver new

# All messages from beginning
nats sub "cbmesh.packet.*" --deliver all

# Last message per subject
nats sub "cbmesh.packet.*" --deliver last_per_subject
```

**Durable consumer:**
```bash
# Create durable consumer
nats consumer add cbmesh my_consumer \
  --filter "cbmesh.packet.*" \
  --deliver all \
  --replay instant

# Subscribe via durable consumer
nats consumer next cbmesh my_consumer --count 10
```

### Filtering

**By packet type:**
```bash
# Only text messages
nats sub "cbmesh.packet.TextMessageApp"

# Text and position
nats sub "cbmesh.packet.{TextMessageApp,PositionApp}"
```

**By sender (requires processing payload):**
NATS doesn't support filtering by message content. Use consumers to filter programmatically.

### Stream Info

**View stream details:**
```bash
nats stream info cbmesh
```

**View consumers:**
```bash
nats consumer ls cbmesh
```

**View consumer info:**
```bash
nats consumer info cbmesh my_consumer
```

## Publishing Messages

**Direct message:**
```bash
nats pub cbmesh.outbound '{
  "to": 305419896,
  "channel": 0,
  "text": "Hello from CLI!",
  "is_dm": true
}'
```

**Broadcast:**
```bash
nats pub cbmesh.outbound '{
  "to": 4294967295,
  "channel": 0,
  "text": "Broadcast message",
  "is_dm": false
}'
```

**Multi-line message:**
```bash
nats pub cbmesh.outbound "$(cat <<EOF
{
  "to": 305419896,
  "channel": 0,
  "text": "Multi-line message from shell",
  "is_dm": true
}
EOF
)"
```

### Node Number Conversion

Node IDs are displayed as `!xxxxxxxx` (hex) but must be decimal in JSON:

**Convert hex to decimal:**
```bash
# Using printf
printf "%d\n" 0x12345678
# Output: 305419896

# Using bc
echo "ibase=16; 12345678" | bc
# Output: 305419896

# Using Python
python3 -c "print(int('12345678', 16))"
# Output: 305419896
```

**Convert decimal to hex:**
```bash
# Using printf
printf "!%08x\n" 305419896
# Output: !12345678

# Using Python
python3 -c "print(f'!{305419896:08x}')"
# Output: !12345678
```

## Historical Replay

JetStream retains messages for 30 days, allowing historical replay.

### Replay All Messages

```bash
# All historical messages
nats sub "cbmesh.packet.*" --deliver all

# Historical text messages only
nats sub "cbmesh.packet.TextMessageApp" --deliver all
```

### Replay from Timestamp

```bash
# Messages since specific time
nats consumer add cbmesh replay_consumer \
  --filter "cbmesh.packet.*" \
  --deliver all \
  --replay instant \
  --max-deliver 1

# Fetch messages
nats consumer next cbmesh replay_consumer --count 100
```

### Replay Last N Messages

```bash
# Last 100 messages
nats consumer add cbmesh last_100 \
  --filter "cbmesh.packet.*" \
  --deliver last \
  --replay instant

nats consumer next cbmesh last_100 --count 100
```

### Stream Stats

```bash
# View stream statistics
nats stream info cbmesh

# Example output:
# Messages: 12,345
# Bytes: 45.2 MB
# First Sequence: 1
# Last Sequence: 12,345
# Consumer Count: 3
```

## Stream Configuration

### View Configuration

```bash
nats stream info cbmesh
```

**Default configuration:**
```
Name: cbmesh
Subjects: cbmesh.*
Storage: File
Retention: Limits
Max Age: 30 days (2,592,000 seconds)
Max Bytes: Unlimited
Max Messages: Unlimited
Max Message Size: Unlimited
Duplicate Window: 2 minutes
```

### Modify Configuration

**Change retention period:**
```bash
# 60 days
nats stream edit cbmesh --max-age=60d

# 7 days
nats stream edit cbmesh --max-age=7d

# 1 year
nats stream edit cbmesh --max-age=8760h
```

**Add storage limit:**
```bash
# Max 100 GB
nats stream edit cbmesh --max-bytes=100GB

# Max 10 million messages
nats stream edit cbmesh --max-msgs=10000000
```

**View changes:**
```bash
nats stream info cbmesh
```

### Delete Stream

**⚠️ Warning:** Deletes all messages permanently.

```bash
nats stream delete cbmesh
```

Relay will recreate stream on next startup with default configuration.

### Purge Messages

Delete all messages but keep stream configuration:

```bash
nats stream purge cbmesh
```

Delete messages matching filter:
```bash
nats stream purge cbmesh --subject "cbmesh.packet.TextMessageApp"
```

## Client Library Examples

While this guide focuses on NATS CLI, here are minimal examples for common languages:

### Python

```python
import asyncio
import json
from nats.js import JetStreamContext
import nats

async def main():
    # Connect
    nc = await nats.connect("nats://localhost:4222")
    js = nc.jetstream()

    # Subscribe to text messages
    sub = await js.subscribe("cbmesh.packet.TextMessageApp")

    async for msg in sub.messages:
        packet = json.loads(msg.data)
        print(f"From: !{packet['from']:08x}")
        print(f"Payload: {packet}")
        await msg.ack()

asyncio.run(main())
```

### JavaScript (Node.js)

```javascript
const { connect, JSONCodec } = require('nats');

async function main() {
  // Connect
  const nc = await connect({ servers: 'nats://localhost:4222' });
  const js = nc.jetstream();
  const jc = JSONCodec();

  // Subscribe to text messages
  const sub = await js.subscribe('cbmesh.packet.TextMessageApp');

  for await (const msg of sub) {
    const packet = jc.decode(msg.data);
    console.log(`From: !${packet.from.toString(16).padStart(8, '0')}`);
    console.log(`Payload:`, packet);
    msg.ack();
  }
}

main();
```

### Go

```go
package main

import (
    "encoding/json"
    "fmt"
    "log"

    "github.com/nats-io/nats.go"
)

func main() {
    // Connect
    nc, _ := nats.Connect("nats://localhost:4222")
    js, _ := nc.JetStream()

    // Subscribe to text messages
    sub, _ := js.Subscribe("cbmesh.packet.TextMessageApp", func(msg *nats.Msg) {
        var packet map[string]interface{}
        json.Unmarshal(msg.Data, &packet)

        from := packet["from"].(float64)
        fmt.Printf("From: !%08x\n", uint32(from))
        fmt.Printf("Payload: %+v\n", packet)

        msg.Ack()
    })

    defer sub.Unsubscribe()
    select {}
}
```

For complete examples, see NATS client library documentation for your language.
