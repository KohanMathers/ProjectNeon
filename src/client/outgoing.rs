use std::net::SocketAddr;
use std::io::{Error, ErrorKind};
use std::time::{SystemTime, Duration};
use super::types::*;
use super::incoming::NeonSocket;

pub fn send_connect_request(
    socket: &NeonSocket,
    relay_addr: SocketAddr,
    client_name: &str,
    target_session_id: u32,
) -> Result<(), Error> {
    let connect_req = ConnectRequest {
        client_version: 1,
        desired_name: client_name.to_string(),
        target_session_id,
        game_identifier: 0,
    };
    
    let connect_packet = NeonPacket {
        packet_type: PacketType::ConnectRequest as u8,
        sequence: 1,
        client_id: 0,
        destination_id: 1,
        payload: PacketPayload::ConnectRequest(connect_req),
    };

    println!("Attempting to connect to session {} via relay...", target_session_id);
    socket.send_packet(&connect_packet, relay_addr)?;
    Ok(())
}

pub fn send_connect_accept_confirmation(
    socket: &NeonSocket,
    relay_addr: SocketAddr,
    client_id: u8,
    accept: ConnectAccept,
) -> Result<(), Error> {
    let register_packet = NeonPacket {
        packet_type: PacketType::ConnectAccept as u8,
        sequence: 2,
        client_id,
        destination_id: 1,
        payload: PacketPayload::ConnectAccept(accept),
    };
    socket.send_packet(&register_packet, relay_addr)?;
    Ok(())
}

pub fn send_ping(
    socket: &NeonSocket,
    relay_addr: SocketAddr,
    client_id: u8,
) -> Result<(), Error> {
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
    socket.send_packet(&packet, relay_addr)
}

pub fn wait_for_connect_response(
    socket: &NeonSocket,
    timeout: Duration,
) -> Result<(u8, u32), Error> {
    socket.socket.set_read_timeout(Some(timeout))?;
    
    let (response, _) = socket.receive_packet()?;
    
    socket.socket.set_read_timeout(None)?;

    if let PacketPayload::ConnectAccept(accept) = response.payload {
        Ok((accept.assigned_client_id, accept.session_id))
    } else if let PacketPayload::ConnectDeny(deny) = response.payload {
        Err(Error::new(ErrorKind::ConnectionRefused, deny.reason))
    } else {
        Err(Error::new(ErrorKind::ConnectionAborted, "Invalid ConnectAccept response"))
    }
}