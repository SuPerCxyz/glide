# Glide LAN-First Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build Glide, a LAN-first, server-fallback cross-node clipboard sync tool with optional keyboard/mouse sharing.

**Architecture:** Rust core owns protocol, device identity, discovery, routing, clipboard model, CLI, and platform adapters. Tauri wraps the desktop app for Linux/Windows. The Docker server provides registration, device registry, relay/fallback storage, temporary auth, cleanup, and optional input relay.

**Tech Stack:** Rust workspace, Tauri desktop, SQLite server metadata, filesystem object store, WebSocket/HTTP APIs, mDNS/UDP multicast LAN discovery, TLS/WSS transport.

---

## Summary

Glide uses full-node sync: copying on any trusted node syncs to all other trusted nodes according to policy. Normal nodes prefer LAN direct transfer, then LAN reverse pull, then server fallback. Temporary CLI sessions are for occasional use on unfamiliar machines and use command-provided auth without persisting credentials.

User-visible clipboard types are simple: text, image, file/folder. Internally, text may carry plain text, HTML/RTF rich text, URL, or color representations so paste fidelity is preserved without exposing extra product complexity.

Keyboard/mouse sharing is optional and disabled by default. It prefers LAN direct mode for low latency, but can fall back to server relay when devices are not on the same LAN or direct connection fails. Server relay mode must show degraded-latency status and keep emergency release controls available.

## Key Changes

- Create a Rust workspace with `glide-core`, `glide-server`, `glide-cli`, and `glide-desktop`.
- Implement a MIME representation model for clipboard items.
- Implement persistent device registration for trusted clients and temporary token auth for one-off CLI use.
- Implement LAN-first routing for trusted clipboard nodes and server-only routing for temporary CLI sessions.
- Implement optional keyboard/mouse routing with priority: LAN direct -> server relay.
- Implement Linux/Windows clipboard adapters with headless CLI support on Linux.
- Implement Docker deployment for the server with retention and capacity cleanup.
- Implement desktop UI controls for sync status, device status, clipboard policy, and input-sharing policy.

## Public Interfaces

- CLI commands: `glide copy`, `glide paste`, `glide history`, `glide devices`.
- Common CLI examples:
  - `glide copy "hello"`
  - `glide copy --file ./a.zip`
  - `glide copy --dir ./docs`
  - `glide copy --image ./pic.png`
  - `glide paste`
  - `glide paste --output ./recv`
  - `glide copy --server https://glide.example.com --token TEMP_TOKEN "hello"`
  - `glide paste --server https://glide.example.com --token TEMP_TOKEN --output ./recv`
- Server env:
  - `GLIDE_DATA_DIR`
  - `GLIDE_MAX_STORAGE_BYTES`
  - `GLIDE_RETENTION_DAYS`
  - `GLIDE_MAX_ITEM_BYTES`
  - `GLIDE_PUBLIC_URL`
  - `GLIDE_ADMIN_TOKEN`
  - `GLIDE_REGISTRATION_TOKEN`
  - `GLIDE_TEMP_TOKEN_DEFAULT_TTL`
  - `GLIDE_INPUT_RELAY_ENABLED`
  - `GLIDE_INPUT_RELAY_MAX_LATENCY_MS`
- Clipboard event fields:
  - `item_id`
  - `source_device_id`
  - `source_session_type`
  - `kind`
  - `representations`
  - `size`
  - `created_at`
  - `payload_refs`
  - `checksum`
  - `delivery_policy`
- Clipboard route priority:
  - Local loop prevention
  - LAN direct
  - LAN reverse pull
  - Server fallback
- Input route priority:
  - LAN direct
  - Server relay
  - Disconnect and release input when both fail

## Implementation Tasks

