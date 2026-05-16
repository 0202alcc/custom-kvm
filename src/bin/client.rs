use std::net::UdpSocket;
use custom_kvm::KvmEvent;
use core_graphics::event::{CGEvent, CGMouseButton, CGEventType};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

fn main() -> std::io::Result<()> {
    // Bind client to its local port to listen
    let socket = UdpSocket::bind("0.0.0.0:8080")?;
    let mut buf = [0u8; 1024];

    println!("KVM Client listening for inputs from remote desktop...");

    loop {
        // block waiting for a UDP packet
        let (amt, _src) = socket.recv_from(&mut buf)?;
        
        // Deserialize back into our structural Rust enum
        let decoded: KvmEvent = bincode::deserialize(&buf[..amt]).unwrap();
        
        match decoded {
            KvmEvent::MouseMove { dx, dy } => {
                let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).unwrap();
                let current_event = CGEvent::new(source.clone()).unwrap();
                let mut point = current_event.location();
                
                point.x += dx as f64;
                point.y += dy as f64;
                
                let move_event = CGEvent::new_mouse_event(source, CGEventType::MouseMoved, point, CGMouseButton::Left).unwrap();
                move_event.post(core_graphics::event::CGEventTapLocation::HID);
            },
            KvmEvent::MouseAbsMove { x, y } => {
                // Warp the cursor immediately to a specific coordinate
                let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).unwrap();
                let point = core_graphics::geometry::CGPoint::new(x as f64, y as f64);
                
                let move_event = CGEvent::new_mouse_event(source, CGEventType::MouseMoved, point, CGMouseButton::Left).unwrap();
                move_event.post(core_graphics::event::CGEventTapLocation::HID);
                println!("Moved cursor to: {}, {}", x, y);
            },
            _ => {}
        }
    }
}