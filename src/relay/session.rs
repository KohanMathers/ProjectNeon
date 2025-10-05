use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use super::types::PeerInfo;

pub struct SessionManager {
    pub sessions: HashMap<u32, Vec<PeerInfo>>,
    pub hosts: HashMap<u32, SocketAddr>,
}

impl SessionManager {
    pub fn new() -> Self {
        SessionManager {
            sessions: HashMap::new(),
            hosts: HashMap::new(),
        }
    }

    pub fn cleanup_dead_connections(&mut self) {
        let timeout = Duration::from_secs(15);
        let now = Instant::now();

        let mut sessions_to_remove: Vec<u32> = Vec::new();

        for (session_id, peers) in &mut self.sessions {
            peers.retain(|peer| {
                if !peer.is_host {
                    let is_alive = now.duration_since(peer.last_seen) < timeout;
                    if !is_alive {
                        println!(
                            "[Relay] Client {} in session {} timed out",
                            peer.client_id, session_id
                        );
                    }
                    is_alive
                } else {
                    true
                }
            });

            if peers.is_empty() {
                sessions_to_remove.push(*session_id);
            }
        }

        for session_id in sessions_to_remove {
            self.sessions.remove(&session_id);
            self.hosts.remove(&session_id);
            println!("[Relay] Removed empty session {}", session_id)
        }
    }

    pub fn update_client_activity(&mut self, client_id: u8, session_id: u32) {
        if let Some(peers) = self.sessions.get_mut(&session_id) {
            for peer in peers.iter_mut() {
                if peer.client_id == client_id {
                    peer.last_seen = Instant::now();
                    break;
                }
            }
        }
    }

    pub fn find_session_for_addr(&self, addr: SocketAddr) -> Option<u32> {
        for (session_id, peers) in &self.sessions {
            if peers.iter().any(|p| p.addr == addr) {
                return Some(*session_id);
            }
        }
        None
    }

    pub fn register_host(&mut self, session_id: u32, addr: SocketAddr) {
        self.hosts.insert(session_id, addr);

        let peer = PeerInfo {
            addr,
            client_id: 1,
            session_id,
            is_host: true,
            last_seen: Instant::now(),
        };

        self.sessions
            .entry(session_id)
            .or_insert_with(Vec::new)
            .retain(|p| p.client_id != 1);
        self.sessions.get_mut(&session_id).unwrap().push(peer);

        println!(
            "[Relay] Host registered for session {} at {}",
            session_id, addr
        );
        self.print_active_sessions();
    }

    pub fn register_client(&mut self, session_id: u32, client_id: u8, addr: SocketAddr) {
        let peer = PeerInfo {
            addr,
            client_id,
            session_id,
            is_host: false,
            last_seen: Instant::now(),
        };

        self.sessions
            .entry(session_id)
            .or_insert_with(Vec::new)
            .retain(|p| p.client_id != client_id);
        self.sessions.get_mut(&session_id).unwrap().push(peer);

        println!(
            "[Relay] Client {} registered to session {} from {}",
            client_id, session_id, addr
        );
        self.print_session_info(session_id);
    }

    pub fn print_active_sessions(&self) {
        println!("\n=== Active Sessions ===");
        if self.sessions.is_empty() {
            println!("No active sessions");
        } else {
            for (session_id, peers) in &self.sessions {
                let host_count = peers.iter().filter(|p| p.is_host).count();
                let client_count = peers.iter().filter(|p| !p.is_host).count();
                println!(
                    "Session {}: {} host(s), {} client(s)",
                    session_id, host_count, client_count
                );
            }
        }
        println!("========================\n");
    }

    fn print_session_info(&self, session_id: u32) {
        if let Some(peers) = self.sessions.get(&session_id) {
            let clients: Vec<_> = peers.iter().filter(|p| !p.is_host).collect();
            println!(
                "  Session {} now has {} client(s) connected",
                session_id,
                clients.len()
            );
        }
    }
}