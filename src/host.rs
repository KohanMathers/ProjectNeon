use bitflags::bitflags;
use std::collections::HashMap;
use std::convert::TryInto;
use std::io::{Error, ErrorKind};
use std::net::{SocketAddr, UdpSocket};
use std::thread::sleep;
use std::time::Duration;
use std::env;

#[derive(Debug, Clone)]
struct PacketHeader {
    magic: u16,
    version: u8,
    packet_type: u8,
    sequence: u16,
    client_id: u8,
    destination_id: u8,
}

#[derive(Debug, Clone)]
enum PacketPayload {
    None,
    Ping(Ping),
    Pong(Pong),
    ConnectRequest(ConnectRequest),
    ConnectAccept(ConnectAccept),
    ConnectDeny(ConnectDeny),
    SessionConfig(SessionConfig),
    PacketTypeRegistry(PacketTypeRegistry),
    GamePacket(Vec<u8>), // 0x10+
}

#[derive(Debug, Clone)]
struct NeonPacket {
    packet_type: u8,
    sequence: u16,
    client_id: u8,
    destination_id: u8,
    payload: PacketPayload,
}

#[derive(Debug, Clone)]
struct ConnectRequest {
    client_version: u8,
    desired_name: String,
    target_session_id: u32,
    game_identifier: u32,
}

#[derive(Debug, Clone)]
struct ConnectAccept {
    assigned_client_id: u8,
    session_id: u32,
}

#[derive(Debug, Clone)]
struct ConnectDeny {
    reason: String,
}

#[derive(Debug, Clone, Copy)]
struct Ping {
    timestamp: u64,
}

#[derive(Debug, Clone)]
struct Pong {
    original_timestamp: u64,
}



#[derive(Debug, Clone)]
struct SessionConfig {
    version: u8,
    tick_rate: u16,
    max_packet_size: u16,
}

#[derive(Debug, Clone)]
struct PacketTypeRegistry {
    entries: Vec<PacketTypeEntry>,
}

#[derive(Debug, Clone)]
struct PacketTypeEntry {
    packet_id: u8,
    name: String,
    description: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum PacketType {
    ConnectRequest = 0x01,
    ConnectAccept = 0x02,
    ConnectDeny = 0x03,
    SessionConfig = 0x04,
    PacketTypeRegistry = 0x05,
    Ping = 0x0B,
    Pong = 0x0C,
    DisconnectNotice = 0x0D,
    GamePacket = 0x10, // 0x10+ application-defined
}

impl PacketPayload {
    fn to_bytes(&self) -> Vec<u8> {
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
            PacketPayload::ConnectDeny(deny) => {
                deny.reason.as_bytes().to_vec()
            }
            PacketPayload::SessionConfig(config) => {
                let mut bytes = vec![config.version];
                bytes.extend(&config.tick_rate.to_le_bytes());
                bytes.extend(&config.max_packet_size.to_le_bytes());
                bytes
            }
        }
    }

    fn from_bytes(packet_type: u8, data: &[u8]) -> Result<Self, Error> {
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
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "ConnectRequest too short",
                    ));
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
            x if x == PacketType::ConnectDeny as u8 => {
                let reason = String::from_utf8_lossy(data).to_string();
                Ok(PacketPayload::ConnectDeny(ConnectDeny { reason }))
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
            x if x == PacketType::SessionConfig as u8 => {
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
            _ => Ok(PacketPayload::None),
        }
    }
}

