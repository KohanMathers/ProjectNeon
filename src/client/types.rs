use std::io::{Error, ErrorKind};
use std::convert::TryInto;

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
    pub game_identifier: u32,
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

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum PacketType {
    ConnectRequest = 0x01,
    ConnectAccept = 0x02,
    ConnectDeny = 0x03,
    SessionConfig = 0x04,
    PacketTypeRegistry = 0x05,
    Ping = 0x0B,
    Pong = 0x0C,
    DisconnectNotice = 0x0D,
    GamePacket = 0x10,
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
                bytes.extend(&req.game_identifier.to_le_bytes());
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
                bytes.push(registry.entries.len() as u8);
                for entry in &registry.entries {
                    bytes.push(entry.packet_id);
                    let name_bytes = entry.name.as_bytes();
                    let desc_bytes = entry.description.as_bytes();
                    bytes.push(name_bytes.len() as u8);
                    bytes.extend(name_bytes);
                    bytes.push(desc_bytes.len() as u8);
                    bytes.extend(desc_bytes);
                }
                bytes
            }
            PacketPayload::GamePacket(data) => data.clone(),
        }
    }

    pub fn from_bytes(packet_type: u8, data: &[u8]) -> Result<Self, Error> {
        match packet_type {
            x if x == PacketType::Ping as u8 => {
                if data.len() < 8 {
                    return Err(Error::new(ErrorKind::InvalidData, "Ping too short"));
                }
                Ok(PacketPayload::Ping(Ping {
                    timestamp: u64::from_le_bytes(data[0..8].try_into().unwrap()),
                }))
            }
            x if x == PacketType::Pong as u8 => {
                if data.len() < 8 {
                    return Err(Error::new(ErrorKind::InvalidData, "Pong too short"));
                }
                Ok(PacketPayload::Pong(Pong {
                    original_timestamp: u64::from_le_bytes(data[0..8].try_into().unwrap()),
                }))
            }
            x if x == PacketType::ConnectRequest as u8 => {
                if data.len() < 9 {
                    return Err(Error::new(ErrorKind::InvalidData, "ConnectRequest too short"));
                }
                let client_version = data[0];
                let target_session_id = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                let game_identifier = u32::from_le_bytes([data[5], data[6], data[7], data[8]]);
                let desired_name = String::from_utf8_lossy(&data[9..]).to_string();
                Ok(PacketPayload::ConnectRequest(ConnectRequest {
                    client_version,
                    desired_name,
                    target_session_id,
                    game_identifier,
                }))
            }
            x if x == PacketType::ConnectAccept as u8 => {
                if data.len() < 5 {
                    return Err(Error::new(ErrorKind::InvalidData, "ConnectAccept too short"));
                }
                let client_id = data[0];
                let session_id = u32::from_le_bytes(data[1..5].try_into().unwrap());
                Ok(PacketPayload::ConnectAccept(ConnectAccept {
                    assigned_client_id: client_id,
                    session_id,
                }))
            }
            x if x == PacketType::ConnectDeny as u8 => {
                let reason = String::from_utf8_lossy(data).to_string();
                Ok(PacketPayload::ConnectDeny(ConnectDeny { reason }))
            }
            x if x == PacketType::SessionConfig as u8 => {
                if data.len() < 5 {
                    return Err(Error::new(ErrorKind::InvalidData, "SessionConfig too short"));
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
            x if x == PacketType::PacketTypeRegistry as u8 => {
                if data.is_empty() {
                    return Ok(PacketPayload::PacketTypeRegistry(PacketTypeRegistry { entries: vec![] }));
                }
                let mut idx = 0;
                let mut entries = Vec::new();
                let count = data[idx] as usize;
                idx += 1;
                for _ in 0..count {
                    if idx >= data.len() { break; }
                    let packet_id = data[idx]; idx += 1;
                    let name_len = data[idx] as usize; idx += 1;
                    let name = String::from_utf8_lossy(&data[idx..idx+name_len]).to_string(); idx += name_len;
                    let desc_len = data[idx] as usize; idx += 1;
                    let description = String::from_utf8_lossy(&data[idx..idx+desc_len]).to_string(); idx += desc_len;
                    entries.push(PacketTypeEntry { packet_id, name, description });
                }
                Ok(PacketPayload::PacketTypeRegistry(PacketTypeRegistry { entries }))
            }
            x if x >= 0x10 => {
                Ok(PacketPayload::GamePacket(data.to_vec()))
            }
            _ => Ok(PacketPayload::None),
        }
    }
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

        Ok(PacketHeader {
            magic,
            version: data[2],
            packet_type: data[3],
            sequence: u16::from_le_bytes([data[4], data[5]]),
            client_id: data[6],
            destination_id: data[7],
        })
    }
}