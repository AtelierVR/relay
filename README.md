# Nox Relay Server

[![Build and Test](https://github.com/AtelierVR/relay/actions/workflows/ci.yml/badge.svg)](https://github.com/AtelierVR/relay/actions/workflows/ci.yml)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

High-performance QUIC relay server for the Nox platform.

## Quick Start

```bash
# Build and run
cargo build --release
./target/release/noxrelay

# Or with Docker
docker-compose up -d
```

## Configuration

Copy `config.example.json` to `config.json` and edit:

```json
{
  "port": 23032,
  "node_gateway": "http://localhost:3042",
  "use_address": "127.0.0.1:23032",
  "token": "",
  "max_instances": 3,
  "connection_timeout": 15,
  "keep_alive_interval": 5,
  "debug": false
}
```

### Automatic Node Discovery

The relay can automatically discover the node gateway URL from a domain name:

```rust
use noxrelay::utils::node_discovery::find_node_gateway;

// Discovers via DNS TXT record (_nox.example.com) or /.well-known/nox
let gateway = find_node_gateway("example.com").await?;
// Returns: "https://example.com:3042" or "http://node.example.com"

// Works with IP addresses
let gateway = find_node_gateway("192.168.1.100:3042").await?;
// Returns: "http://192.168.1.100:3042"
```

**DNS TXT Record Setup** (optional):
```
_nox.example.com. 300 IN TXT "mg=https://node.example.com:3042"
```

## Docker

```bash
npm run compose          # Production (optimized, ~15MB)
npm run compose:dev      # Development (with debug symbols)
npm run compose:fast     # Quick testing
```

See [docker/BUILD.md](docker/BUILD.md) for detailed Docker documentation.

## Development

```bash
# Format and lint
cargo fmt && cargo clippy -- -D warnings

# Run tests
cargo test
```

## Features

- QUIC transport with built-in encryption
- WebSocket connection to node server
- Multi-instance support
- RSA authentication + Argon2 hashing
- Real-time system monitoring
- Optimized for 1000+ concurrent clients

## License

AGPL-3.0 - Copyright © 2026 Hactazia

See [LICENSE](LICENSE) for details.
