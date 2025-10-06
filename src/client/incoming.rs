use std::net::SocketAddr;
use std::io::{Error, ErrorKind};
use super::types::*;

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
    relay_addr: SocketAddr,
    client_id: u8,
    on_pong: &mut Option<Box<dyn FnMut(u64, u64) + Send>>,
    on_session_config: &mut Option<Box<dyn FnMut(u8, u16, u16) + Send>>,
    on_packet_type_registry: &mut Option<Box<dyn FnMut(Vec<(u8, String, String)>) + Send>>,
    on_unhandled_packet: &mut Option<Box<dyn FnMut(u8, u8) + Send>>,
    on_wrong_destination: &mut Option<Box<dyn FnMut(u8, u8) + Send>>,
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
                            let response_time = pong_time - pong.original_timestamp;
                            
                            if let Some(callback) = on_pong {
                                callback(response_time, pong_time);
                            }
                        }
                        PacketPayload::SessionConfig(config) => {
                            send_ack(socket, relay_addr, client_id, packet.sequence)?;

                            if let Some(callback) = on_session_config {
                                callback(config.version, config.tick_rate, config.max_packet_size);
                            }
                        }
                        PacketPayload::PacketTypeRegistry(registry) => {
                            let entries: Vec<(u8, String, String)> = registry.entries
                                .into_iter()
                                .map(|e| (e.packet_id, e.name, e.description))
                                .collect();
                            
                            if let Some(callback) = on_packet_type_registry {
                                callback(entries);
                            }
                        }
                        _ => {
                            if let Some(callback) = on_unhandled_packet {
                                callback(packet.packet_type, packet.client_id);
                            }
                        }
                    }
                } else {
                    if let Some(callback) = on_wrong_destination {
                        callback(client_id, packet.destination_id);
                    }
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

fn send_ack(
    socket: &NeonSocket,
    relay_addr: SocketAddr,
    client_id: u8,
    sequence: u16,
) -> Result<(), Error> {
    let ack_packet = NeonPacket {
        packet_type: PacketType::Ack as u8,
        sequence: 0,
        client_id,
        destination_id: 1,
        payload: PacketPayload::Ack(Ack {
            acknowledged_sequences: vec![sequence],
        }),
    };
    socket.send_packet(&ack_packet, relay_addr)
}