use std::convert::TryInto;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct PacketHeader {
    pub magic: u16,
    pub version: u8,
    pub packet_type: u8,
    pub sequence: u16,
    pub client_id: u8,
    pub destination_id: u8,
}

#[derive(Debug, Clone)]
pub enum PacketPayload {
    None,
    Ping(Ping),
    Pong(Pong),
    ConnectRequest(ConnectRequest),
    ConnectAccept(ConnectAccept),
    ConnectDeny(ConnectDeny),
    SessionConfig(SessionConfig),
    Ack(Ack),
    PacketTypeRegistry(PacketTypeRegistry),
    GamePacket(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct NeonPacket {
    pub packet_type: u8,
    pub sequence: u16,
    pub client_id: u8,
    pub destination_id: u8,
    pub payload: PacketPayload,
}

#[derive(Debug, Clone)]
pub struct ConnectRequest {
    pub client_version: u8,
    pub desired_name: String,
    pub target_session_id: u32,
    pub game_identifier: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ConnectAccept {
    pub assigned_client_id: u8,
    pub session_id: u32,
}

#[derive(Debug, Clone)]
pub struct ConnectDeny {
    pub reason: String,
}

#[derive(Debug, Clone, Copy)]
pub struct Ping {
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct Pong {
    pub original_timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub version: u8,
    pub tick_rate: u16,
    pub max_packet_size: u16,
}

#[derive(Debug, Clone)]
pub struct Ack {
    pub acknowledged_sequences: Vec<u16>,
}

#[derive(Debug, Clone)]
pub struct PacketTypeRegistry {
    pub entries: Vec<PacketTypeEntry>,
}

#[derive(Debug, Clone)]
pub struct PacketTypeEntry {
    pub packet_id: u8,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum CorePacketType {
    ConnectRequest = 0x01,
    ConnectAccept = 0x02,
    ConnectDeny = 0x03,
    SessionConfig = 0x04,
    PacketTypeRegistry = 0x05,
    Ping = 0x0B,
    Pong = 0x0C,
    DisconnectNotice = 0x0D,
    Ack = 0x0E,
}

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub addr: SocketAddr,
    pub client_id: u8,
    pub session_id: u32,
    pub last_seen: Instant,
    pub is_host: bool,
}

#[derive(Debug, Clone)]
pub struct PendingConnection {
    pub client_addr: SocketAddr,
    pub session_id: u32,
    pub client_name: String,
}

impl PacketHeader {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(&self.magic.to_le_bytes());
        bytes.push(self.version);
        bytes.push(self.packet_type);
        bytes.extend(&self.sequence.to_le_bytes());
        bytes.push(self.client_id);
        bytes.push(self.destination_id);
        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Result<PacketHeader, Error> {
        if data.len() < 8 {
            return Err(Error::new(ErrorKind::InvalidData, "Data too short"));
        }

        let magic = u16::from_le_bytes([data[0], data[1]]);
        if magic != 0x4E45 {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid magic number"));
        }

        let version = data[2];
        let packet_type = data[3];
        let sequence = u16::from_le_bytes([data[4], data[5]]);
        let client_id = data[6];
        let destination_id = data[7];

        Ok(PacketHeader {
            magic,
            version,
            packet_type,
            sequence,
            client_id,
            destination_id,
        })
    }
}

impl PacketPayload {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            PacketPayload::None => vec![],
            PacketPayload::Ping(ping) => ping.timestamp.to_le_bytes().to_vec(),
            PacketPayload::Pong(pong) => pong.original_timestamp.to_le_bytes().to_vec(),
            PacketPayload::ConnectRequest(req) => {
                let mut bytes = vec![req.client_version];
                bytes.extend(&req.target_session_id.to_le_bytes());
                
                if let Some(game_id) = req.game_identifier {
                    bytes.push(1);
                    bytes.extend(&game_id.to_le_bytes());
                } else {
                    bytes.push(0);
                }
                
                bytes.extend(req.desired_name.as_bytes());
                bytes
            }
            PacketPayload::ConnectAccept(accept) => {
                let mut bytes = vec![accept.assigned_client_id];
                bytes.extend(&accept.session_id.to_le_bytes());
                bytes
            }
            PacketPayload::ConnectDeny(deny) => {
                deny.reason.as_bytes().to_vec()
            }
            PacketPayload::SessionConfig(config) => {
                let mut bytes = vec![config.version];
                bytes.extend(&config.tick_rate.to_le_bytes());
                bytes.extend(&config.max_packet_size.to_le_bytes());
                bytes
            }
            PacketPayload::PacketTypeRegistry(registry) => {
                let mut bytes = Vec::new();
                // Use a single byte for entry count to match client/host parsing
                bytes.push(registry.entries.len() as u8);

                for entry in &registry.entries {
                    bytes.push(entry.packet_id);
                    let name_bytes = entry.name.as_bytes();
                    bytes.push(name_bytes.len() as u8);
                    bytes.extend(name_bytes);

                    let desc_bytes = entry.description.as_bytes();
                    // Use a single byte for description length to match client/host parsing
                    bytes.push(desc_bytes.len() as u8);
                    bytes.extend(desc_bytes);
                }

                bytes
            }
            PacketPayload::Ack(ack) => {
                let mut bytes = Vec::new();
                bytes.push(ack.acknowledged_sequences.len() as u8);
                for seq in &ack.acknowledged_sequences {
                    bytes.extend(&seq.to_le_bytes());
                }
                bytes
            }
            PacketPayload::GamePacket(data) => data.clone(),
        }
    }

    pub fn from_bytes(packet_type: u8, data: &[u8]) -> Result<Self, Error> {
        match packet_type {
            x if x == CorePacketType::Ping as u8 => {
                if data.len() < 8 {
                    return Err(Error::new(ErrorKind::InvalidData, "Ping too short"));
                }
                let timestamp = u64::from_le_bytes(data[0..8].try_into().unwrap());
                Ok(PacketPayload::Ping(Ping { timestamp }))
            }
            x if x == CorePacketType::Pong as u8 => {
                if data.len() < 8 {
                    return Err(Error::new(ErrorKind::InvalidData, "Pong too short"));
                }
                let timestamp = u64::from_le_bytes(data[0..8].try_into().unwrap());
                Ok(PacketPayload::Pong(Pong {
                    original_timestamp: timestamp,
                }))
            }
            x if x == CorePacketType::ConnectRequest as u8 => {
                if data.len() < 6 {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "ConnectRequest too short",
                    ));
                }
                let client_version = data[0];
                let target_session_id = u32::from_le_bytes(data[1..5].try_into().unwrap());
                
                let has_game_id = data[5];
                let (game_identifier, name_start) = if has_game_id == 1 {
                    if data.len() < 10 {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            "ConnectRequest with game_id too short",
                        ));
                    }
                    (Some(u32::from_le_bytes(data[6..10].try_into().unwrap())), 10)
                } else {
                    (None, 6)
                };
                
                let desired_name = String::from_utf8_lossy(&data[name_start..]).to_string();
                Ok(PacketPayload::ConnectRequest(ConnectRequest {
                    client_version,
                    desired_name,
                    target_session_id,
                    game_identifier,
                }))
            }
            x if x == CorePacketType::ConnectAccept as u8 => {
                if data.len() < 5 {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "ConnectAccept too short",
                    ));
                }
                let client_id = data[0];
                let session_id = u32::from_le_bytes(data[1..5].try_into().unwrap());
                Ok(PacketPayload::ConnectAccept(ConnectAccept {
                    assigned_client_id: client_id,
                    session_id,
                }))
            }
            x if x == CorePacketType::ConnectDeny as u8 => {
                let reason = String::from_utf8_lossy(data).to_string();
                Ok(PacketPayload::ConnectDeny(ConnectDeny { reason }))
            }
            x if x == CorePacketType::SessionConfig as u8 => {
                if data.len() < 5 {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "SessionConfig too short",
                    ));
                }
                let version = data[0];
                let tick_rate = u16::from_le_bytes([data[1], data[2]]);
                let max_packet_size = u16::from_le_bytes([data[3], data[4]]);
                Ok(PacketPayload::SessionConfig(SessionConfig {
                    version,
                    tick_rate,
                    max_packet_size,
                }))
            }
            x if x == CorePacketType::PacketTypeRegistry as u8 => {
                if data.is_empty() {
                    return Ok(PacketPayload::PacketTypeRegistry(PacketTypeRegistry { entries: vec![] }));
                }
                let mut offset = 0;
                if offset >= data.len() { return Err(Error::new(ErrorKind::InvalidData, "PacketTypeRegistry malformed")); }
                let entry_count = data[offset] as usize;
                offset += 1;
                let mut entries = Vec::new();
                for _ in 0..entry_count {
                    if offset + 2 > data.len() { return Err(Error::new(ErrorKind::InvalidData, "PacketTypeRegistry malformed")); }
                    let packet_id = data[offset]; offset += 1;
                    let name_len = data[offset] as usize; offset += 1;
                    if offset + name_len > data.len() { return Err(Error::new(ErrorKind::InvalidData, "PacketTypeRegistry malformed")); }
                    let name = String::from_utf8_lossy(&data[offset..offset + name_len]).to_string(); offset += name_len;
                    if offset >= data.len() { return Err(Error::new(ErrorKind::InvalidData, "PacketTypeRegistry malformed")); }
                    let desc_len = data[offset] as usize; offset += 1;
                    if offset + desc_len > data.len() { return Err(Error::new(ErrorKind::InvalidData, "PacketTypeRegistry malformed")); }
                    let description = String::from_utf8_lossy(&data[offset..offset + desc_len]).to_string(); offset += desc_len;
                    entries.push(PacketTypeEntry { packet_id, name, description });
                }
                Ok(PacketPayload::PacketTypeRegistry(PacketTypeRegistry { entries }))
            }
            x if x == CorePacketType::Ack as u8 => {
                if data.is_empty() {
                    return Ok(PacketPayload::Ack(Ack { acknowledged_sequences: vec![] }));
                }
                let count = data[0] as usize;
                let mut sequences = Vec::new();
                let mut offset = 1;
                for _ in 0..count {
                    if offset + 2 <= data.len() {
                        sequences.push(u16::from_le_bytes([data[offset], data[offset + 1]]));
                        offset += 2;
                    }
                }
                Ok(PacketPayload::Ack(Ack { acknowledged_sequences: sequences }))
            }
            x if x >= 0x10 => Ok(PacketPayload::GamePacket(data.to_vec())),
            _ => Ok(PacketPayload::None),
        }
    }
}