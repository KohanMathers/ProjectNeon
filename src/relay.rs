use std::collections::HashMap;
use std::convert::TryInto;
use std::io::{Error, ErrorKind};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};
use std::thread::sleep;

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

#[derive(Debug, Clone, Copy)]
struct SessionConfig {
    version: u8,
    tick_rate: u16,
    feature_flags: FeatureSet,
}

bitflags::bitflags! {
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
                let timestamp = u64::from_le_bytes(data[0..8].try_into().unwrap());
                Ok(PacketPayload::Ping(Ping { timestamp }))
            }
            x if x == PacketType::Pong as u8 => {
                if data.len() < 8 {
                    return Err(Error::new(ErrorKind::InvalidData, "Pong too short"));
                }
                let timestamp = u64::from_le_bytes(data[0..8].try_into().unwrap());
                Ok(PacketPayload::Pong(Pong {
                    original_timestamp: timestamp,
                }))
            }
            x if x == PacketType::ConnectRequest as u8 => {
                if data.len() < 5 {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "ConnectRequest too short",
                    ));
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
            x if x == PacketType::SessionConfig as u8 => {
                if data.len() < 11 {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "SessionConfig too short",
                    ));
                }
                let version = data[0];
                let tick_rate = u16::from_le_bytes([data[1], data[2]]);
                let feature_flags = FeatureSet::from_bits_truncate(u64::from_le_bytes(
                    data[3..11].try_into().unwrap(),
                ));
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

struct NeonSocket {
    socket: UdpSocket,
}

