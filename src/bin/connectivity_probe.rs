use std::net::UdpSocket;
use std::env;
use custom_kvm::KvmEvent;
use bincode;

fn main() {
    let server_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:49152".to_string());
    
    println!("Sending Heartbeat probe to: {}", server_addr);

    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind local socket");
    
    let event = KvmEvent::Heartbeat;
    let encoded = bincode::serialize(&event).expect("Failed to serialize Heartbeat");
    
    match socket.send_to(&encoded, &server_addr) {
        Ok(bytes) => println!("Successfully sent {} bytes (Heartbeat) to {}", bytes, server_addr),
        Err(e) => eprintln!("Failed to send packet: {}", e),
    }
}
