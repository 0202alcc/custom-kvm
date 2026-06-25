use std::net::UdpSocket;
use custom_kvm::config::ClientConfig;

fn main() {
    let config = ClientConfig::load("kvm-client.toml").unwrap_or_else(|_| {
        eprintln!("Warning: Could not load kvm-client.toml, using defaults");
        ClientConfig::default()
    });

    let server_addr = &config.server_addr;
    println!("Sending diagnostic ping to: {}", server_addr);

    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind local socket");
    let msg = "KVM_DIAG_PING";
    
    match socket.send_to(msg.as_bytes(), server_addr) {
        Ok(bytes) => println!("Successfully sent {} bytes to {}", bytes, server_addr),
        Err(e) => eprintln!("Failed to send packet: {}", e),
    }
}
