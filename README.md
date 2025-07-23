# Project Neon Protocol Specification

### Overview
Project Neon is a lightweight, relay-based, modular multiplayer protocol designed to support all QTI games, from physics-heavy stealth titles to inventory-based RPGs.

It is transport-agnostic (built on UDP) and uses a feature-flag-based model to enable per-game customization. It supports real-time physics syncing, state snapshots, input streaming, and optional modules like inventory, weapons, and emotes.

---

### Core Design Goals
- Low-latency, fast UDP communication
- Modular packet types, only enabled if a feature is in use
- Peer-host authority model, optionally migratable
- Relay architecture for NAT traversal and consistent connectivity
- Engine-agnostic: designed for Unreal clients, but adaptable
- Extensible protocol: future-proof without breaking older clients
