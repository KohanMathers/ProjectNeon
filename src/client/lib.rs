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

pub struct NeonClient {
    socket: NeonSocket,
    relay_addr: Option<SocketAddr>,
    client_id: Option<u8>,
    session_id: Option<u32>,
    name: String,
    auto_ping: bool,
    ping_interval: Duration,
    last_ping: Option<Instant>,
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
        })
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

        println!("Successfully connected to session {}! Assigned client ID: {}", received_session_id, assigned_client_id);
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

            process_incoming_packets(&self.socket, client_id)
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