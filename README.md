# mesh-jawn

A Meshtastic bot and relay written in Rust that bridges Meshtastic mesh networks with a persistent streaming setup for storage and interactivity. Also, and most importantly, an excuse to mess around with :crab: Rust and learn what the heck [NATS](https://docs.nats.io/) is.

## Overview

mesh-jawn connects to Meshtastic radios via TCP and provides a relay and bot. They run as separate processes, communicating through [NATS](https://docs.nats.io/). This separation allows multiple bots (or other consumers) to operate concurrently without interfering with radio connectivity.

- **Relay**: Publishes all mesh packets to NATS JetStream with 30-day retention, enabling message persistence, historical replay, and multi-language integrations
- **Bot**: Responds to commands sent to the radio's DMs

### Key Features

- **Persistent Message History**: 30-day JetStream retention of all mesh packets (configurable)
- **Node Discovery**: Automatic tracking of mesh nodes in NATS KV store
- **Bidirectional Relay**: Receive from mesh → publish to NATS, subscribe from NATS → transmit to mesh
- **Command Bot**: Extensible command system

Also NATS has [clients for basically every language](https://docs.nats.io/using-nats/developer), so you can run the relay and write whatever you want in whatever you want!

## Project Structure

```text
mesh-jawn/
├── src/
│   ├── main.rs                 # Entry point and CLI
│   ├── radio/                  # Radio communication layer
│   ├── relay/                  # Relay mode implementation
│   ├── bot/                    # Bot mode implementation
│   └── utils/                  # Shared utilities
│       ├── logging.rs          # Logging configuration
│       └── bounded_message.rs  # Message truncation
└── docs/                       # Documentation
```

## Installation

> [!IMPORTANT]
> Right now there's a [bug](https://github.com/meshtastic/rust/issues/71) in the
> Meshtastic rust library that trips up the packet reader if there are emoji in
> a node name. Right now I'm used a patched version of the library locally, and all
> of my documentation assumes that you are doing the same.

```bash
# Clone this repo
git clone https://github.com/Cbeck527/mesh-jawn.git

# Clone the meshtastic-rust repository and apply patch
git clone https://github.com/meshtastic/meshtastic-rust.git
cd meshtastic-rust
git apply ../mesh-jawn/etc/temp-disable-malformed-packet-detection.patch
```

### Docker

``` bash
# ... Edit docker-compose.yaml to set your radio's IP address ...

# Start all services
docker compose up -d
```

Your Meshtastic radio needs to be accessible on your host network.

See [docs/deployment.md](docs/deployment.md) for detailed deployment options.


### Locally

**Prerequisites:**

- New-ish version of Rust
- NATS server with JetStream enabled
- meshtastic-rust library as sibling directory

```bash
# Install NATS server (macOS)
brew install nats-server

# Start NATS with JetStream
nats-server -js

# Build mesh-jawn
cd mesh-jawn
cargo build --release

# Run relay (separate terminal)
cargo run --release -- relay --radio 192.168.1.100

# Run bot (separate terminal)
cargo run --release -- bot
```

## Architecture

See [docs/architecture.md](docs/architecture.md) for detailed system design.

### Relay

Maintains the radio connection and acts as a bidirectional bridge:

- Connects to Meshtastic device via TCP
- Publishes to JetStream subjects: `cbmesh.packet.<PORTNUM>`
- Subscribes to `cbmesh.outbound` subject
- Forwards messages to radio for mesh transmission

The relay also reads the radio's NodeDB on connection and stores this in the
NATS KV. NodeInfo packets received from the mesh updates (or adds) the
associated K/V entry.

### Bot

Processes DMs and responds. It subscribes to `cbmesh.packet.TextMessageApp` for
incoming text messages.


## Usage

``` bash
$ mesh-jawn --help
Meshtastic bot and relay

Usage: mesh-jawn [OPTIONS] <COMMAND>

Commands:
  bot    Run as interactive bot responding to commands
  relay  Relay all packets to NATS JetStream
  help   Print this message or the help of the given subcommand(s)

Options:
      --verbose                  Enable verbose (DEBUG) logging
      --log-format <LOG_FORMAT>  Log format (console or json) [default: console] [possible values: console, json]
  -h, --help                     Print help
```

### Running the Relay

```bash
$ mesh-jawn relay --help
Relay all packets to NATS JetStream

Usage: mesh-jawn relay [OPTIONS] --radio <RADIO>

Options:
      --radio <RADIO>            Hostname or IP address of Meshtastic radio
      --radio-port <RADIO_PORT>  TCP port for radio connection [default: 4403]
      --nats-url <NATS_URL>      NATS server URL [default: nats://localhost:4222]
      --read-only                Read-only mode: log instead of transmitting
  -h, --help                     Print help
```

### Running the Bot

```bash
$ mesh-jawn bot --help
Run as interactive bot responding to commands

Usage: mesh-jawn bot [OPTIONS]

Options:
      --nats-url <NATS_URL>  NATS server URL [default: nats://localhost:4222]
  -h, --help                 Print help
```

## NATS Integration

See [docs/nats.md](docs/nats-integration.md) for more info about NATS.

## FAQ

### Why Separate Relay and Bot?

I ran into issues when multiple clients were trying to talk to the radio at the same time.

### Why NATS w/ JetStream?

I messed around with MQTT first, but wanted something with persistence. I like making graphs and stuff
so I also wanted a way to write a quick-and-dirty script to say something like "give me all of the mesh
packets from the last hour!"

I ended up reading about NATS and it seemed to fit the bill! The KV storage was a bonus, and that's what
made me mess around with the node tracking stuff.

### Why Rust?

Why not? lol

### Is this overkill?

Probably.

## Contributing

Contributions are welcome! This is an early-stage project and feedback is appreciated.

## Links and Inspiration

- **Meshtastic**: https://meshtastic.org
- **NATS**: https://nats.io

Many thanks to these projects for inspiration and code snippets:

- https://github.com/morria/Hops
- https://github.com/kstrauser/frozenbbs/
- https://github.com/TheCommsChannel/TC2-BBS-mesh
- https://github.com/deadchannelsky/BBMesh
- https://github.com/l5yth/potato-mesh

## TODO

- [ ] Automatic radio reconnection on disconnect
- [ ] Additional bot commands (weather, node stats, etc.)
- [ ] NATS authentication support
- [ ] Support for multiple radios
