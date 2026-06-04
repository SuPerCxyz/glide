# Glide

LAN-first, server-fallback cross-node clipboard sync tool with optional keyboard/mouse sharing.

## Overview

Glide synchronizes clipboard content across your trusted devices. It prefers direct LAN transfer for speed, falls back to the central server when devices aren't on the same network, and supports one-off temporary CLI sessions without persisting credentials.

**User-visible clipboard types:** text, image, file/folder. Rich text, URLs, and colors are preserved internally as MIME representations for paste fidelity.

## Features

- **LAN-first routing** — mDNS/UDP multicast discovery, direct peer-to-peer transfer
- **Server fallback** — WebSocket sync and HTTP payload relay when LAN isn't available
- **Temporary CLI sessions** — `--server` + `--token` one-off auth, no config written
- **Full-node sync** — copying on any trusted node syncs to all trusted nodes
- **Optional keyboard/mouse sharing** — LAN direct mode with server relay fallback, emergency release controls
- **Per-device and per-type sync policies** — fine-grained control over what syncs where
- **Docker deployment** — single-compose server with retention and capacity cleanup

## Architecture

```
┌──────────────────┐       LAN (mDNS/UDP)       ┌──────────────────┐
│  glide-desktop   │◄──────────────────────────►│  glide-desktop   │
│  (Linux/Windows) │                            │  (Windows)       │
└────────┬─────────┘                            └────────┬─────────┘
         │                                               │
         │  WebSocket/HTTP (TLS)                        │
         ▼                                               ▼
    ┌─────────────────────────────────────────────────────────┐
    │                 glide-server (Docker)                   │
    │                                                         │
    │  Device Registry │ Clipboard Relay │ Temporary Auth     │
    │  SQLite │ Filesystem Payload Store │ Input Relay (opt.) │
    └─────────────────────────────────────────────────────────┘
         ▲
         │  --server + --token (one-off, no config written)
         │
┌────────┴────────┐
│   glide-cli     │
│  (headless)     │
└─────────────────┘
```

### Transfer Route Priority

1. **LAN Direct** — peer-to-peer on the same network (lowest latency)
2. **LAN Reverse Pull** — target pulls from source over LAN
3. **Server Fallback** — relayed through the central server

### Input Route Priority

1. **LAN Direct** — lowest latency for keyboard/mouse
2. **Server Relay** — when devices aren't on the same LAN
3. **Disconnect & Release** — when both fail, input control is released

## Project Structure

| Crate | Description |
|-------|-------------|
| [`glide-core`](crates/glide-core) | Shared types: device identity, clipboard items, MIME representations, payload refs, transfer sessions, sync events, input events, policies |
| [`glide-server`](crates/glide-server) | Docker-deployable central server: Axum HTTP/WebSocket API, SQLite device registry, filesystem payload store, temporary token auth, input relay, periodic cleanup |
| [`glide-cli`](crates/glide-cli) | Headless CLI tool: `glide copy`, `glide paste`, `glide history`, `glide devices` |
| [`glide-desktop`](crates/glide-desktop) | Desktop library: cross-platform clipboard adapter trait, input sharing module with edge crossing detection and rate limiting, sync policy UI state |

## Quick Start

### Server

```bash
# Start with Docker Compose
docker compose up -d

# Or build and run manually
cargo build --release --package glide-server
GLIDE_LISTEN_ADDR=0.0.0.0:8080 cargo run --package glide-server
```

### CLI

Persistent mode (requires `~/.config/glide/config.json`):

```bash
glide copy "hello world"
glide copy --file ./document.zip
glide copy --dir ./project
glide copy --image ./screenshot.png

glide paste
glide paste --output ./received_file

glide history --limit 50
glide devices
```

Temporary session (no config file written):

```bash
glide --server https://glide.example.com --token TEMP_TOKEN "hello"
glide --server https://glide.example.com --token TEMP_TOKEN paste --output ./recv
```

