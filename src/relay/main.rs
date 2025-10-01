mod types;
mod socket;
mod session;
mod relay;

use relay::RelayNode;

fn main() {
    println!("Project Neon Protocol v0.2 - Relay");
    println!("===================================");
    println!("Starting relay node...");
    println!();

    let mut relay = RelayNode::new("0.0.0.0:7777").expect("Failed to start relay");
    relay.run().expect("Relay failed");
}