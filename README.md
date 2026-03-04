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
  "relay_address": "0.0.0.0:30000",
  "master_url": "ws://localhost:3000/relay",
  "relay_id": "relay-01",
  "max_clients": 1000,
  "debug": false
}
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
- WebSocket connection to master server
- Multi-instance support
- RSA authentication + Argon2 hashing
- Real-time system monitoring
- Optimized for 1000+ concurrent clients

## License

AGPL-3.0 - Copyright © 2026 Hactazia

See [LICENSE](LICENSE) for details.
