use std::net::SocketAddr;
use std::io::Error;
use super::types::*;
use super::incoming::NeonSocket;

pub fn send_host_registration(
    socket: &NeonSocket,
    relay_addr: SocketAddr,
    host_client_id: u8,
    session_id: u32,
) -> Result<(), Error> {
    let host_register_packet = NeonPacket {
        packet_type: PacketType::ConnectAccept as u8,
        sequence: 0,
        client_id: host_client_id,
        destination_id: 1,
        payload: PacketPayload::ConnectAccept(ConnectAccept {
            assigned_client_id: host_client_id,
            session_id,
        }),
    };

    socket.send_packet(&host_register_packet, relay_addr)?;
    Ok(())
}

pub fn send_connect_accept(
    socket: &NeonSocket,
    relay_addr: SocketAddr,
    assigned_id: u8,
    session_id: u32,
) -> Result<(), Error> {
    let accept = ConnectAccept {
        assigned_client_id: assigned_id,
        session_id,
    };

    let accept_packet = NeonPacket {
        packet_type: PacketType::ConnectAccept as u8,
        sequence: 1,
        client_id: assigned_id,
        destination_id: assigned_id,
        payload: PacketPayload::ConnectAccept(accept),
    };

    socket.send_packet(&accept_packet, relay_addr)?;
    println!("[Host] Sent ConnectAccept to relay for client {}", assigned_id);
    Ok(())
}

pub fn send_connect_deny(
    socket: &NeonSocket,
    relay_addr: SocketAddr,
    reason: String,
) -> Result<(), Error> {
    let deny_packet = NeonPacket {
        packet_type: PacketType::ConnectDeny as u8,
        sequence: 1,
        client_id: 1,
        destination_id: 0,
        payload: PacketPayload::ConnectDeny(ConnectDeny { reason }),
    };

    socket.send_packet(&deny_packet, relay_addr)?;
    println!("[Host] Sent ConnectDeny to relay");
    Ok(())
}

pub fn send_session_config(
    socket: &NeonSocket,
    relay_addr: SocketAddr,
    assigned_id: u8,
) -> Result<(), Error> {
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

    socket.send_packet(&config_packet, relay_addr)?;
    println!("[Host] Sent SessionConfig to relay for client {}", assigned_id);
    Ok(())
}

pub fn send_packet_type_registry(
    socket: &NeonSocket,
    relay_addr: SocketAddr,
    assigned_id: u8,
) -> Result<(), Error> {
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
    
    socket.send_packet(&registry_packet, relay_addr)?;
    println!("[Host] Sent PacketTypeRegistry to relay for client {}", assigned_id);
    Ok(())
}