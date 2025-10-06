use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::thread::sleep;
use std::time::{Duration, Instant};

use super::socket::NeonSocket;
use super::session::SessionManager;
use super::types::*;

pub struct RelayNode {
    socket: NeonSocket,
    session_manager: SessionManager,
    pending_connections: HashMap<SocketAddr, PendingConnection>,
}

impl RelayNode {
    pub fn new(bind_addr: &str) -> Result<Self, Error> {
        Ok(RelayNode {
            socket: NeonSocket::new(bind_addr)?,
            session_manager: SessionManager::new(),
            pending_connections: HashMap::new(),
        })
    }

    pub fn run(&mut self) -> Result<(), Error> {
        println!("Relay node listening on {}...", "0.0.0.0:7777");
        println!("Protocol Version: 0.2");
        println!();
        
        self.socket.set_nonblocking(true)?;
        
        let mut last_cleanup = Instant::now();
        let cleanup_interval = Duration::from_secs(5);

        loop {
            match self.socket.receive_packet() {
                Ok((packet, addr)) => {
                    self.handle_packet(packet, addr)?;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    // No packets available
                }
                Err(e) => return Err(e),
            }

            if last_cleanup.elapsed() >= cleanup_interval {
                self.session_manager.cleanup_dead_connections();
                last_cleanup = Instant::now();
            }

            sleep(Duration::from_millis(1));
        }
    }

    fn handle_packet(&mut self, packet: NeonPacket, addr: SocketAddr) -> Result<(), Error> {
        if packet.packet_type < 0x10 {
            self.handle_core_packet(packet, addr)
        } else {
            self.handle_game_packet(packet, addr)
        }
    }

    fn handle_core_packet(&mut self, packet: NeonPacket, addr: SocketAddr) -> Result<(), Error> {
        match packet.packet_type {
            x if x == CorePacketType::ConnectRequest as u8 => {
                if let PacketPayload::ConnectRequest(req) = packet.payload {
                    self.handle_connect_request(req, addr)?;
                }
            }
            x if x == CorePacketType::ConnectAccept as u8 => {
                if let PacketPayload::ConnectAccept(accept) = packet.payload.clone() {
                    if let Some(host_addr) = self.session_manager.hosts.get(&accept.session_id) {
                        if addr == *host_addr && packet.client_id != 1 {
                            self.route_connect_accept_to_client(accept, packet.client_id)?;
                            return Ok(());
                        }
                    }

                    if packet.client_id == 1 {
                        self.session_manager.register_host(accept.session_id, addr);
                    } else {
                        self.session_manager.register_client(accept.session_id, packet.client_id, addr);
                    }
                }
            }
            x if x == CorePacketType::ConnectDeny as u8 => {
                if let PacketPayload::ConnectDeny(deny) = packet.payload {
                    self.handle_connect_deny(deny, addr)?;
                }
            }
            _ => {
                self.forward_to_peers(&packet, addr)?;
                if let Some(session_id) = self.session_manager.find_session_for_addr(addr) {
                    self.session_manager.update_client_activity(packet.client_id, session_id);
                }
            }
        }
        Ok(())
    }

    fn handle_game_packet(&mut self, packet: NeonPacket, addr: SocketAddr) -> Result<(), Error> {
        self.forward_to_peers(&packet, addr)?;
        if let Some(session_id) = self.session_manager.find_session_for_addr(addr) {
            self.session_manager.update_client_activity(packet.client_id, session_id);
        }
        Ok(())
    }

