use std::io::{stdin, stdout, Write};
use project_neon::client::NeonClient;

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
    let relay_addr = if relay_input.is_empty() {
        "127.0.0.1:7777"
    } else {
        &relay_input
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

    match client.connect(target_session_id, relay_addr) {
        Ok(()) => {
            println!("Connection successful! Starting main loop...");
            println!("Press Ctrl+C to disconnect.\n");
        }
        Err(e) => {
            println!("Failed to connect to session: {}", e);
            return;
        }
    }

    if let Err(e) = client.run() {
        println!("Client error: {}", e);
    }
}