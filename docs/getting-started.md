# Getting Started

## Prerequisites

- Rust 2021 edition (stable)
- Docker (optional)
- A running [NoxVR Node](https://github.com/AtelierVR/node) instance

## Build & Run

```bash
cargo build --release
./target/release/noxrelay
```

## Docker

```bash
npm run compose          # Production (~15 MB optimized image)
npm run compose:dev      # Development (with debug symbols)
npm run compose:fast     # Quick rebuild for testing
```

See [docker/BUILD.md](../docker/BUILD.md) for detailed Docker documentation.

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

The relay can resolve the node gateway from a domain name via DNS TXT or `.well-known/nox`:

```
_nox.example.com. 300 IN TXT "mg=https://node.example.com:3042"
```
