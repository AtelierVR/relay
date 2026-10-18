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

### Server transfer

A relay can ask a connected client to move to another relay while keeping its identity
(world, room, user). The client carries an opaque **transfer token** — a client-unique
byte sequence issued elsewhere (the node) — and presents it in its next handshake, instead of
running the usual challenge-response exchange.

No configuration is required: the relay neither issues nor signs tokens, it only relays the one
it is given.

Issue a transfer from the relay console:

```
transfer <client id|ip:port> <host:port> <token> [reason...]
```

The `token` is read as hex when the argument looks like hex, and as its UTF-8 bytes otherwise.
The client receives a `Transfer` packet, disconnects, reconnects to the address, and presents the
token in its handshake.

> Token resolution is not implemented yet: the receiving relay rejects every token for now
> ([`transfer::consume`](../src/transfer.rs) always returns `None`), so the client falls back to
> the normal authentication. It will later resolve to a `TransferResult` (which room, which
> user, …), at which point the relay answers with the `Transferred` handshake flag and the client
> skips authentication (its identity still comes from the room `enter` response). Clients that do
> not implement the protocol simply ignore the request (the token is optional in the handshake).

### Auto-enter (default room)

Clients normally pick their room themselves: they list the rooms and join the one matching the
identifier they were given. A relay can advertise a **default room** instead, so the client joins
it when it has no valid target of its own — this is what makes a standalone relay (a single room,
nothing to choose from) and a "hub" work with clients that were never told about them.

The handshake response carries it (`HandshakeFlags.AutoRoom` + a trailing `u8`, `0xFF` = none):

- a relay hosting a **single room** always advertises it;
- otherwise the first configured room flagged `hub` is used:

```json
{
  "instances": [
    { "capacity": 0, "hub": true, "world": "w:123@example.com" },
    { "capacity": 40, "world": "w:123@example.com" }
  ]
}
```

The client only falls back to it: when its requested room is found, that one wins. A transfer
reuses the same path, so a transferred client lands on the target relay's default room when
the identifier it carries means nothing there (typically a group of standalone relays).

### Moving between rooms

A client can be moved to another room of the **same** relay (no reconnection, no token, the
loaded world is reused):

```
move <client id|ip:port> <room> [reason...]
```

`<room>` is the room id as listed by the relay (the `iid` shown by `instances`). The client quits
its current room and joins the target one on the same transport. A room of another world requires a
`transfer` (the bundle would have to be downloaded).

A `Transfer` packet (`0x17`) starts with an operation byte that selects the layout:

```
[length: u16][state: u16][type: u8 = 0x17][op: u8] …
  op = 0 (server) → [address: string][token: prefixed bytes][reason: string]
  op = 1 (room)   → [room: u8][reason: string]
```

`op = 0` changes relay (the target picks the room, see auto-enter above), `op = 1` moves the client
to another room of the current relay without reconnecting.

#### Who moves the client?

The client owns its connection: the relay can only *ask* it to leave or change room, then confirm.
A modified client could ignore the packet — which is why the request is made **enforceable**:

- `transfer_timeout` (`NOX_TRANSFER_TIMEOUT`, seconds, default `15`, `0` = never) is the grace
  period given to the client;
- while a transfer is pending, the relay refuses that client any room other than the target
  (`EnterResult::Refused`, reason "You are being moved to room N") — it cannot stay or come back;
- once the deadline passes, the relay applies it itself: the player is removed from its rooms
  (a `Leave` is broadcast to the others) and the QUIC connection is closed.

The pending transfer is dropped as soon as the client complies (it joins the target room, or its
connection goes away for a relay change). Enforcement is a per-second sweep, so a compliant client
never notices it.

### Packet rate limiting

A player is supposed to send its traffic at the instance TPS (transforms, properties, events, voice),
but the relay cannot trust it. Each handler meters what it receives and the excess is **dropped**
(never a `Disconnect`: dropping is what the unreliable channels already do, and it never punishes a
client for a latency spike).

The metering is generic — a handler only picks **which bytes identify its traffic** and the rate:

```rust
// the handler owns its layout, so it owns the key: the payload ranges that identify its traffic
#[inline]
fn entity_part_key(packet: &Packet) -> u64 {
    // [iid][sub-type][entity: u32][part: u16] → one budget per (entity, part)
    rate::key(packet, &[(0, 1), (2, 4), (6, 2)])
}

if !rate::check(state, client_id, entity_part_key(&packet)) {
    return; // over budget, dropped
}
```

The check is the **first thing** a handler does, before any room/player/entity lookup: a dropped
packet only costs one client lock and one hash-map lookup. `rate::check` resolves the budget from
the client's room itself (the TPS), so there is no second lock to take.

`rate::key(packet, &groups)` takes `(offset, length)` byte ranges, so a key can hash exactly the
fields that matter and skip the rest: `&[]` gives one budget for the whole packet type (the "once
per tick" traffic), `&[(0, 2)]` one budget per room whatever the payload. Where the metering is
plugged:

| packet | key builder | one budget per |
|---|---|---|
| `Transform` (EntityPart) | `entity_part_key` — iid + entity + part | (entity, part) |
| `Transform` (ByPath) | `path_key` — iid only | **all paths together** |
| `Stream` | `channel_key` — iid + channel | voice channel |
| `Properties` | `entity_key` — iid + entity | entity |
| `PlayerUpdate` | `player_key` — iid + pid | player |
| `Event` | `event_key` — iid + name | event name |
| `Entities` | `op_key` — iid + op | operation |

Each of those helpers lives next to the handler that owns the layout — `rate.rs` only provides the
generic [`key`]/[`check`] primitives.

Everything else (handshake, authentication, enter/quit, traveling, …) is not limited: it is
lifecycle or rare control traffic.

```json
{
  "rate_limit": {
    "enabled": true,
    "burst_ms": 250
  }
}
```

The bucket capacity is `rate × burst_ms / 1000 + 1`: that is the **margin for latency**. A client
that was stalled (GC pause, lag spike, loss burst) can catch up with a short burst; a client that
constantly sends more than its budget runs out of tokens and loses the surplus. Buckets are reset
when the client enters or leaves a room, since the budget follows the room's TPS.

Because the key is per (entity, part) / per channel, a client flooding part 2 of its avatar only
loses that part's surplus: the other parts — and the same part id on another entity (a prop it
owns) — keep their own budget instead of sharing a pool the flood would empty. The number of
buckets per client is capped (`MAX_BUCKETS = 512`), so random ids cannot allocate them forever.

`clients <id>` shows the room and the drop counter, so an abusive client is visible without
disconnecting it.

