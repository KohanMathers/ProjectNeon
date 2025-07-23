use std::net::{UdpSocket, SocketAddr};
use std::io::{Error, ErrorKind};
use std::time::SystemTime;
use std::convert::TryInto;
use std::time::{Instant, Duration};
use std::thread::sleep;

#[derive(Debug, Clone)]
struct PacketHeader {
    magic: u16,
    version: u8,
    packet_type: u8,
    sequence: u16,
    client_id: u8,
}

#[derive(Debug, Clone)]
enum PacketPayload {
    None,
    Ping(Ping),
    Pong(Pong),
    ConnectRequest(ConnectRequest),
    ConnectAccept(ConnectAccept),
}

#[derive(Debug, Clone)]
struct NeonPacket {
    packet_type: u8,
    sequence: u16,
    client_id: u8,
    payload: PacketPayload,
}

struct NeonSocket {
    socket: UdpSocket,
}

#[derive(Debug, Clone)]
struct ConnectRequest {
    client_version: u8,
    desired_name: String,
}

#[derive(Debug, Clone)]
struct ConnectAccept {
    assigned_client_id: u8,
    session_id: u32,
}

#[derive(Debug, Clone)]
struct Ping {
    timestamp: u64,
}

#[derive(Debug, Clone)]
struct Pong {
    original_timestamp: u64,
}

struct NeonClient {
    socket: NeonSocket,
    host_addr: Option<SocketAddr>,
    client_id: Option<u8>,
    connected: bool,
    name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum PacketType {
    ConnectRequest = 0x01,
    ConnectAccept = 0x02,
    Ping = 0x0B,
    Pong = 0x0C,
}

impl PacketPayload {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            PacketPayload::None => vec![],
            PacketPayload::Ping(ping) => ping.timestamp.to_le_bytes().to_vec(),
            PacketPayload::Pong(pong) => pong.original_timestamp.to_le_bytes().to_vec(),
            _ => vec![],
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
                Ok(PacketPayload::Pong(Pong { original_timestamp: timestamp }))
            }
            _ => Ok(PacketPayload::None),
        }
    }
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
        };
        let mut bytes = header.to_bytes();
        bytes.extend(packet.payload.to_bytes());
        self.socket.send_to(&bytes, addr)?;
        Ok(())
    }

    fn receive_packet(&self) -> Result<(NeonPacket, SocketAddr), Error> {
        let mut buf = [0; 1024];
        let (size, addr) = self.socket.recv_from(&mut buf)?;
        let header = PacketHeader::from_bytes(&buf[..7])?;
        let payload = PacketPayload::from_bytes(header.packet_type, &buf[7..size])?;
        Ok((
            NeonPacket {
                packet_type: header.packet_type,
                sequence: header.sequence,
                client_id: header.client_id,
                payload,
            },
            addr,
        ))
    }
}

impl NeonClient {
    fn new(name: String) -> Result<Self, Error> {
        let socket = NeonSocket::new("0.0.0.0:0")?;
        Ok(NeonClient {
            socket,
            host_addr: None,
            client_id: None,
            connected: false,
            name,
        })
    }

    fn connect_to_host(&mut self, host_addr: SocketAddr) -> Result<(), Error> {
        println!("Connecting to host at {}", host_addr);
        self.host_addr = Some(host_addr);

        let connect_packet = NeonPacket {
            packet_type: PacketType::ConnectRequest as u8,
            sequence: 1,
            client_id: 0,
            payload: PacketPayload::ConnectRequest(ConnectRequest {
                client_version: 1,
                desired_name: self.name.clone(),
            }),
        };

        self.socket.send_packet(&connect_packet, host_addr)?;

        let (response, _) = self.socket.receive_packet()?;
        if response.packet_type == PacketType::ConnectAccept as u8 {
            self.client_id = Some(response.client_id);
            self.connected = true;
            println!("Connected! Assigned client ID: {}", response.client_id);
            Ok(())
        } else {
            Err(Error::new(ErrorKind::ConnectionRefused, "Connection rejected"))
        }
    }

    fn send_to_host(&self, packet: &NeonPacket) -> Result<(), Error> {
        if let Some(host_addr) = self.host_addr {
            self.socket.send_packet(packet, host_addr)
        } else {
            Err(Error::new(ErrorKind::NotConnected, "Not connected to host"))
        }
    }

    fn send_ping(&self) -> Result<(), Error> {
        if let Some(client_id) = self.client_id {
            let ping = Ping {
                timestamp: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            };
            let packet = NeonPacket {
                packet_type: PacketType::Ping as u8,
                sequence: 0,
                client_id,
                payload: PacketPayload::Ping(ping),
            };
            self.send_to_host(&packet)
        } else {
            Err(Error::new(ErrorKind::NotConnected, "No client ID assigned"))
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

        let version = data[2];
        let packet_type = data[3];
        let sequence = u16::from_le_bytes([data[4], data[5]]);
        let client_id = data[6];

        Ok(PacketHeader {
            magic,
            version,
            packet_type,
            sequence,
            client_id,
        })
    }
}

fn main() {
    println!("Project Neon Alpha Build 2 - Client");
    println!("Starting client...");
    let mut client = NeonClient::new("TestClient".to_string()).expect("Failed to create client");
    let host_addr = "127.0.0.1:7777".parse().expect("Invalid host address");
    client.connect_to_host(host_addr).expect("Failed to connect");

    let ping_interval = Duration::from_secs(5);
    let mut last_ping = Instant::now() - ping_interval;

    loop {
        if last_ping.elapsed() >= ping_interval {
            client.send_ping().expect("Ping failed");
            last_ping = Instant::now();
        }

        match client.socket.receive_packet() {
            Ok((packet, _)) => {
                match packet.packet_type {
                    x if x == PacketType::Pong as u8 => {
                        if let PacketPayload::Pong(pong) = packet.payload {
                            println!("Received pong with timestamp: {}", pong.original_timestamp);
                        }
                    }
                    _ => println!("Received unexpected packet: {:?}", packet),
                }
            }
            Err(e) => {
                println!("Error receiving packet: {:?}", e);
                sleep(Duration::from_millis(10));
            }
        }
    }
}