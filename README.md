# Project Neon Protocol Specification

Version: 0.1  
Author: Kohan Mathers

---

## Overview

Project Neon is a lightweight, relay-based, modular multiplayer protocol designed to support all QTI games, from physics-heavy stealth titles to inventory-based RPGs.

It is transport-agnostic (built on UDP) and uses a feature-flag-based model to enable per-game customization. It supports real-time physics syncing, state snapshots, input streaming, and optional modules like inventory, weapons, and emotes.

---

## Core Design Goals

- Low-latency, fast UDP communication
- Modular packet types, only enabled if a feature is in use
- Peer-host authority model, optionally migratable
- Relay architecture for NAT traversal and consistent connectivity
- Engine-agnostic: designed for Unreal clients, but adaptable
- Extensible protocol: future-proof without breaking older clients

---

## Packet Layout

All Neon packets follow this general structure:

```rust
struct NeonPacket {
    header: PacketHeader,
    payload: Payload,
}`
```

### PacketHeader

```rust
struct PacketHeader {
    magic: u16,        // 0x4E45 = "NE"
    version: u8,       // Protocol version
    packet_type: u8,   // Enum
    sequence: u16,     // For ordering
    client_id: u8,     // Sender
}
```

---

## Packet Types

```rust
enum PacketType {
    0x01 = ConnectRequest,
    0x02 = ConnectAccept,
    0x03 = ConnectDeny,
    0x04 = SessionConfig,
    0x05 = InputState,
    0x06 = WorldSnapshot,
    0x07 = RagdollState,
    0x08 = InventoryState,
    0x09 = WeaponState,
    0x0A = EmoteAction,
    0x0B = Ping,
    0x0C = Pong,
    0x0D = DisconnectNotice,
}
```

---

## Feature Negotiation

### FeatureSet Flags

Each session begins with a `SessionConfig` packet sent by the host/authoritative peer. It includes:

```rust
bitflags! {
    struct FeatureSet: u64 {
        const MOVEMENT     = 0b00000001;
        const RAGDOLL      = 0b00000010;
        const INVENTORY    = 0b00000100;
        const WEAPONS      = 0b00001000;
        const EMOTES       = 0b00010000;
        const ABILITIES    = 0b00100000;
        const CUSTOM_UI    = 0b01000000;
        const VOIP         = 0b10000000;
    }
}
```

### SessionConfig Payload

```rust
struct SessionConfig {
    version: u8,
    tick_rate: u16,
    feature_flags: FeatureSet,
}
```

Clients use this to determine what modules to enable/disable.

---

## Core Packet Payloads

### ConnectRequest

Sent from client to host/relay to initiate a session.

```rust
struct ConnectRequest {
    client_version: u8,
    desired_name: String,
    preferred_features: FeatureSet,
}
```

---

### InputState

Only present if MOVEMENT is enabled.

```rust
struct InputState {
    actor_id: u32,
    tick: u32,
    movement: [f32; 2], // x/y
    aim: f32,
    jump: bool,
    fire: bool,
}
```

---

### WorldSnapshot

Sent by the authoritative peer to all clients.

```rust
struct WorldSnapshot {
    tick: u32,
    entities: Vec<EntityState>,
}
```

---

### RagdollState

Only sent if RAGDOLL is enabled.

```rust
struct RagdollState {
    actor_id: u32,
    bone_count: u8,
    bones: [BoneState; MAX_BONES],
}
```

```rust
struct BoneState {
    position: [f32; 3],
    rotation: [f32; 4], // quaternion
    velocity: [f32; 3],
}
```

---

### InventoryState

Only sent if INVENTORY is enabled.

```rust
struct InventoryState {
    actor_id: u32,
    slots: Vec<InventorySlot>,
}
```

```rust
struct InventorySlot {
    slot_id: u8,
    item_id: u16,
    quantity: u8,
}
```

---

### WeaponState, EmoteAction, etc.

(To be added as needed.)

---

## Testing & Debug Mode

- All packets can include an optional debug trailer with CRC or signature
- Consider reserving a DEBUG_PACKET type to allow verbose logging in test builds

---

## Security (Planned)

- Future versions may include:
  - HMAC headers
  - Optional encryption with symmetric keys
  - Replay protection
  - Rate-limiting on the relay

---

## Relay Behavior

- Stateless, forward-only
- No need to parse payloads
- Optionally validate headers and track ping
- Can act as basic NAT punchthrough helper if needed

---

## Future Extensions

- Host migration support
- Matchmaking layer
- Reliable UDP module (NeonReliable)
- Compression (e.g., quantized transforms)
- In-editor Neon simulator for QTI games