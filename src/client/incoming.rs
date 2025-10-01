use std::net::SocketAddr;
use std::io::{Error, ErrorKind};
use crate::types::*;

pub struct NeonSocket {
    pub socket: std::net::UdpSocket,
}

impl NeonSocket {
    pub fn new(bind_addr: &str) -> Result<Self, Error> {
        let socket = std::net::UdpSocket::bind(bind_addr)?;
        socket.set_nonblocking(true)?;
        Ok(Self { socket })
    }

    pub fn send_packet(&self, packet: &NeonPacket, addr: SocketAddr) -> Result<(), Error> {
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

    pub fn receive_packet(&self) -> Result<(NeonPacket, SocketAddr), Error> {
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

pub fn process_incoming_packets(
    socket: &NeonSocket,
    client_id: u8,
) -> Result<(), Error> {
    loop {
        match socket.receive_packet() {
            Ok((packet, _)) => {
                if packet.destination_id == client_id {
                    match packet.payload {
                        PacketPayload::Pong(pong) => {
                            let pong_time = std::time::SystemTime::now()
                                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64;
                            println!("Got pong! Response time: {} ms", (pong_time - pong.original_timestamp));
                        }
                        PacketPayload::SessionConfig(config) => {
                            println!("Session config: {:?}", config);
                        }
                        PacketPayload::PacketTypeRegistry(registry) => {
                            println!("Received PacketTypeRegistry:");
                            for entry in registry.entries {
                                println!("  0x{:02X}: {} - {}", entry.packet_id, entry.name, entry.description);
                            }
                        }
                        _ => {
                            println!("Unhandled packet: {:?}", packet);
                        }
                    }
                } else {
                    println!("Packet not for me! My ID: {} Packet ID: {}", client_id, packet.client_id)
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