- [ ] Create workspace structure and shared crate boundaries.
- [ ] Define core types: device identity, clipboard item, MIME representation, payload reference, transfer session, sync event, input event.
- [ ] Implement server database schema for devices, clipboard items, payload objects, temporary tokens, input relay sessions, and cleanup metadata.
- [ ] Implement server APIs for device registration, token validation, WebSocket sync, HTTP payload upload/download, history query, devices query, and cleanup.
- [ ] Implement server input relay API over authenticated WebSocket for trusted persistent devices only.
- [ ] Implement temporary token support with TTL, max uses, allowed operations, and max item size.
- [ ] Implement LAN discovery with mDNS and UDP multicast for trusted persistent clients.
- [ ] Implement clipboard route selection: direct LAN, reverse LAN pull, server fallback.
- [ ] Implement input route selection: direct LAN first, server relay fallback when enabled.
- [ ] Implement input relay safeguards: heartbeat, latency measurement, rate limiting, disconnect release, emergency release.
- [ ] Implement CLI persistent mode using local config when available.
- [ ] Implement CLI single-use auth mode using `--server` and `--token` without writing credentials.
- [ ] Implement `glide copy` for text, image, file, and folder.
- [ ] Implement `glide paste` with stdout output for text and `--output` for binary/file payloads.
- [ ] Implement `glide history` and `glide devices` as read-only commands.
- [ ] Implement Linux clipboard adapter for X11/Wayland where available and headless fallback through CLI.
- [ ] Implement Windows clipboard adapter for text, rich text representation, images, and file lists.
- [ ] Implement desktop tray app with connection status, pause sync, recent history, device status, and settings.
- [ ] Implement per-device and per-type sync policies.
- [ ] Implement optional keyboard/mouse module with edge crossing, topology, hotkey switch, LAN mode, server relay mode, and emergency release.
- [ ] Add Dockerfile and `docker-compose.yml` for server deployment.
- [ ] Add package build targets for Windows `exe/msi` and Linux `deb/rpm/AppImage`.
- [ ] Add user documentation for deployment, registration, CLI temporary use, Linux headless use, security model, and input relay latency tradeoff.

## Test Plan

- [ ] Unit test serialization and deserialization for all core protocol types.
- [ ] Unit test MIME representation selection for plain text, rich text, URL, color, image, file, and folder.
- [ ] Unit test temporary token expiry, max-use counting, allowed operation checks, and item size limits.
- [ ] Unit test clipboard route selection priority and fallback behavior.
- [ ] Unit test input route selection from LAN direct to server relay.
- [ ] Integration test server startup, persistent device registration, WebSocket sync, and payload upload/download.
- [ ] Integration test temporary CLI copy/paste with no config file written.
- [ ] Integration test trusted-node LAN direct route and server fallback route.
- [ ] CLI test `glide copy`, `glide paste`, `glide history`, and `glide devices` in headless Linux.
- [ ] Desktop test Linux GNOME Wayland, GNOME X11, KDE Plasma, Windows 10/11.
- [ ] Clipboard test text, internally preserved rich text, image, file, and folder across Linux/Windows.
- [ ] Security test unregistered device rejection, revoked token rejection, path traversal rejection, checksum mismatch rejection.
- [ ] Storage test retention cleanup, capacity cleanup, object/database consistency, and preserved latest usable item.
- [ ] Input module test LAN direct, server relay fallback, edge crossing, hotkey switching, disconnect release, emergency release, and permission failure reporting.
- [ ] Input relay test high-latency warning, relay disabled rejection, rate limiting, and trusted-device-only enforcement.
- [ ] Packaging test Docker compose startup, Linux package install, AppImage run, and Windows installer run.

## Assumptions

- Glide is full-node sync only; no target-specific send command.
- Product-level clipboard types are text, image, and file/folder.
- Rich text, URL, and color are internal representations of text, not prominent user-facing categories.
- Persistent clients may use LAN discovery and direct transfer.
- Temporary CLI sessions never join the trusted device mesh and default to server-only clipboard transfer.
- Temporary CLI tokens do not allow keyboard/mouse control.
- Service-side clipboard storage is plaintext, but all client-server and client-client transport must be encrypted.
- Keyboard/mouse sync is optional, disabled by default, and supports LAN direct plus server relay fallback.
- Server-relayed keyboard/mouse sync is expected to have higher latency than LAN mode and must show that status to the user.
- Android, iOS, macOS, and Chrome are future clients, but protocol and auth are designed to allow them later.