    fn handle_connect_request(
        &mut self,
        req: ConnectRequest,
        client_addr: SocketAddr,
    ) -> Result<(), Error> {
        let target_session = req.target_session_id;

        println!(
            "[Relay] Client '{}' from {} requesting to join session {}",
            req.desired_name, client_addr, target_session
        );
        
        if let Some(game_id) = req.game_identifier {
            println!("[Relay]   Game ID: 0x{:08X}", game_id);
        }

        if let Some(host_addr) = self.session_manager.hosts.get(&target_session) {
            println!(
                "[Relay] Forwarding connection request to host at {}",
                host_addr
            );

            self.pending_connections.insert(
                client_addr,
                PendingConnection {
                    client_addr,
                    session_id: target_session,
                    client_name: req.desired_name.clone(),
                },
            );

            let forward_packet = NeonPacket {
                packet_type: CorePacketType::ConnectRequest as u8,
                sequence: 1,
                client_id: 0,
                destination_id: 1,
                payload: PacketPayload::ConnectRequest(req.clone()),
            };

            self.socket.send_packet(&forward_packet, *host_addr)?;
        } else {
            println!(
                "[Relay] Session {} not found (no host registered)",
                target_session
            );
        }

        Ok(())
    }

    fn handle_connect_deny(
        &mut self,
        deny: ConnectDeny,
        host_addr: SocketAddr,
    ) -> Result<(), Error> {
        let mut client_addr_to_send = None;
        
        for (session_id, host) in &self.session_manager.hosts {
            if *host == host_addr {
                for (addr, pending) in &self.pending_connections {
                    if pending.session_id == *session_id {
                        client_addr_to_send = Some(*addr);
                        break;
                    }
                }
                break;
            }
        }
        
        if let Some(client_addr) = client_addr_to_send {
            println!(
                "[Relay] Routing ConnectDeny back to {}",
                client_addr
            );
            
            let deny_packet = NeonPacket {
                packet_type: CorePacketType::ConnectDeny as u8,
                sequence: 1,
                client_id: 0,
                destination_id: 0,
                payload: PacketPayload::ConnectDeny(deny),
            };
            
            self.socket.send_packet(&deny_packet, client_addr)?;
            self.pending_connections.remove(&client_addr);
        } else {
            println!("[Relay] No pending connection found for ConnectDeny");
        }
        
        Ok(())
    }

    fn route_connect_accept_to_client(
        &mut self,
        accept: ConnectAccept,
        client_id: u8,
    ) -> Result<(), Error> {
        let mut client_addr_to_send = None;

        for (addr, pending) in &self.pending_connections {
            if pending.session_id == accept.session_id {
                client_addr_to_send = Some(*addr);
                break;
            }
        }

        if let Some(client_addr) = client_addr_to_send {
            println!(
                "[Relay] Routing ConnectAccept for client {} back to {}",
                client_id, client_addr
            );

            let response_packet = NeonPacket {
                packet_type: CorePacketType::ConnectAccept as u8,
                sequence: 1,
                client_id,
                destination_id: client_id,
                payload: PacketPayload::ConnectAccept(accept),
            };

            self.socket.send_packet(&response_packet, client_addr)?;
            self.pending_connections.remove(&client_addr);
        } else {
            println!("[Relay] No pending connection found for ConnectAccept");
        }

        Ok(())
    }

    fn forward_to_peers(&self, packet: &NeonPacket, sender_addr: SocketAddr) -> Result<(), Error> {
        for (_session_id, peers) in &self.session_manager.sessions {
            if let Some(_sender) = peers.iter().find(|p| p.addr == sender_addr) {
                if let Some(dest_peer) = peers.iter().find(|p| p.client_id == packet.destination_id) {
                    if dest_peer.addr != sender_addr {
                        match self.socket.send_packet(packet, dest_peer.addr) {
                            Ok(()) => {
                                // Successfully forwarded
                            }
                            Err(e) => {
                                println!(
                                    "[Relay] Failed to forward packet from {} to client {} at {}: {}",
                                    sender_addr, packet.destination_id, dest_peer.addr, e
                                );
                            }
                        }
                    }
                } else {
                    println!(
                        "[Relay] Destination client {} not found in session, dropping packet from {}",
                        packet.destination_id, sender_addr
                    );
                    println!("{:?}", packet)
                }
                
                return Ok(());
            }
        }

        println!("[Relay] Unknown sender: {}, dropping packet", sender_addr);
        Ok(())
    }

    pub fn session_count(&self) -> usize {
        self.session_manager.sessions.len()
    }

    pub fn total_client_count(&self) -> usize {
        self.session_manager.sessions.values()
            .map(|peers| peers.len())
            .sum()
    }
}