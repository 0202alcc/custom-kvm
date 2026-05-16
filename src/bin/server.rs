use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::panic;
use custom_kvm::KvmEvent;
use evdev::{InputEventKind, RelativeAxisType, EventType};

const LINUX_SCREEN_WIDTH: i32 = 1920;
const LINUX_SCREEN_HEIGHT: i32 = 1080;

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:8080")?;
    let client_address = "0.0.0.0:8080"; 

    let device = match find_mouse_device() {
        Some(dev) => dev,
        None => std::process::exit(1),
    };

    // Wrap the device so it can be shared safely with the crash/signal handlers
    let shared_device = Arc::new(Mutex::new(device));

    // --- 1. SET UP THE CTRL+C SIGNAL HANDLER ---
    let d_ctrlc = Arc::clone(&shared_device);
    ctrlc::set_handler(move || {
        println!("\n🛑 Intercepted exit signal! Safely releasing mouse...");
        if let Ok(mut dev) = d_ctrlc.lock() {
            let _ = dev.ungrab(); // Force ungrab back to OS
        }
        std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");

    // --- 2. SET UP THE PANIC HOOK (FOR CODE CRASHES) ---
    let d_panic = Arc::clone(&shared_device);
    panic::set_hook(Box::new(move |panic_info| {
        eprintln!("💥 Code panicked: {:?}", panic_info);
        eprintln!("Safely releasing mouse before exit...");
        if let Ok(mut dev) = d_panic.lock() {
            let _ = dev.ungrab();
        }
    }));

    // --- KVM STATE VARIABLES ---
    let mut virtual_x = LINUX_SCREEN_WIDTH / 2;
    let mut virtual_y = LINUX_SCREEN_HEIGHT / 2;
    let mut is_controlling_mac = false;

    println!("KVM Edge Detection active. Move mouse past right edge to switch to Mac.");

    loop {
        // Lock the device briefly to fetch the incoming events
        let mut events = {
            let mut dev = shared_device.lock().unwrap();
            dev.fetch_events().unwrap().collect::<Vec<_>>()
        };

        for event in events {
            let mut dx = 0;
            let mut dy = 0;

            match event.kind() {
                InputEventKind::RelAxis(RelativeAxisType::REL_X) => dx = event.value(),
                InputEventKind::RelAxis(RelativeAxisType::REL_Y) => dy = event.value(),
                _ => {}
            }

            if dx == 0 && dy == 0 { continue; }

            virtual_x += dx;
            virtual_y += dy;
            virtual_y = virtual_y.clamp(0, LINUX_SCREEN_HEIGHT);

            if !is_controlling_mac {
                virtual_x = virtual_x.clamp(0, LINUX_SCREEN_WIDTH);

                if virtual_x >= LINUX_SCREEN_WIDTH {
                    println!("🔄 Transitioning control to Mac!");
                    
                    // Lock and Grab
                    shared_device.lock().unwrap().grab().expect("Failed to grab mouse");
                    is_controlling_mac = true;
                    
                    let edge_entry = KvmEvent::MouseAbsMove { x: 0, y: virtual_y };
                    socket.send_to(&bincode::serialize(&edge_entry).unwrap(), client_address)?;
                }
            } else {
                let move_ev = KvmEvent::MouseMove { dx, dy };
                socket.send_to(&bincode::serialize(&move_ev).unwrap(), client_address)?;

                if virtual_x < 0 {
                    println!("🔄 Returning control to Linux!");
                    
                    // Lock and Ungrab
                    shared_device.lock().unwrap().ungrab().expect("Failed to ungrab mouse");
                    is_controlling_mac = false;
                    virtual_x = LINUX_SCREEN_WIDTH - 10;
                }
            }
        }
    }
}

fn find_mouse_device() -> Option<evdev::Device> {
    // evdev::enumerate() automatically crawls /dev/input/
    for (_path, device) in evdev::enumerate() {
        // Check if the device reports Relative movements (like a mouse)
        if let Some(_supported_events) = device.supported_events().into_iter().next() {
            if device.supported_events().contains(EventType::RELATIVE) {
                
                // Verify it specifically has X and Y axes to rule out odd scroll wheels
                if let Some(rel_axes) = device.supported_relative_axes() {
                    if rel_axes.contains(RelativeAxisType::REL_X) && rel_axes.contains(RelativeAxisType::REL_Y) {
                        println!(
                            "🎯 Auto-detected mouse: \"{}\"", 
                            device.name().unwrap_or("Unknown Device")
                        );
                        return Some(device);
                    }
                }
            }
        }
    }
    None
}