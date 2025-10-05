use std::net::{SocketAddr, UdpSocket};
use std::io::{Error, ErrorKind};
use super::types::*;

pub struct NeonSocket {
    pub socket: UdpSocket,
}

impl NeonSocket {
    pub fn new(bind_addr: &str) -> Result<Self, Error> {
        let socket = UdpSocket::bind(bind_addr)?;
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

pub fn handle_ping(
    socket: &NeonSocket,
    relay_addr: SocketAddr,
    host_client_id: u8,
    packet: &NeonPacket,
    ping: &Ping,
) -> Result<(), Error> {
    let pong_packet = NeonPacket {
        packet_type: PacketType::Pong as u8,
        sequence: packet.sequence,
        client_id: host_client_id,
        destination_id: packet.client_id,
        payload: PacketPayload::Pong(Pong {
            original_timestamp: ping.timestamp,
        }),
    };
    socket.send_packet(&pong_packet, relay_addr)?;
    println!("[Host] Responded to ping from client {}", packet.client_id);
    Ok(())
}