use project_neon::relay::NeonRelay;

fn main() {
    println!("Project Neon Protocol v0.2 - Relay");
    println!("===================================");
    println!("Starting relay node...");
    println!();

    let mut relay = match NeonRelay::new("0.0.0.0:7777") {
        Ok(relay) => relay,
        Err(e) => {
            println!("Failed to start relay: {}", e);
            return;
        }
    };

    if let Err(e) = relay.start() {
        println!("Relay failed: {}", e);
    }
}