impl NeonSocket {
    fn new(addr: &str) -> Result<Self, Error> {
        let socket = UdpSocket::bind(addr)?;
        Ok(NeonSocket { socket })
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

#[derive(Debug, Clone)]
struct PeerInfo {
    addr: SocketAddr,
    client_id: u8,
    session_id: u32,
    last_seen: Instant,
    is_host: bool,
}

#[derive(Debug, Clone)]
struct PendingConnection {
    client_addr: SocketAddr,
    session_id: u32,
    client_name: String,
}

struct RelayNode {
    socket: NeonSocket,
    sessions: HashMap<u32, Vec<PeerInfo>>,
    hosts: HashMap<u32, SocketAddr>,
    pending_connections: HashMap<SocketAddr, PendingConnection>,
}

impl RelayNode {
    fn new(bind_addr: &str) -> Result<Self, Error> {
        Ok(RelayNode {
            socket: NeonSocket::new(bind_addr)?,
            sessions: HashMap::new(),
            hosts: HashMap::new(),
            pending_connections: HashMap::new(),
        })
    }

    fn cleanup_dead_connections(&mut self) {
        let timeout = Duration::from_secs(15);
        let now = Instant::now();

        let mut sessions_to_remove: Vec<u32> = Vec::new();

        for (session_id, peers) in &mut self.sessions {
            peers.retain(|peer| {
                if !peer.is_host {
                    let is_alive = now.duration_since(peer.last_seen) < timeout;
                    if !is_alive {
                        println!(
                            "[Relay] Client {} in session {} timed out",
                            peer.client_id, session_id
                        );
                    }
                    is_alive
                } else {
                    true
                }
            });

            if peers.is_empty() {
                sessions_to_remove.push(*session_id);
            }
        }

        for session_id in sessions_to_remove {
            self.sessions.remove(&session_id);
            self.hosts.remove(&session_id);
            println!("[Relay] Removed empty session {}", session_id)
        }
    }

    fn update_client_activity(&mut self, client_id: u8, session_id: u32) {
        if let Some(peers) = self.sessions.get_mut(&session_id) {
            for peer in peers.iter_mut() {
                if peer.client_id == client_id {
                    peer.last_seen = Instant::now();
                    break;
                }
            }
        }
    }

    fn find_session_for_addr(&self, addr: SocketAddr) -> Option<u32> {
        for (session_id, peers) in &self.sessions {
            if peers.iter().any(|p| p.addr == addr) {
                return Some(*session_id);
            }
        }
        None
    }

    fn run(&mut self) -> Result<(), Error> {
        println!("Relay node listening on {}...", "0.0.0.0:7777");
        println!();
        
        self.socket.socket.set_nonblocking(true)?;
        
        let mut last_cleanup = Instant::now();
        let cleanup_interval = Duration::from_secs(5);

        loop {
            match self.socket.receive_packet() {
                Ok((packet, addr)) => {
                    match packet.packet_type {
                        x if x == PacketType::ConnectRequest as u8 => {
                            if let PacketPayload::ConnectRequest(req) = packet.payload {
                                self.handle_connect_request(req, addr)?;
                            }
                        }
                        x if x == PacketType::ConnectAccept as u8 => {
                            if let PacketPayload::ConnectAccept(accept) = packet.payload.clone() {
                                if let Some(host_addr) = self.hosts.get(&accept.session_id) {
                                    if addr == *host_addr && packet.client_id != 1 {
                                        self.route_connect_accept_to_client(accept, packet.client_id)?;
                                        continue;
                                    }
                                }

                                if packet.client_id == 1 {
                                    self.register_host(accept.session_id, addr);
                                } else {
                                    self.register_client(accept.session_id, packet.client_id, addr);
                                }
                            }
                        }
                        _ => {
                            self.forward_to_peers(&packet, addr)?;
                            if let Some(session_id) = self.find_session_for_addr(addr) {
                                self.update_client_activity(packet.client_id, session_id);
                            }
                        }
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    // No packets available, continue to cleanup check
                }
                Err(e) => return Err(e),
            }

            if last_cleanup.elapsed() >= cleanup_interval {
                self.cleanup_dead_connections();
                last_cleanup = Instant::now();
            }

            sleep(Duration::from_millis(1));
        }
    }

    fn handle_connect_request(
        &mut self,
        req: ConnectRequest,
        client_addr: SocketAddr,
    ) -> Result<(), Error> {
        let target_session = req.target_session_id;

        println!(
            "[Relay] Client '{}' from {} requesting to join session {}",
            req.desired_name, client_addr, target_session
        );
        // Debug: print raw bytes of ConnectRequest as received
        let mut debug_bytes = vec![req.client_version];
        debug_bytes.extend(&req.target_session_id.to_le_bytes());
        debug_bytes.extend(req.desired_name.as_bytes());
        println!("[DEBUG] Received ConnectRequest raw bytes: {:?}", debug_bytes);

        if let Some(host_addr) = self.hosts.get(&target_session) {
            println!(
                "[Relay] Forwarding connection request to host at {}",
                host_addr
            );

            self.pending_connections.insert(
                client_addr,
                PendingConnection {
                    client_addr,
                    session_id: target_session,
                    client_name: req.desired_name.clone(),
                },
            );

            let forward_packet = NeonPacket {
                packet_type: PacketType::ConnectRequest as u8,
                sequence: 1,
                client_id: 0,
                destination_id: 1, // always to host
                payload: PacketPayload::ConnectRequest(req.clone()),
            };
            // Debug: print raw bytes of ConnectRequest as forwarded
            let payload_bytes = PacketPayload::ConnectRequest(req).to_bytes();
            println!("[DEBUG] Forwarded ConnectRequest raw bytes: {:?}", payload_bytes);

            self.socket.send_packet(&forward_packet, *host_addr)?;
        } else {
            println!(
                "[Relay] Session {} not found (no host registered)",
                target_session
            );
        }

        Ok(())
    }

    fn route_connect_accept_to_client(
        &mut self,
        accept: ConnectAccept,
        client_id: u8,
    ) -> Result<(), Error> {
        let mut client_addr_to_send = None;

        for (addr, pending) in &self.pending_connections {
            if pending.session_id == accept.session_id {
                client_addr_to_send = Some(*addr);
                break;
            }
        }

        if let Some(client_addr) = client_addr_to_send {
            println!(
                "[Relay] Routing ConnectAccept for client {} back to {}",
                client_id, client_addr
            );

            let response_packet = NeonPacket {
                packet_type: PacketType::ConnectAccept as u8,
                sequence: 1,
                client_id,
                destination_id: client_id, // to the client
                payload: PacketPayload::ConnectAccept(accept),
            };

            self.socket.send_packet(&response_packet, client_addr)?;

            self.pending_connections.remove(&client_addr);
        } else {
            println!("[Relay] No pending connection found for ConnectAccept");
        }

        Ok(())
    }

    fn register_host(&mut self, session_id: u32, addr: SocketAddr) {
        self.hosts.insert(session_id, addr);

        let peer = PeerInfo {
            addr,
            client_id: 1,
            session_id,
            is_host: true,
            last_seen: Instant::now(),
        };

        self.sessions
            .entry(session_id)
            .or_insert_with(Vec::new)
            .retain(|p| p.client_id != 1);
        self.sessions.get_mut(&session_id).unwrap().push(peer);

        println!(
            "[Relay] Host registered for session {} at {}",
            session_id, addr
        );
        self.print_active_sessions();
    }

    fn register_client(&mut self, session_id: u32, client_id: u8, addr: SocketAddr) {
        let peer = PeerInfo {
            addr,
            client_id,
            session_id,
            is_host: false,
            last_seen: Instant::now(),
        };

        self.sessions
            .entry(session_id)
            .or_insert_with(Vec::new)
            .retain(|p| p.client_id != client_id);
        self.sessions.get_mut(&session_id).unwrap().push(peer);

        println!(
            "[Relay] Client {} registered to session {} from {}",
            client_id, session_id, addr
        );
        self.print_session_info(session_id);
    }

    fn forward_to_peers(&self, packet: &NeonPacket, sender_addr: SocketAddr) -> Result<(), Error> {
        for (session_id, peers) in &self.sessions {
            if let Some(sender) = peers.iter().find(|p| p.addr == sender_addr) {
                let mut forwarded = false;

                for peer in peers {
                    if peer.addr != sender_addr {
                        match self.socket.send_packet(packet, peer.addr) {
                            Ok(()) => {
                                forwarded = true;
                            }
                            Err(e) => {
                                println!(
                                    "[Relay] Failed to forward packet to {}: {}",
                                    peer.addr, e
                                );
                            }
                        }
                    }
                }

                return Ok(());
            }
        }

        println!("[Relay] Unknown sender: {}, dropping packet", sender_addr);
        Ok(())
    }

    fn print_active_sessions(&self) {
        println!("\n=== Active Sessions ===");
        if self.sessions.is_empty() {
            println!("No active sessions");
        } else {
            for (session_id, peers) in &self.sessions {
                let host_count = peers.iter().filter(|p| p.is_host).count();
                let client_count = peers.iter().filter(|p| !p.is_host).count();
                println!(
                    "Session {}: {} host(s), {} client(s)",
                    session_id, host_count, client_count
                );
            }
        }
        println!("========================\n");
    }

    fn print_session_info(&self, session_id: u32) {
        if let Some(peers) = self.sessions.get(&session_id) {
            let clients: Vec<_> = peers.iter().filter(|p| !p.is_host).collect();
            println!(
                "  Session {} now has {} client(s) connected",
                session_id,
                clients.len()
            );
        }
    }
}

fn main() {
    println!("Project Neon Alpha Build 12 - Relay");
    println!("===================================");
    println!("Starting relay node...");

    let mut relay = RelayNode::new("0.0.0.0:7777").expect("Failed to start relay");
    relay.run().expect("Relay failed");
}
