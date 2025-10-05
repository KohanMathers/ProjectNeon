pub mod types;
mod socket;
mod session;
mod relay;

use std::io::Error;
pub use relay::RelayNode;
pub use types::{NeonPacket, PacketPayload};

pub struct NeonRelay {
    relay: RelayNode,
}

impl NeonRelay {
    /// Create a new relay server bound to the specified address
    pub fn new(bind_addr: &str) -> Result<Self, Error> {
        Ok(Self {
            relay: RelayNode::new(bind_addr)?,
        })
    }

    /// Get the number of active sessions
    pub fn session_count(&self) -> usize {
        self.relay.session_count()
    }

    /// Get the total number of connected clients across all sessions
    pub fn total_client_count(&self) -> usize {
        self.relay.total_client_count()
    }

    /// Start the relay server (blocks)
    pub fn start(&mut self) -> Result<(), Error> {
        self.relay.run()
    }
}