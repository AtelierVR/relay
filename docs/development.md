# Development

## Commands

```bash
cargo fmt                          # Format code
cargo clippy -- -D warnings        # Lint
cargo test                         # Run tests
```

## Project Structure

| Path | Description |
|---|---|
| `src/main.rs` | Entry point, QUIC server setup |
| `src/instance/` | Instance lifecycle management |
| `src/player/` | Player session handling |
| `src/client/` | QUIC client connection logic |
| `src/master/` | WebSocket link to node server |
| `src/handlers/` | Packet handler dispatch |
| `src/commands/` | Relay commands (e.g. send-command) |
| `src/config.rs` | Configuration loading |
