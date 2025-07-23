use std::net::{UdpSocket, SocketAddr};
use std::io::{Error, ErrorKind};
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct PacketHeader {
    magic: u16,
    version: u8,
    packet_type: u8,
    sequence: u16,
    client_id: u8,
}

#[derive(Debug, Clone)]
struct NeonPacket {
    packet_type: u8,
    sequence: u16,
    client_id: u8,
}

struct NeonSocket {
    socket: UdpSocket,
}

#[derive(Debug, Clone)]
struct ClientInfo {
    addr: SocketAddr,
    name: String,
    connected: bool,
}

struct NeonHost {
    socket: NeonSocket,
    connected_clients: HashMap<u8, ClientInfo>,
    next_client_id: u8,
    session_id: u32,
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

struct NeonClient {
    socket: NeonSocket,
    host_addr: Option<SocketAddr>,
    client_id: Option<u8>,
    connected: bool,
    name: String,
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
        let bytes = header.to_bytes();
        self.socket.send_to(&bytes, addr)?;
        Ok(())
    }

    fn receive_packet(&self) -> Result<(NeonPacket, SocketAddr), Error> {
        let mut buf = [0; 1024];
        let (size, addr) = self.socket.recv_from(&mut buf)?;
        let header = PacketHeader::from_bytes(&buf[..size])?;
        Ok((NeonPacket {
            packet_type: header.packet_type,
            sequence: header.sequence,
            client_id: header.client_id,
        }, addr))
    }
}

impl NeonHost {
    fn new(addr: &str) -> Result<Self, Error> {
        let socket = NeonSocket::new(addr)?;
        Ok(NeonHost {
            socket,
            connected_clients: HashMap::new(),
            next_client_id: 1,
            session_id: 12345,
        })
    }

    fn listen_for_connections(&mut self) -> Result<(), Error> {
        println!("Host listening for connections...");
        loop {
            let (packet, addr) = self.socket.receive_packet()?;
            match packet.packet_type {
                x if x == PacketType::ConnectRequest as u8 => {
                    println!("Received connection request from {}", addr);
                    self.handle_connect_request(addr)?;
                }
                x if x == PacketType::Ping as u8 => {
                    println!("Received ping from client {}, responding with pong", packet.client_id);
                    self.socket.send_packet(&NeonPacket {
                        packet_type: PacketType::Pong as u8,
                        sequence: packet.sequence,
                        client_id: packet.client_id,
                    }, addr)?;
                }
                _ => {
                    println!("Received unknown packet type: {}", packet.packet_type);
                }
            }
        }
    }

    fn handle_connect_request(&mut self, from: SocketAddr) -> Result<(), Error> {
        if self.next_client_id >= 255 {
            println!("Server full, rejecting connection from {}", from);
            // TODO: Send ConnectDeny packet
            return Ok(());
        }

        let client_id = self.next_client_id;
        self.next_client_id += 1;

        // Add client to our list
        self.connected_clients.insert(client_id, ClientInfo {
            addr: from,
            name: format!("Client{}", client_id),
            connected: true,
        });

        // Send ConnectAccept
        let accept_packet = NeonPacket {
            packet_type: PacketType::ConnectAccept as u8,
            sequence: 0,
            client_id,
        };

        self.socket.send_packet(&accept_packet, from)?;
        println!("Accepted client {} from {}", client_id, from);
        Ok(())
    }

    fn broadcast_to_clients(&self, packet: &NeonPacket) -> Result<(), Error> {
        for client in self.connected_clients.values() {
            if client.connected {
                self.socket.send_packet(packet, client.addr)?;
            }
        }
        Ok(())
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
            let ping_packet = NeonPacket {
                packet_type: PacketType::Ping as u8,
                sequence: 1,
                client_id,
            };
            self.send_to_host(&ping_packet)
        } else {
            Err(Error::new(ErrorKind::NotConnected, "No client ID assigned"))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum PacketType {
    ConnectRequest = 0x01,
    ConnectAccept = 0x02,
    Ping = 0x0B,
    Pong = 0x0C,
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
    println!("Project Neon Alpha Build 2");
    
    println!("Basic packet test:");
    let packet = NeonPacket {
        packet_type: PacketType::Ping as u8,
        sequence: 1,
        client_id: 1,
    };
    println!("Created packet: {:?}", packet);

    // Server test
    /*
    println!("\nStarting host...");
    let mut host = NeonHost::new("127.0.0.1:7777").unwrap();
    host.listen_for_connections().unwrap();
    */

    // Client test
    /*
    println!("\nStarting client...");
    let mut client = NeonClient::new("TestClient".to_string()).unwrap();
    let host_addr = "127.0.0.1:7777".parse().unwrap();
    client.connect_to_host(host_addr).unwrap();
    client.send_ping().unwrap();
    */
}