# Project Neon Protocol Specification

Version: 0.2
Author: Kohan Mathers

---

## Overview

Project Neon is a **fully modular**, relay-based multiplayer protocol that is completely game-agnostic. Unlike traditional protocols with hardcoded game features, Neon provides only the bare essentials for connection management, leaving all game-specific logic to be defined by the application layer.

This allows true crossovers by letting each game define its own packet vocabulary.

---

## Core Design Philosophy

- **Minimal Core**: Only connection/session management is hardcoded
- **Zero Assumptions**: No built-in movement, inventory, combat, or any game mechanics
- **Dynamic Packet Registry**: Games register their own packet types at runtime
- **Universal Relay**: Relay forwards anything without understanding it
- **Complete Modularity**: From simple chat apps to complex MMOs using the same base protocol

---

## Packet Layout

All Neon packets follow this structure:

```rust
struct NeonPacket {
    header: PacketHeader,
    payload: Vec<u8>,  // Raw bytes - game interprets
}
```

### PacketHeader

```rust
struct PacketHeader {
    magic: u16,          // 0x4E45 = "NE"
    version: u8,         // Protocol version (core only)
    packet_type: u8,     // See packet types below
    sequence: u16,       // For ordering/reliability
    client_id: u8,       // Sender
    destination_id: u8,  // Target (0 = broadcast, 1 = host, 2+ = clients)
}
```

---

## Core Packet Types

**Only these packet types are part of the core protocol:**

```rust
enum CorePacketType {
    // Connection Management (0x01-0x0F reserved)
    0x01 = ConnectRequest,
    0x02 = ConnectAccept,
    0x03 = ConnectDeny,
    0x04 = SessionConfig,
    0x05 = PacketTypeRegistry,
    0x0B = Ping,
    0x0C = Pong,
    0x0D = DisconnectNotice,
    
    // Game-Defined Range (0x10-0xFF)
    0x10+ = GamePacket,  // Everything else is application-defined
}
```

---

## Core Packet Payloads

### ConnectRequest

```rust
struct ConnectRequest {
    client_version: u8,      // Client's protocol version
    desired_name: String,    // Display name
    target_session_id: u32,  // Which session to join
    game_identifier: u32,    // Game hash/ID (optional validation)
}
```

### ConnectAccept

```rust
struct ConnectAccept {
    assigned_client_id: u8,
    session_id: u32,
}
```

### ConnectDeny

```rust
struct ConnectDeny {
    reason: String,
}
```

### SessionConfig

```rust
struct SessionConfig {
    version: u8,              // Session protocol version
    tick_rate: u16,           // Server tick rate (informational)
    max_packet_size: u16,     // MTU hint
}
```

### PacketTypeRegistry

Allows host to share packet type definitions with clients (optional, for debugging/tooling):

```rust
struct PacketTypeRegistry {
    entries: Vec<PacketTypeEntry>,
}

struct PacketTypeEntry {
    packet_id: u8,           // e.g., 0x10
    name: String,            // e.g., "PlayerMovement"
    description: String,     // Optional schema info
}
```

### Ping/Pong

```rust
struct Ping {
    timestamp: u64,
}

struct Pong {
    original_timestamp: u64,
}
```

---

## Game-Defined Packets (0x10+)

**Everything from 0x10 onwards is application-defined.** The protocol doesn't care what you send.

---

## Game Packet Structure

Games are free to structure their payloads however they want:

```rust
// Example: A movement packet
struct GameMovementPacket {
    actor_id: u32,
    position: [f32; 3],
    rotation: [f32; 4],
    velocity: [f32; 3],
    // ... whatever the game needs
}

// Sent as:
NeonPacket {
    header: PacketHeader {
        packet_type: 0x10,  // Registered as "Movement"
        ...
    },
    payload: serialize(GameMovementPacket { ... }),
}
```

---

## Relay Behavior

The relay is **completely payload-agnostic**:

1. Receives packet
2. Validates header (magic, version)
3. Routes based on `destination_id`
4. Forwards raw bytes without parsing payload

**The relay never needs to understand game packets.**

---

## Session Discovery & Matching

Since there are no feature flags, games identify compatibility through:

