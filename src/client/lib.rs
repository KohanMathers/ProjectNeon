pub mod types;
mod incoming;
mod outgoing;

use std::net::SocketAddr;
use std::io::{Error, ErrorKind};
use std::time::{Instant, Duration};
use std::thread::sleep;

pub use types::{PacketPayload, NeonPacket};
use incoming::{NeonSocket, process_incoming_packets};
use outgoing::*;

pub type PongCallback = Box<dyn FnMut(u64, u64) + Send>; // (response_time_ms, timestamp)
pub type SessionConfigCallback = Box<dyn FnMut(u8, u16, u16) + Send>; // (version, tick_rate, max_packet_size)
pub type PacketTypeRegistryCallback = Box<dyn FnMut(Vec<(u8, String, String)>) + Send>; // Vec<(id, name, description)>
pub type UnhandledPacketCallback = Box<dyn FnMut(u8, u8) + Send>; // (packet_type, from_client_id)
pub type WrongDestinationCallback = Box<dyn FnMut(u8, u8) + Send>; // (my_id, packet_destination_id)

pub struct NeonClient {
    socket: NeonSocket,
    relay_addr: Option<SocketAddr>,
    client_id: Option<u8>,
    session_id: Option<u32>,
    name: String,
    auto_ping: bool,
    ping_interval: Duration,
    last_ping: Option<Instant>,
    
    on_pong: Option<PongCallback>,
    on_session_config: Option<SessionConfigCallback>,
    on_packet_type_registry: Option<PacketTypeRegistryCallback>,
    on_unhandled_packet: Option<UnhandledPacketCallback>,
    on_wrong_destination: Option<WrongDestinationCallback>,
}

impl NeonClient {
    /// Create a new client with a name
    pub fn new(name: String) -> Result<Self, Error> {
        Ok(Self {
            socket: NeonSocket::new("0.0.0.0:0")?,
            relay_addr: None,
            client_id: None,
            session_id: None,
            name,
            auto_ping: true,
            ping_interval: Duration::from_secs(5),
            last_ping: None,
            on_pong: None,
            on_session_config: None,
            on_packet_type_registry: None,
            on_unhandled_packet: None,
            on_wrong_destination: None,
        })
    }

    /// Set callback for when a pong is received
    pub fn on_pong<F>(&mut self, callback: F)
    where
        F: FnMut(u64, u64) + Send + 'static,
    {
        self.on_pong = Some(Box::new(callback));
    }

    /// Set callback for when session config is received
    pub fn on_session_config<F>(&mut self, callback: F)
    where
        F: FnMut(u8, u16, u16) + Send + 'static,
    {
        self.on_session_config = Some(Box::new(callback));
    }

    /// Set callback for when packet type registry is received
    pub fn on_packet_type_registry<F>(&mut self, callback: F)
    where
        F: FnMut(Vec<(u8, String, String)>) + Send + 'static,
    {
        self.on_packet_type_registry = Some(Box::new(callback));
    }

    /// Set callback for unhandled packets
    pub fn on_unhandled_packet<F>(&mut self, callback: F)
    where
        F: FnMut(u8, u8) + Send + 'static,
    {
        self.on_unhandled_packet = Some(Box::new(callback));
    }

    /// Set callback for packets sent to wrong destination
    pub fn on_wrong_destination<F>(&mut self, callback: F)
    where
        F: FnMut(u8, u8) + Send + 'static,
    {
        self.on_wrong_destination = Some(Box::new(callback));
    }

    /// Set whether to automatically send pings (default: true)
    pub fn set_auto_ping(&mut self, enabled: bool) {
        self.auto_ping = enabled;
    }

    /// Set the ping interval (default: 5 seconds)
    pub fn set_ping_interval(&mut self, interval: Duration) {
        self.ping_interval = interval;
    }

    /// Get the client's assigned ID (None if not connected)
    pub fn client_id(&self) -> Option<u8> {
        self.client_id
    }

    /// Get the session ID (None if not connected)
    pub fn session_id(&self) -> Option<u32> {
        self.session_id
    }

    /// Get the client name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Connect to a session
    pub fn connect(&mut self, session_id: u32, relay_addr: &str) -> Result<(), Error> {
        let relay_addr = relay_addr.parse()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid relay address"))?;
        
        self.relay_addr = Some(relay_addr);
        self.socket.socket.set_nonblocking(false)?;

        send_connect_request(&self.socket, relay_addr, &self.name, session_id)?;

        let (assigned_client_id, received_session_id) = wait_for_connect_response(&self.socket, Duration::from_secs(10))?;
        
        self.socket.socket.set_nonblocking(true)?;
        
        if received_session_id != session_id {
            return Err(Error::new(ErrorKind::ConnectionRefused, 
                format!("Session ID mismatch: requested {}, got {}", session_id, received_session_id)));
        }
        
        self.client_id = Some(assigned_client_id);
        self.session_id = Some(received_session_id);

        let accept = types::ConnectAccept {
            assigned_client_id,
            session_id: received_session_id,
        };
        send_connect_accept_confirmation(&self.socket, relay_addr, assigned_client_id, accept)?;

        Ok(())
    }

    /// Manually send a ping
    pub fn send_ping(&self) -> Result<(), Error> {
        if let (Some(relay_addr), Some(client_id)) = (self.relay_addr, self.client_id) {
            send_ping(&self.socket, relay_addr, client_id)
        } else {
            Err(Error::new(ErrorKind::NotConnected, "Client not connected"))
        }
    }

    /// Process incoming packets once
    pub fn process_packets(&mut self) -> Result<(), Error> {
        if let Some(client_id) = self.client_id {
            if self.auto_ping {
                let should_ping = self.last_ping
                    .map(|t| t.elapsed() >= self.ping_interval)
                    .unwrap_or(true);
                
                if should_ping {
                    self.send_ping()?;
                    self.last_ping = Some(Instant::now());
                }
            }

            process_incoming_packets(
                &self.socket,
                self.relay_addr.unwrap(),
                client_id,
                &mut self.on_pong,
                &mut self.on_session_config,
                &mut self.on_packet_type_registry,
                &mut self.on_unhandled_packet,
                &mut self.on_wrong_destination,
            )
        } else {
            Err(Error::new(ErrorKind::NotConnected, "Client not connected"))
        }
    }

    /// Run the client in a loop (blocks)
    pub fn run(&mut self) -> Result<(), Error> {
        loop {
            self.process_packets()?;
            sleep(Duration::from_millis(10));
        }
    }
}