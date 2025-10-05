mod types;
mod incoming;
mod outgoing;

use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::thread::sleep;
use std::time::Duration;

use types::*;
use incoming::{NeonSocket, handle_ping};
use outgoing::*;

pub struct NeonHost {
    socket: NeonSocket,
    relay_addr: SocketAddr,
    client_id: u8,
    session_id: u32,
    connected_clients: HashMap<u8, String>,
    next_client_id: u8,
}

impl NeonHost {
    /// Create a new host with a specific session ID and relay address
    pub fn new(session_id: u32, relay_addr: &str) -> Result<Self, Error> {
        let relay_addr = relay_addr.parse()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid relay address"))?;
        
        Ok(Self {
            socket: NeonSocket::new("0.0.0.0:0")?,
            relay_addr,
            client_id: 1,
            session_id,
            connected_clients: HashMap::new(),
            next_client_id: 2,
        })
    }

    /// Get the session ID
    pub fn session_id(&self) -> u32 {
        self.session_id
    }

    /// Get the relay address
    pub fn relay_addr(&self) -> SocketAddr {
        self.relay_addr
    }

    /// Get connected client count
    pub fn client_count(&self) -> usize {
        self.connected_clients.len()
    }

    /// Start the host and begin accepting connections
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