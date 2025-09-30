use std::net::{UdpSocket, SocketAddr};
use std::io::{Error, ErrorKind, stdin, stdout, Write};
use std::time::{SystemTime, Instant, Duration};
use std::convert::TryInto;
use std::thread::sleep;

use bitflags::bitflags;

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
    SessionConfig(SessionConfig),
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
}

#[derive(Debug, Clone)]
struct ConnectAccept {
    assigned_client_id: u8,
    session_id: u32,
}

#[derive(Debug, Clone, Copy)]
struct Ping {
    timestamp: u64,
}

#[derive(Debug, Clone)]
struct Pong {
    original_timestamp: u64,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct FeatureSet: u64 {
        const MOVEMENT = 0b00000001;
        const RAGDOLL = 0b00000010;
        const INVENTORY = 0b00000100;
        const WEAPONS = 0b00001000;
        const EMOTES = 0b00010000;
        const ABILITIES = 0b00100000;
        const CUSTOM_UI = 0b01000000;
        const VOIP  = 0b10000000;
    }
}

#[derive(Debug, Clone)]
struct SessionConfig {
    version: u8,
    tick_rate: u16,
    feature_flags: FeatureSet,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum PacketType {
    ConnectRequest = 0x01,
    ConnectAccept = 0x02,
    ConnectDeny = 0x03,
    SessionConfig = 0x04,
    Ping = 0x0B,
    Pong = 0x0C,
    DisconnectNotice = 0x0D,
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
                bytes.extend(req.desired_name.as_bytes());
                bytes
            }
            PacketPayload::ConnectAccept(accept) => {
                let mut bytes = vec![accept.assigned_client_id];
                bytes.extend(&accept.session_id.to_le_bytes());
                bytes
            }
            PacketPayload::SessionConfig(config) => {
                let mut bytes = vec![config.version];
                bytes.extend(&config.tick_rate.to_le_bytes());
                bytes.extend(&config.feature_flags.bits().to_le_bytes());
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
                if data.len() < 5 {
                    return Err(Error::new(ErrorKind::InvalidData, "ConnectRequest too short"));
                }
                let client_version = data[0];
                let target_session_id = u32::from_le_bytes(data[1..5].try_into().unwrap());
                let desired_name = String::from_utf8_lossy(&data[5..]).to_string();
                Ok(PacketPayload::ConnectRequest(ConnectRequest {
                    client_version,
                    desired_name,
                    target_session_id,
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
            x if x == PacketType::SessionConfig as u8 => {
                if data.len() < 11 {
                    return Err(Error::new(ErrorKind::InvalidData, "SessionConfig too short"));
                }
                let version = data[0];
                let tick_rate = u16::from_le_bytes([data[1], data[2]]);
                let feature_flags = FeatureSet::from_bits_truncate(u64::from_le_bytes(data[3..11].try_into().unwrap()));
                Ok(PacketPayload::SessionConfig(SessionConfig {
                    version,
                    tick_rate,
                    feature_flags,
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
        if data.len() < 7 {
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
        Ok((NeonPacket {
            packet_type: header.packet_type,
            sequence: header.sequence,
            client_id: header.client_id,
            destination_id: header.destination_id,
            payload,
        }, addr))
    }
}

struct NeonClient {
    socket: NeonSocket,
    relay_addr: Option<SocketAddr>,
    client_id: Option<u8>,
    session_id: Option<u32>,
    name: String,
}

impl NeonClient {
    fn new(name: String) -> Result<Self, Error> {
        Ok(Self {
            socket: NeonSocket::new("0.0.0.0:0")?,
            relay_addr: None,
            client_id: None,
            session_id: None,
            name,
        })
    }

    fn connect_to_session(&mut self, relay_addr: SocketAddr, target_session_id: u32) -> Result<(), Error> {
        self.relay_addr = Some(relay_addr);
        self.socket.socket.set_nonblocking(false)?;

        let connect_req = ConnectRequest {
            client_version: 1,
            desired_name: self.name.clone(),
            target_session_id,
        };
        let connect_packet = NeonPacket {
            packet_type: PacketType::ConnectRequest as u8,
            sequence: 1,
            client_id: 0,
            destination_id: 1,
            payload: PacketPayload::ConnectRequest(connect_req.clone()),
        };

        println!("Attempting to connect to session {} via relay...", target_session_id);
        self.socket.send_packet(&connect_packet, relay_addr)?;

        self.socket.socket.set_read_timeout(Some(Duration::from_secs(10)))?;

        let (response, _) = self.socket.receive_packet()?;
        self.socket.socket.set_nonblocking(true)?;
        self.socket.socket.set_read_timeout(None)?;

        if let PacketPayload::ConnectAccept(accept) = response.payload {
            let client_id = accept.assigned_client_id;
            let session_id = accept.session_id;
            
            if session_id != target_session_id {
                return Err(Error::new(ErrorKind::ConnectionRefused, 
                    format!("Session ID mismatch: requested {}, got {}", target_session_id, session_id)));
            }
            
            self.client_id = Some(client_id);
            self.session_id = Some(session_id);

            let register_packet = NeonPacket {
                packet_type: PacketType::ConnectAccept as u8,
                sequence: 2,
                client_id,
                destination_id: 1,
                payload: PacketPayload::ConnectAccept(accept),
            };
            self.socket.send_packet(&register_packet, relay_addr)?;

            println!("Successfully connected to session {}! Assigned client ID: {}", session_id, client_id);
            Ok(())
        } else {
            Err(Error::new(ErrorKind::ConnectionAborted, "Invalid ConnectAccept response"))
        }
    }

    fn send_ping(&self) -> Result<(), Error> {
        if let (Some(relay_addr), Some(client_id)) = (self.relay_addr, self.client_id) {
            let timestamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            let packet = NeonPacket {
                packet_type: PacketType::Ping as u8,
                sequence: 10,
                client_id,
                destination_id: 1,
                payload: PacketPayload::Ping(Ping { timestamp }),
            };

            println!("Sending ping @ {}", timestamp);
            self.socket.send_packet(&packet, relay_addr)
        } else {
            Err(Error::new(ErrorKind::NotConnected, "Client not initialized"))
        }
    }

    fn process_incoming_packets(&mut self) -> Result<(), Error> {
        loop {
            match self.socket.receive_packet() {
                Ok((packet, _)) => {
                    if packet.destination_id == self.client_id.unwrap() {
                        match packet.payload {
                                PacketPayload::Pong(pong) => {
                                    let pong_time = SystemTime::now()
                                        .duration_since(SystemTime::UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis() as u64;
                                    println!("Got pong! Response time: {} ms", (pong_time - pong.original_timestamp));
                                }
                                PacketPayload::SessionConfig(config) => {
                                    println!("Session config: {:?}", config);
                                }
                                _ => {
                                    println!("Unhandled packet: {:?}", packet);
                                }
                        }
                    } else {
                        println!("Packet not for me! My ID: {} Packet ID: {}", self.client_id.unwrap(), packet.client_id)
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

fn get_user_input(prompt: &str) -> String {
    print!("{}", prompt);
    stdout().flush().unwrap();
    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn main() {
    println!("Project Neon Alpha Build 12 - Client");
    println!("==============================================");
    
    let client_name = get_user_input("Enter your client name: ");
    if client_name.is_empty() {
        println!("Client name cannot be empty!");
        return;
    }
    
    let session_input = get_user_input("Enter session ID to connect to: ");
    let target_session_id: u32 = match session_input.parse() {
        Ok(id) => id,
        Err(_) => {
            println!("Invalid session ID! Please enter a valid number.");
            return;
        }
    };
    
    let relay_input = get_user_input("Enter relay address (default: 127.0.0.1:7777): ");
    let relay_addr_str = if relay_input.is_empty() {
        "127.0.0.1:7777"
    } else {
        &relay_input
    };
    
    let relay_addr: SocketAddr = match relay_addr_str.parse() {
        Ok(addr) => addr,
        Err(_) => {
            println!("Invalid relay address format!");
            return;
        }
    };

    println!("\nAttempting connection...");
    println!("Client Name: {}", client_name);
    println!("Target Session ID: {}", target_session_id);
    println!("Relay Address: {}", relay_addr);
    println!();

    let mut client = match NeonClient::new(client_name) {
        Ok(client) => client,
        Err(e) => {
            println!("Failed to create client: {}", e);
            return;
        }
    };

    match client.connect_to_session(relay_addr, target_session_id) {
        Ok(()) => {
            println!("Connection successful! Starting ping loop...");
            println!("Press Ctrl+C to disconnect.\n");
        }
        Err(e) => {
            println!("Failed to connect to session: {}", e);
            println!("Make sure:");
            println!("1. The relay server is running");
            println!("2. A host is running with the specified session ID");
            println!("3. The session ID is correct");
            return;
        }
    }

    let ping_interval = Duration::from_secs(5);
    let mut last_ping = Instant::now() - ping_interval;

    loop {
        if last_ping.elapsed() >= ping_interval {
            if let Err(e) = client.send_ping() {
                println!("Failed to send ping: {}", e);
                break;
            }
            last_ping = Instant::now();
        }

        if let Err(e) = client.process_incoming_packets() {
            println!("Error processing packets: {}", e);
            break;
        }

        sleep(Duration::from_millis(10));
    }
}