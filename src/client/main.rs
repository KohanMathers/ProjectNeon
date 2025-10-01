mod types;
mod incoming;
mod outgoing;

use std::net::SocketAddr;
use std::io::{Error, ErrorKind, stdin, stdout, Write};
use std::time::{Instant, Duration};
use std::thread::sleep;

use types::*;
use incoming::{NeonSocket, process_incoming_packets};
use outgoing::*;

struct NeonClient {
    socket: NeonSocket,
    relay_addr: Option<SocketAddr>,
    client_id: Option<u8>,
    session_id: Option<u32>,
    name: String,
}

impl NeonClient {
    fn new(name: String) -> Result<Self, Error> {
        Ok(Self {
            socket: NeonSocket::new("0.0.0.0:0")?,
            relay_addr: None,
            client_id: None,
            session_id: None,
            name,
        })
    }

    fn connect_to_session(&mut self, relay_addr: SocketAddr, target_session_id: u32) -> Result<(), Error> {
        self.relay_addr = Some(relay_addr);
        self.socket.socket.set_nonblocking(false)?;

        send_connect_request(&self.socket, relay_addr, &self.name, target_session_id)?;

        let (assigned_client_id, session_id) = wait_for_connect_response(&self.socket, Duration::from_secs(10))?;
        
        self.socket.socket.set_nonblocking(true)?;
        
        if session_id != target_session_id {
            return Err(Error::new(ErrorKind::ConnectionRefused, 
                format!("Session ID mismatch: requested {}, got {}", target_session_id, session_id)));
        }
        
        self.client_id = Some(assigned_client_id);
        self.session_id = Some(session_id);

        let accept = ConnectAccept {
            assigned_client_id,
            session_id,
        };
        send_connect_accept_confirmation(&self.socket, relay_addr, assigned_client_id, accept)?;

        println!("Successfully connected to session {}! Assigned client ID: {}", session_id, assigned_client_id);
        Ok(())
    }

    fn send_ping(&self) -> Result<(), Error> {
        if let (Some(relay_addr), Some(client_id)) = (self.relay_addr, self.client_id) {
            send_ping(&self.socket, relay_addr, client_id)
        } else {
            Err(Error::new(ErrorKind::NotConnected, "Client not initialized"))
        }
    }

    fn process_incoming_packets(&self) -> Result<(), Error> {
        if let Some(client_id) = self.client_id {
            process_incoming_packets(&self.socket, client_id)
        } else {
            Err(Error::new(ErrorKind::NotConnected, "Client not initialized"))
        }
    }
}

fn get_user_input(prompt: &str) -> String {
    print!("{}", prompt);
    stdout().flush().unwrap();
    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn main() {
    println!("Project Neon Alpha Build 12 - Client");
    println!("==============================================");
    
    let client_name = get_user_input("Enter your client name: ");
    if client_name.is_empty() {
        println!("Client name cannot be empty!");
        return;
    }
    
    let session_input = get_user_input("Enter session ID to connect to: ");
    let target_session_id: u32 = match session_input.parse() {
        Ok(id) => id,
        Err(_) => {
            println!("Invalid session ID! Please enter a valid number.");
            return;
        }
    };
    
    let relay_input = get_user_input("Enter relay address (default: 127.0.0.1:7777): ");
    let relay_addr_str = if relay_input.is_empty() {
        "127.0.0.1:7777"
    } else {
        &relay_input
    };
    
    let relay_addr: SocketAddr = match relay_addr_str.parse() {
        Ok(addr) => addr,
        Err(_) => {
            println!("Invalid relay address format!");
            return;
        }
    };

    println!("\nAttempting connection...");
    println!("Client Name: {}", client_name);
    println!("Target Session ID: {}", target_session_id);
    println!("Relay Address: {}", relay_addr);
    println!();

    let mut client = match NeonClient::new(client_name) {
        Ok(client) => client,
        Err(e) => {
            println!("Failed to create client: {}", e);
            return;
        }
    };

    match client.connect_to_session(relay_addr, target_session_id) {
        Ok(()) => {
            println!("Connection successful! Starting ping loop...");
            println!("Press Ctrl+C to disconnect.\n");
        }
        Err(e) => {
            println!("Failed to connect to session: {}", e);
            return;
        }
    }

    let ping_interval = Duration::from_secs(5);
    let mut last_ping = Instant::now() - ping_interval;

    loop {
        if last_ping.elapsed() >= ping_interval {
            if let Err(e) = client.send_ping() {
                println!("Failed to send ping: {}", e);
                break;
            }
            last_ping = Instant::now();
        }

        if let Err(e) = client.process_incoming_packets() {
            println!("Error processing packets: {}", e);
            break;
        }

        sleep(Duration::from_millis(10));
    }
}