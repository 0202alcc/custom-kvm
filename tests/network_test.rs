use custom_kvm::KvmEvent;
use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

#[test]
fn test_kvm_event_serialization_network() {
    // 1. Server Setup: Bind to a random port
    let server_socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind server socket");
    server_socket.set_read_timeout(Some(Duration::from_secs(2))).expect("Failed to set timeout");
    let server_addr = server_socket.local_addr().expect("Failed to get server address");

    // 2. Client Setup: Separate socket
    let client_socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind client socket");

    // Server receiving thread
    let server_addr_clone = server_addr;
    let handle = thread::spawn(move || {
        let mut buf = [0u8; 1024];
        
        // Sequence 1: Verify Heartbeat
        let (amt, src) = server_socket.recv_from(&mut buf).expect("Server failed to receive heartbeat");
        let event: KvmEvent = bincode::deserialize(&buf[..amt]).expect("Server failed to deserialize heartbeat");
        assert_eq!(event, KvmEvent::Heartbeat);

        // Sequence 2: Verify MouseMove
        let (amt, _) = server_socket.recv_from(&mut buf).expect("Server failed to receive mouse move");
        let event: KvmEvent = bincode::deserialize(&buf[..amt]).expect("Server failed to deserialize mouse move");
        assert_eq!(event, KvmEvent::MouseMove { dx: 10, dy: -5 });
        
        src
    });

    // 3. Packet Sequence
    // Client sends Heartbeat
    let hb = KvmEvent::Heartbeat;
    let hb_encoded = bincode::serialize(&hb).expect("Failed to serialize heartbeat");
    client_socket.send_to(&hb_encoded, server_addr).expect("Failed to send heartbeat");

    // Client sends MouseMove
    let mm = KvmEvent::MouseMove { dx: 10, dy: -5 };
    let mm_encoded = bincode::serialize(&mm).expect("Failed to serialize mouse move");
    client_socket.send_to(&mm_encoded, server_addr).expect("Failed to send mouse move");

    let _client_addr = handle.join().expect("Server thread panicked");
}
