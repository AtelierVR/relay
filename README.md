<div align="center">
  <img src="logo.png" width="320" alt="NoxVR" />
  <h1>Relay</h1>
  <p>High-performance QUIC relay server for NoxVR game instances.</p>

  [![CI](https://github.com/AtelierVR/relay/actions/workflows/ci.yml/badge.svg)](https://github.com/AtelierVR/relay/actions/workflows/ci.yml)
  ![Rust](https://img.shields.io/badge/Rust-2021-f74c00?logo=rust&logoColor=white)
  ![QUIC](https://img.shields.io/badge/Transport-QUIC-6366f1)
  ![Docker](https://img.shields.io/badge/Docker-ready-2496ed?logo=docker&logoColor=white)
  ![License](https://img.shields.io/badge/License-AGPL--3.0-22c55e)

  <p>Part of the <a href="https://github.com/AtelierVR"><strong>NoxVR</strong></a> ecosystem</p>
</div>

---

## Overview

**NoxVR Relay** is a high-throughput relay written in Rust that manages real-time multiplayer sessions for the NoxVR platform. It bridges QUIC-connected game clients with the central node server over WebSocket.

## Features

- **QUIC transport** — encrypted, low-latency connections via `quinn`
- **Multi-instance** — manages multiple concurrent game world instances
- **RSA + Argon2 authentication** — secure client and server handshake
- **WebSocket bridge** — real-time communication with the node server
- **Automatic node discovery** — via DNS TXT record or `.well-known/nox`
- **System monitoring** — real-time resource usage streaming
- **Optimized** — designed to handle 1000+ concurrent clients

## Documentation

- [Getting Started](docs/getting-started.md) — build, Docker, configuration
- [Development](docs/development.md) — commands and project structure

---

<div align="center">
  <p>Made with ♥ by <a href="https://github.com/AtelierVR">AtelierVR</a> &nbsp;·&nbsp; <a href="https://www.gnu.org/licenses/agpl-3.0">AGPL-3.0</a></p>
  <p>Part of the <strong>NoxVR</strong> project — a federated social VR platform</p>
</div>