## Server Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `GLIDE_DATA_DIR` | `./data` | Server data directory (SQLite + payload files) |
| `GLIDE_MAX_STORAGE_BYTES` | `1073741824` | Maximum storage capacity (1 GB) |
| `GLIDE_RETENTION_DAYS` | `30` | Clipboard item retention period |
| `GLIDE_MAX_ITEM_BYTES` | `10485760` | Maximum single item size (10 MB) |
| `GLIDE_PUBLIC_URL` | `http://localhost:8080` | Public-facing server URL |
| `GLIDE_ADMIN_TOKEN` | — | Admin authentication token |
| `GLIDE_REGISTRATION_TOKEN` | — | Required for trusted device registration |
| `GLIDE_TEMP_TOKEN_DEFAULT_TTL` | `3600` | Temporary token lifetime in seconds |
| `GLIDE_INPUT_RELAY_ENABLED` | `false` | Enable keyboard/mouse input relay |
| `GLIDE_INPUT_RELAY_MAX_LATENCY_MS` | `200` | Maximum relay latency before warning |
| `GLIDE_LISTEN_ADDR` | `0.0.0.0:8080` | Bind address |

## API Reference

### HTTP Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/v1/health` | Health check |
| `POST` | `/api/v1/devices/register` | Register a device |
| `GET` | `/api/v1/devices` | List all devices |
| `POST` | `/api/v1/tokens/validate` | Validate a temporary token |
| `GET` | `/api/v1/clipboard/history` | Query clipboard history |
| `POST` | `/api/v1/payload/upload` | Upload a payload (multipart) |
| `GET` | `/api/v1/payload/{id}` | Download a payload |
| `POST` | `/api/v1/cleanup` | Trigger retention/capacity cleanup |

### WebSocket Endpoints

| Path | Description |
|------|-------------|
| `/ws/sync` | Clipboard sync channel (trusted devices) |
| `/ws/input` | Input relay channel (trusted devices only, requires `GLIDE_INPUT_RELAY_ENABLED=true`) |

### Clipboard Event Fields

| Field | Type | Description |
|-------|------|-------------|
| `item_id` | `string` | Unique item identifier |
| `source_device_id` | `string` | Originating device |
| `source_session_type` | `persistent \| temporary` | Session that created the item |
| `kind` | `text \| image \| file` | User-visible clipboard type |
| `representations` | `array` | MIME representations array |
| `size` | `number` | Total payload size in bytes |
| `created_at` | `number` | Epoch milliseconds |
| `checksum` | `string` | SHA-256 of primary representation |
| `delivery_policy` | `broadcast \| targeted \| local_only` | Sync delivery behavior |

## Security Model

- **Transport encryption** — all client-server and client-client communication uses TLS/WSS
- **Server storage** — clipboard stored plaintext on server; filesystem payloads for binary data
- **Persistent devices** — register with `GLIDE_REGISTRATION_TOKEN`, store credentials locally
- **Temporary tokens** — TTL-bound, max-use-limited, operation-restricted, size-limited
- **Temporary sessions** — never join the trusted device mesh, default to server-only transfer
- **Input relay** — restricted to trusted persistent devices only; emergency release always available
- **Path traversal protection** — payload IDs validated against `/` and `..`

## Build & Test

```bash
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Check without building
cargo check --workspace

# Run server
cargo run --package glide-server

# Run CLI
cargo run --package glide-cli -- copy "test"
```

## Docker Compose

```yaml
# docker-compose.yml (included)
services:
  glide-server:
    build: .
    ports:
      - "8080:8080"
    environment:
      - GLIDE_DATA_DIR=/data
      - GLIDE_RETENTION_DAYS=30
      - GLIDE_INPUT_RELAY_ENABLED=false
    volumes:
      - glide-data:/data
    restart: unless-stopped

volumes:
  glide-data:
```

## Clipboard Route Flow

```
Copy on Device A
       │
       ▼
┌─────────────────┐
│ Local loopback? │ ── Yes ──► Discard (prevent echo)
└────────┬────────┘
         │ No
         ▼
┌─────────────────┐
│ LAN peer found? │ ── Yes ──► LAN Direct transfer
└────────┬────────┘
         │ No
         ▼
┌───────────────────────┐
│ Server reachable?     │ ── Yes ──► Server Fallback relay
└────────┬──────────────┘
         │ No
         ▼
    Queue for retry
```

## Keyboard/Mouse Sharing

Disabled by default. When enabled:

- **LAN Direct** — lowest latency, preferred route
- **Server Relay** — higher latency, status indicator shows degraded
- **Emergency Release** — immediate disconnect on any node
- **Hotkey switch** — toggle between LAN and server modes
- **Edge crossing** — cursor crossing screen edge switches control to next monitor's device
- **Rate limiting** — caps events per second to prevent overload
- **Latency warning** — alerts when relay latency exceeds threshold

## License

MIT