1. **Game Identifier**: Hash or ID in ConnectRequest
2. **Version Checking**: Host can reject incompatible clients
3. **PacketTypeRegistry**: Optional negotiation of supported packets
4. **Out-of-band Matching**: External matchmaking services

---

## Benefits of This Approach

### Complete Freedom
- FPS, RPG, puzzle, chat app - all use same protocol
- No protocol updates needed for new game types

### True Modularity
- Replace/extend any packet type without core changes
- Multiple games can coexist on same relay

### Crossover Support
- Shared packet types for common features
- Game-specific packets ignored by others
- Universal translator pattern possible

### Simplicity
- Core protocol is tiny (~8 packet types)
- Games handle their own complexity
- Relay is dumb and fast

---

## Implementation Strategy

### For Game Developers

1. Define your packet types (0x10+)
2. Implement serialization for your packets
3. Send/receive through Neon core
4. Optionally share PacketTypeRegistry for debugging

### Example Code Structure

```rust
// Game-specific packet handler
match packet.header.packet_type {
    0x01..=0x0F => core_handler.handle(packet),
    0x10..=0xFF => game_handler.handle(packet),
}
```

---

## Usage

### Building from Source

```bash
# Clone the repository
git clone https://github.com/KohanMathers/ProjectNeon
cd ProjectNeon

# Build release binaries
cargo build --release

# Binaries will be in target/release/
# - relay (standalone relay server)
# - client (example client)
# - host (example host)
# - libproject_neon.so (C FFI library)
```

### Using Precompiled Binaries

#### Running the Relay Server

```bash
# Start relay on default port (7777)
./relay

# Or specify a custom address
./relay --bind 0.0.0.0:8888
```

#### C/C++ Integration

For integrating with C/C++ applications (Unreal Engine, Unity, custom engines):

**Required Files:**
- `libproject_neon.so` (Linux) / `project_neon.dll` (Windows) / `libproject_neon.dylib` (macOS)
- `project_neon.h`

**Basic Usage:**

```c
#include "project_neon.h"

// Create and connect a client
NeonClientHandle* client = neon_client_new("PlayerName");
if (neon_client_connect(client, 12345, "127.0.0.1:7777")) {
    printf("Connected! Client ID: %u\n", neon_client_get_id(client));
}

// In your game loop
while (game_running) {
    neon_client_process_packets(client);
    // Your game logic here
}

// Cleanup
neon_client_free(client);
```

**Host Example:**

```c
// Create host (blocking call - run in thread!)
NeonHostHandle* host = neon_host_new(12345, "127.0.0.1:7777");

// Start host in separate thread
pthread_create(&thread, NULL, host_thread, host);

// Check connected clients
size_t count = neon_host_get_client_count(host);
```

#### Linking in Your Build System

**CMake:**
```cmake
target_include_directories(YourProject PRIVATE ${NEON_INCLUDE_DIR})
target_link_libraries(YourProject ${NEON_LIB_DIR}/libproject_neon.so)
```

**Unreal Engine (.Build.cs):**
```csharp
PublicIncludePaths.Add(Path.Combine(ModuleDirectory, "ThirdParty/Neon/include"));
PublicAdditionalLibraries.Add(Path.Combine(ModuleDirectory, "ThirdParty/Neon/lib/project_neon.dll"));
```

**Manual GCC:**
```bash
gcc -o mygame main.c -I./include -L./lib -lproject_neon -lpthread -Wl,-rpath,./lib
```

### Rust Integration

```toml
[dependencies]
project_neon = { path = "../projectneon" }
```

```rust
use project_neon::{NeonClient, NeonHost, NeonRelay};

let mut client = NeonClient::new("PlayerName".to_string())?;
client.connect(12345, "127.0.0.1:7777")?;

loop {
    client.process_packets()?;
    // Your game logic
}
```

### Testing Your Setup

```bash
# Terminal 1: Start relay
./relay

# Terminal 2: Run test client
./test_neon
```

The test program will create a host and two clients, demonstrating the full connection flow.

---

## Future Possibilities

- **Universal Game Protocol Library**: Common packet types (movement, chat, etc.)
- **Cross-Game Standards**: Agreed-upon packet IDs for interoperability
- **Protocol Bridges**: Translate between different games' packet formats
- **Visual Packet Inspector**: Debug tool that uses PacketTypeRegistry