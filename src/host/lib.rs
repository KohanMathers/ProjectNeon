mod types;
mod incoming;
mod outgoing;

use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;

use types::*;
use incoming::{NeonSocket, handle_ping};
use outgoing::*;

pub type ClientConnectCallback = Box<dyn FnMut(u8, String, u32) + Send>; // (client_id, name, session_id)
pub type ClientDenyCallback = Box<dyn FnMut(String, String) + Send>; // (name, reason)
pub type PingReceivedCallback = Box<dyn FnMut(u8) + Send>; // (from_client_id)
pub type UnhandledPacketCallback = Box<dyn FnMut(u8, u8, SocketAddr) + Send>; // (packet_type, from_client_id, addr)

pub struct NeonHost {
    socket: NeonSocket,
    relay_addr: SocketAddr,
    client_id: u8,
    session_id: u32,
    connected_clients: HashMap<u8, String>,
    next_client_id: u8,
    pending_acks: HashMap<u8, PendingAck>,

    on_client_connect: Option<ClientConnectCallback>,
    on_client_deny: Option<ClientDenyCallback>,
    on_ping_received: Option<PingReceivedCallback>,
    on_unhandled_packet: Option<UnhandledPacketCallback>,
}

const ACK_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_RETRIES: u8 = 5;

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
            pending_acks: HashMap::new(),
            on_client_connect: None,
            on_client_deny: None,
            on_ping_received: None,
            on_unhandled_packet: None,
        })
    }

    /// Set callback for when a client connects
    pub fn on_client_connect<F>(&mut self, callback: F)
    where
        F: FnMut(u8, String, u32) + Send + 'static,
    {
        self.on_client_connect = Some(Box::new(callback));
    }

    /// Set callback for when a client is denied
    pub fn on_client_deny<F>(&mut self, callback: F)
    where
        F: FnMut(String, String) + Send + 'static,
    {
        self.on_client_deny = Some(Box::new(callback));
    }

    /// Set callback for when a ping is received
    pub fn on_ping_received<F>(&mut self, callback: F)
    where
        F: FnMut(u8) + Send + 'static,
    {
        self.on_ping_received = Some(Box::new(callback));
    }

    /// Set callback for unhandled packets
    pub fn on_unhandled_packet<F>(&mut self, callback: F)
    where
        F: FnMut(u8, u8, SocketAddr) + Send + 'static,
    {
        self.on_unhandled_packet = Some(Box::new(callback));
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
        send_host_registration(&self.socket, self.relay_addr, self.client_id, self.session_id)?;

        loop {
            self.check_pending_acks()?;

            match self.socket.receive_packet() {
                Ok((packet, addr)) => match packet.payload {
                    PacketPayload::ConnectRequest(req) => {
                        self.handle_connect_request(req, addr)?;
                    }
                    PacketPayload::Ack(ack) => {
                        self.handle_ack(packet.client_id, ack)?;
                    }
                    PacketPayload::Ping(_) => {
                        handle_ping(&self.socket, self.relay_addr, self.client_id, &packet)?;
                        
                        if let Some(callback) = &mut self.on_ping_received {
                            callback(packet.client_id);
                        }
                    }
                    _ => {
                        if let Some(callback) = &mut self.on_unhandled_packet {
                            callback(packet.packet_type, packet.client_id, addr);
                        }
                    }
                },
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    sleep(Duration::from_millis(10));
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn check_pending_acks(&mut self) -> Result<(), Error> {
        let mut to_retry = Vec::new();
        let mut to_remove = Vec::new();

    for (client_id, pending) in &self.pending_acks {
            if pending.sent_at.elapsed() >= ACK_TIMEOUT {
                if pending.retry_count >= MAX_RETRIES {
                    to_remove.push(*client_id);
                } else {
                    to_retry.push(*client_id);
                }
            }
        }

    for client_id in to_retry {
            if let Some(pending) = self.pending_acks.get_mut(&client_id) {
                self.socket.send_packet(&pending.packet, self.relay_addr)?;
                pending.sent_at = Instant::now();
                pending.retry_count += 1;
            }
        }

        for client_id in to_remove {
            self.pending_acks.remove(&client_id);
        }

        Ok(())
    }

    fn handle_ack(&mut self, client_id: u8, ack: Ack) -> Result<(), Error> {
        if let Some(pending) = self.pending_acks.get(&client_id) {
            if ack.acknowledged_sequences.contains(&pending.sequence) {
                self.pending_acks.remove(&client_id);
            }
        }

        Ok(())
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
            return Ok(());
        }

        if self.is_name_taken(&req.desired_name) {
            let reason = format!("Name '{}' is already in use", req.desired_name);
            
            if let Some(callback) = &mut self.on_client_deny {
                callback(req.desired_name.clone(), reason.clone());
            }

            send_connect_deny(&self.socket, self.relay_addr, reason)?;
            return Ok(());
        }

        let assigned_id = self.next_client_id;
        self.next_client_id += 1;

        send_connect_accept(&self.socket, self.relay_addr, assigned_id, self.session_id)?;

        // Delay is needed because the client doesn't have enough time to register otherwise
        sleep(Duration::from_millis(50));

        let sequence = 2;
        let config_packet = send_session_config(&self.socket, self.relay_addr, assigned_id, sequence)?;

        self.pending_acks.insert(assigned_id, PendingAck {
            packet: config_packet,
            sequence,
            sent_at: Instant::now(),
            retry_count: 0,
        });
        
        send_packet_type_registry(&self.socket, self.relay_addr, assigned_id)?;

        self.connected_clients.insert(assigned_id, req.desired_name.clone());
        
        if let Some(callback) = &mut self.on_client_connect {
            callback(assigned_id, req.desired_name, req.target_session_id);
        }

        Ok(())
    }
}