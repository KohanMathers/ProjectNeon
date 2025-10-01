mod types;
mod incoming;
mod outgoing;

use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::thread::sleep;
use std::time::Duration;
use std::env;

use types::*;
use incoming::{NeonSocket, handle_ping};
use outgoing::*;

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

        send_host_registration(&self.socket, self.relay_addr, self.client_id, self.session_id)?;

        loop {
            match self.socket.receive_packet() {
                Ok((packet, addr)) => match packet.payload {
                    PacketPayload::ConnectRequest(req) => {
                        self.handle_connect_request(req, addr)?;
                    }
                    PacketPayload::Ping(ping) => {
                        handle_ping(&self.socket, self.relay_addr, self.client_id, &packet, &ping)?;
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

            send_connect_deny(
                &self.socket,
                self.relay_addr,
                format!("Name '{}' is already in use", req.desired_name),
            )?;
            return Ok(());
        }

        let assigned_id = self.next_client_id;
        self.next_client_id += 1;

        println!(
            "[Host] New client '{}' requesting to join session {} -> assigned ID {}",
            req.desired_name, req.target_session_id, assigned_id
        );

        send_connect_accept(&self.socket, self.relay_addr, assigned_id, self.session_id)?;
        send_session_config(&self.socket, self.relay_addr, assigned_id)?;
        send_packet_type_registry(&self.socket, self.relay_addr, assigned_id)?;

        self.connected_clients.insert(assigned_id, req.desired_name.clone());
        println!(
            "[Host] Client {} ('{}') accepted and configured",
            assigned_id, req.desired_name
        );

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