impl PacketHeader {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(&self.magic.to_le_bytes());
        bytes.push(self.version);
        bytes.push(self.packet_type);
        bytes.extend(&self.sequence.to_le_bytes());
        bytes.push(self.client_id);
        bytes.push(self.destination_id);
        bytes
    }

    fn from_bytes(data: &[u8]) -> Result<PacketHeader, Error> {
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

struct NeonSocket {
    socket: UdpSocket,
}

impl NeonSocket {
    fn new(bind_addr: &str) -> Result<Self, Error> {
        let socket = UdpSocket::bind(bind_addr)?;
        socket.set_nonblocking(true)?;
        Ok(Self { socket })
    }

    fn send_packet(&self, packet: &NeonPacket, addr: SocketAddr) -> Result<(), Error> {
        let header = PacketHeader {
            magic: 0x4E45,
            version: 1,
            packet_type: packet.packet_type,
            sequence: packet.sequence,
            client_id: packet.client_id,
            destination_id: packet.destination_id,
        };
        let mut bytes = header.to_bytes();
        bytes.extend(packet.payload.to_bytes());
        self.socket.send_to(&bytes, addr)?;
        Ok(())
    }

    fn receive_packet(&self) -> Result<(NeonPacket, SocketAddr), Error> {
        let mut buf = [0; 1024];
        let (size, addr) = self.socket.recv_from(&mut buf)?;
        let header = PacketHeader::from_bytes(&buf[..8])?;
        let payload = PacketPayload::from_bytes(header.packet_type, &buf[8..size])?;
        Ok((
            NeonPacket {
                packet_type: header.packet_type,
                sequence: header.sequence,
                client_id: header.client_id,
                destination_id: header.destination_id,
                payload,
            },
            addr,
        ))
    }
}

pub struct HostSession {
    socket: NeonSocket,
    relay_addr: SocketAddr,
    client_id: u8,
    session_id: u32,
    connected_clients: HashMap<u8, String>,
    next_client_id: u8,
}

impl HostSession {
    pub fn new(relay_addr: SocketAddr, session_id: u32) -> Result<Self, Error> {
        Ok(Self {
            socket: NeonSocket::new("0.0.0.0:0")?,
            relay_addr,
            client_id: 1,
            session_id,
            connected_clients: HashMap::new(),
            next_client_id: 2,
        })
    }

    pub fn start(&mut self) -> Result<(), Error> {
        println!(
            "[Host] Starting session {} via relay {}",
            self.session_id, self.relay_addr
        );

        let host_register_packet = NeonPacket {
            packet_type: PacketType::ConnectAccept as u8,
            sequence: 0,
            client_id: self.client_id,
            destination_id: 1,
            payload: PacketPayload::ConnectAccept(ConnectAccept {
                assigned_client_id: self.client_id,
                session_id: self.session_id,
            }),
        };

        self.socket
            .send_packet(&host_register_packet, self.relay_addr)?;

        loop {
            match self.socket.receive_packet() {
                Ok((packet, addr)) => match packet.payload {
                    PacketPayload::ConnectRequest(req) => {
                        self.handle_connect_request(req, addr)?;
                    }
                    PacketPayload::Ping(ping) => {
                        let pong_packet = NeonPacket {
                            packet_type: PacketType::Pong as u8,
                            sequence: packet.sequence,
                            client_id: self.client_id,
                            destination_id: packet.client_id,
                            payload: PacketPayload::Pong(Pong {
                                original_timestamp: ping.timestamp,
                            }),
                        };
                        self.socket.send_packet(&pong_packet, self.relay_addr)?;
                        println!("[Host] Responded to ping from client {}", packet.client_id);
                    }
                    _ => {
                        println!("[Host] Unhandled packet from {}: {:?}", addr, packet);
                    }
                },
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    sleep(Duration::from_millis(10));
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn is_name_taken(&self, name: &str) -> bool {
        self.connected_clients.values().any(|n| n == name)
    }

    fn handle_connect_request(
        &mut self,
        req: ConnectRequest,
        _client_addr: SocketAddr,
    ) -> Result<(), Error> {
        if req.target_session_id != self.session_id {
            println!(
                "[Host] Client '{}' requested wrong session {} (we are {}), ignoring",
                req.desired_name, req.target_session_id, self.session_id
            );
            return Ok(());
        }

        if self.is_name_taken(&req.desired_name) {
            println!(
                "[Host] Client '{}' rejected - name already in use",
                req.desired_name
            );

            let deny_packet = NeonPacket {
                packet_type: PacketType::ConnectDeny as u8,
                sequence: 1,
                client_id: 1,
                destination_id: 0,
                payload: PacketPayload::ConnectDeny(ConnectDeny {
                    reason: format!("Name '{}' is already in use", req.desired_name),
                }),
            };

            self.socket.send_packet(&deny_packet, self.relay_addr)?;
            println!("[Host] Sent ConnectDeny to relay for duplicate name");
            return Ok(());
        }

        let assigned_id = self.next_client_id;
        self.next_client_id += 1;

        println!(
            "[Host] New client '{}' requesting to join session {} -> assigned ID {}",
            req.desired_name, req.target_session_id, assigned_id
        );

        let accept = ConnectAccept {
            assigned_client_id: assigned_id,
            session_id: self.session_id,
        };

        let accept_packet = NeonPacket {
            packet_type: PacketType::ConnectAccept as u8,
            sequence: 1,
            client_id: assigned_id,
            destination_id: assigned_id,
            payload: PacketPayload::ConnectAccept(accept.clone()),
        };

        self.socket.send_packet(&accept_packet, self.relay_addr)?;
        println!(
            "[Host] Sent ConnectAccept to relay for client {}",
            assigned_id
        );


        let config = SessionConfig {
            version: 1,
            tick_rate: 60,
            max_packet_size: 1024,
        };

        let config_packet = NeonPacket {
            packet_type: PacketType::SessionConfig as u8,
            sequence: 2,
            client_id: assigned_id,
            destination_id: assigned_id,
            payload: PacketPayload::SessionConfig(config),
        };

        self.socket.send_packet(&config_packet, self.relay_addr)?;
        println!(
            "[Host] Sent SessionConfig to relay for client {}",
            assigned_id
        );

        // Optionally send PacketTypeRegistry (for debugging/tooling)
        let registry = PacketTypeRegistry {
            entries: vec![
                PacketTypeEntry {
                    packet_id: 0x10,
                    name: "GamePacket".to_string(),
                    description: "Application-defined packet".to_string(),
                },
            ],
        };
        let registry_packet = NeonPacket {
            packet_type: PacketType::PacketTypeRegistry as u8,
            sequence: 3,
            client_id: assigned_id,
            destination_id: assigned_id,
            payload: PacketPayload::PacketTypeRegistry(registry),
        };
        self.socket.send_packet(&registry_packet, self.relay_addr)?;
        println!("[Host] Sent PacketTypeRegistry to relay for client {}", assigned_id);

        // Add client to connected_clients map
        self.connected_clients.insert(assigned_id, req.desired_name.clone());
        println!("[Host] Client {} ('{}') accepted and configured", assigned_id, req.desired_name);
        
        Ok(())
    }
}

fn main() {
    println!("Project Neon Alpha Build 12 - Host");
    let relay_addr = "127.0.0.1:7777".parse().unwrap();

    let args: Vec<String> = env::args().collect();
    let session_id = if args.len() > 1 {
        match args[1].parse::<u32>() {
            Ok(id) => id,
            Err(_) => {
                println!("Invalid session ID argument, using random session ID.");
                rand::random::<u32>()
            }
        }
    } else {
        rand::random::<u32>()
    };

    let mut host = HostSession::new(relay_addr, session_id).unwrap();

    println!("Host will create session ID: {}", host.session_id);
    println!();

    host.start().expect("Host failed");
}