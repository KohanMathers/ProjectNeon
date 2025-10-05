use std::env;
use project_neon::host::NeonHost;

fn main() {
    println!("Project Neon Alpha Build 12 - Host");
    
    let args: Vec<String> = env::args().collect();
    let session_id = if args.len() > 1 {
        match args[1].parse::<u32>() {
            Ok(id) => id,
            Err(_) => {
                println!("Invalid session ID argument, using random session ID.");
                rand::random::<u32>()
            }
        }
    } else {
        rand::random::<u32>()
    };

    let relay_addr = "127.0.0.1:7777";

    let mut host = match NeonHost::new(session_id, relay_addr) {
        Ok(host) => host,
        Err(e) => {
            println!("Failed to create host: {}", e);
            return;
        }
    };

    println!("Host will create session ID: {}", host.session_id());
    println!();

    if let Err(e) = host.start() {
        println!("Host failed: {}", e);
    }
}