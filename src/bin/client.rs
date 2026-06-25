// Platform-gated client implementation: macOS uses CoreGraphics for injection; other OSes run a no-op listener.

use std::net::UdpSocket;
use std::process::{Command, Stdio};
use std::sync::Arc;
#[allow(unused_imports)]
use custom_kvm::{KvmEvent, DisplayInfo, DisplayOrientation};

#[cfg(target_os = "macos")]
use core_graphics::event::{CGEvent, CGMouseButton, CGEventType};
#[cfg(target_os = "macos")]
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

/// Kill any existing kvm-client processes to free up the port
#[cfg(target_os = "macos")]
fn cleanup_existing_clients() {
    // Use pkill to find and kill existing kvm-client processes (excluding this one)
    // The -f flag searches the full command line, and "not $$" would exclude current process
    // But simpler approach: just kill any kvm-client that isn't us
    let _ = Command::new("pkill")
        .args(&["-f", "kvm-client"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output();

    // Give the OS a moment to close the socket
    std::thread::sleep(std::time::Duration::from_millis(100));
}

#[cfg(not(target_os = "macos"))]
fn cleanup_existing_clients() {
    // Non-macOS: use killall if available
    let _ = Command::new("killall")
        .arg("kvm-client")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output();

    std::thread::sleep(std::time::Duration::from_millis(100));
}

fn send_log(socket: &UdpSocket, addr: &str, message: &str) {
    let log_event = KvmEvent::LogMessage {
        message: message.to_string(),
    };
    if let Ok(serialized) = bincode::serialize(&log_event) {
        let _ = socket.send_to(&serialized, addr);
    }
}

fn get_macos_displays(socket: &UdpSocket, addr: &str) -> Vec<DisplayInfo> {
    send_log(socket, addr, "CLIENT_DEBUG: Entering get_macos_displays");
    
    // 1. Get actual screen frames using JXA (JavaScript for Automation)
    send_log(socket, addr, "CLIENT_DEBUG: Executing osascript for screen frames...");
    let jxa_script = r#"
        var screens = $.NSScreen.screens;
        screens.map(s => {
            var frame = s.frame();
            return JSON.stringify({
                x: frame.origin.x,
                y: frame.origin.y,
                width: frame.size.width,
                height: frame.size.height
            });
        }).join('\n');
    "#;

    let frames_output = std::process::Command::new("osascript")
        .args(&["-l", "JavaScript", "-e", jxa_script])
        .output();

    let mut screen_frames = Vec::new();
    if let Ok(out) = frames_output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            if let Ok(frame) = serde_json::from_str::<serde_json::Value>(line) {
                screen_frames.push((
                    frame["x"].as_f64().unwrap_or(0.0) as i32,
                    frame["y"].as_f64().unwrap_or(0.0) as i32,
                    frame["width"].as_f64().unwrap_or(0.0) as i32,
                    frame["height"].as_f64().unwrap_or(0.0) as i32,
                ));
            }
        }
    }

    // 2. Get rotations from system_profiler
    send_log(socket, addr, "CLIENT_DEBUG: Executing system_profiler SPDisplaysDataType for rotations...");
    let profiler_output = std::process::Command::new("system_profiler")
        .arg("SPDisplaysDataType")
        .output();

    let mut displays = Vec::new();
    if let Ok(out) = profiler_output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let lines: Vec<&str> = stdout.lines().collect();
        
        let mut display_idx = 0;
        let mut current_orientation = DisplayOrientation::Normal;

        for line in lines {
            if line.contains("Resolution") {
                // We found a display. Match it with the frame from JXA if available.
                let (x, y, width, height) = if display_idx < screen_frames.len() {
                    screen_frames[display_idx]
                } else {
                    (0, 0, 0, 0)
                };

                displays.push(DisplayInfo {
                    id: display_idx as u32,
                    orientation: current_orientation,
                    x,
                    y,
                    width,
                    height,
                });
                
                display_idx += 1;
                current_orientation = DisplayOrientation::Normal;
            } else if line.contains("Rotation") {
                if line.contains("90") {
                    current_orientation = DisplayOrientation::Left;
                } else if line.contains("270") {
                    current_orientation = DisplayOrientation::Right;
                } else if line.contains("180") {
                    current_orientation = DisplayOrientation::Inverted;
                }
            }
        }
    }

    send_log(socket, addr, &format!("CLIENT_DEBUG: Detected {} displays on macOS with absolute coordinates", displays.len()));
    displays
}

#[cfg(target_os = "macos")]
fn run_client(config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Kill any existing kvm-client processes to avoid "address already in use" errors
    cleanup_existing_clients();

    // Load configuration
    let config = custom_kvm::config::ClientConfig::load(config_path)
        .unwrap_or_else(|_| custom_kvm::config::ClientConfig::default());

    // Initialize logging
    custom_kvm::logging::init_logging(&config.log_level)?;

    log::info!("KVM Client (macOS) starting");
    log::debug!("Binding to: {}", config.bind_addr);
    log::debug!("Server address: {}", config.server_addr);

    // Bind client to its local port to listen
    let socket = UdpSocket::bind(&config.bind_addr)?;
    let socket_arc = Arc::new(socket);
    let mut buf = [0u8; 1024];

    log::info!("KVM Client (macOS) listening for inputs from remote desktop...");

    // Start heartbeat thread
    let heartbeat_socket = Arc::clone(&socket_arc);
    let server_addr = config.server_addr.clone();
    std::thread::spawn(move || {
        let mut heartbeat_count: u32 = 0;
        
        loop {
            // Report display configuration on startup (first 5 heartbeats) and every 30 heartbeats (60s)
            if heartbeat_count < 5 || heartbeat_count % 30 == 0 {
                send_log(&heartbeat_socket, &server_addr, &format!("CLIENT_DEBUG: Attempting to send display report (count: {})", heartbeat_count));
                let displays = get_macos_displays(&heartbeat_socket, &server_addr);
                send_log(&heartbeat_socket, &server_addr, &format!("CLIENT_DEBUG: Got {} displays, preparing report", displays.len()));
                
                let report = KvmEvent::DisplayReport { displays };
                match bincode::serialize(&report) {
                    Ok(serialized) => {
                        send_log(&heartbeat_socket, &server_addr, &format!("CLIENT_DEBUG: Serialized report size: {} bytes", serialized.len()));
                        if let Err(e) = heartbeat_socket.send_to(&serialized, &server_addr) {
                            send_log(&heartbeat_socket, &server_addr, &format!("CLIENT_DEBUG: send_to failed: {}", e));
                        } else {
                            send_log(&heartbeat_socket, &server_addr, "CLIENT_DEBUG: send_to successful");
                        }
                    }
                    Err(e) => {
                        send_log(&heartbeat_socket, &server_addr, &format!("CLIENT_DEBUG: Serialization failed: {}", e));
                    }
                }
            }

            let heartbeat = KvmEvent::Heartbeat;
            if let Ok(serialized) = bincode::serialize(&heartbeat) {
                log::debug!("Sending heartbeat to {}", server_addr);
                if let Err(e) = heartbeat_socket.send_to(&serialized, &server_addr) {
                    log::error!("Failed to send heartbeat to server {}: {}", server_addr, e);
                }
            }
            
            heartbeat_count += 1;
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    });

    // Track pressed mouse buttons to send Drag events when appropriate
    let mut button_state = [false; 3]; // left, right, middle

    loop {
        let (amt, src) = socket_arc.recv_from(&mut buf)?;
        log::debug!("Received {} bytes from {}", amt, src);

        match bincode::deserialize::<KvmEvent>(&buf[..amt]) {
            Ok(decoded) => {
                log::debug!("Decoded event: {:?}", decoded);
                match decoded {
                    KvmEvent::MouseMove { dx, dy } => {
                        log::debug!("Processing MouseMove: dx={}, dy={}", dx, dy);
                        // Use CoreGraphics to move mouse relatively
                        match CGEventSource::new(CGEventSourceStateID::CombinedSessionState) {
                            Ok(source) => {
                                match CGEvent::new(source.clone()) {
                                    Ok(current_event) => {
                                        let mut point = current_event.location();
                                        point.x += dx as f64;
                                        point.y += dy as f64;

                                        // Choose dragged event type if a button is held
                                        let evt_type = if button_state[0] {
                                            CGEventType::LeftMouseDragged
                                        } else if button_state[1] {
                                            CGEventType::RightMouseDragged
                                        } else if button_state[2] {
                                            CGEventType::OtherMouseDragged
                                        } else {
                                            CGEventType::MouseMoved
                                        };

                                        match CGEvent::new_mouse_event(source, evt_type, point, CGMouseButton::Left) {
                                            Ok(move_event) => {
                                                move_event.post(core_graphics::event::CGEventTapLocation::HID);
                                                log::debug!("Mouse moved relatively to: {}, {}", point.x, point.y);
                                            }
                                            Err(_) => log::error!("Failed to create mouse move event"),
                                        }
                                    }
                                    Err(_) => log::error!("Failed to get current mouse event"),
                                }
                            }
                            Err(_) => log::error!("Failed to create event source"),
                        }
                    }
                    KvmEvent::MouseAbsMove { x, y } => {
                        log::debug!("Processing MouseAbsMove: x={}, y={}", x, y);
                        match CGEventSource::new(CGEventSourceStateID::CombinedSessionState) {
                            Ok(source) => {
                                let point = core_graphics::geometry::CGPoint::new(x as f64, y as f64);

                                // Choose dragged event type if a button is held
                                let evt_type = if button_state[0] {
                                    CGEventType::LeftMouseDragged
                                } else if button_state[1] {
                                    CGEventType::RightMouseDragged
                                } else if button_state[2] {
                                    CGEventType::OtherMouseDragged
                                } else {
                                    CGEventType::MouseMoved
                                };

                                match CGEvent::new_mouse_event(source, evt_type, point, CGMouseButton::Left) {
                                    Ok(move_event) => {
                                        move_event.post(core_graphics::event::CGEventTapLocation::HID);
                                        log::info!("Moved cursor to: {}, {}", x, y);
                                    }
                                    Err(_) => log::error!("Failed to create mouse move event"),
                                }
                            }
                            Err(_) => log::error!("Failed to create event source"),
                        }
                    }
                    KvmEvent::MouseButton { button, is_down } => {
                        log::info!("MouseButton event received: button={}, is_down={}", button, is_down);

                        // Update local button state so subsequent MouseMove events can be treated as drags
                        if (button as usize) < button_state.len() {
                            button_state[button as usize] = is_down;
                        }

                        match CGEventSource::new(CGEventSourceStateID::CombinedSessionState) {
                            Ok(source) => {
                                match CGEvent::new(source.clone()) {
                                    Ok(current_event) => {
                                        let point = current_event.location();

                                        let (mouse_button, event_type) = match button {
                                            0 => {
                                                // Left mouse button
                                                if is_down {
                                                    (CGMouseButton::Left, CGEventType::LeftMouseDown)
                                                } else {
                                                    (CGMouseButton::Left, CGEventType::LeftMouseUp)
                                                }
                                            }
                                            1 => {
                                                // Right mouse button
                                                if is_down {
                                                    (CGMouseButton::Right, CGEventType::RightMouseDown)
                                                } else {
                                                    (CGMouseButton::Right, CGEventType::RightMouseUp)
                                                }
                                            }
                                            2 => {
                                                // Middle mouse button
                                                if is_down {
                                                    (CGMouseButton::Center, CGEventType::OtherMouseDown)
                                                } else {
                                                    (CGMouseButton::Center, CGEventType::OtherMouseUp)
                                                }
                                            }
                                            _ => {
                                                log::warn!("Unknown mouse button: {}", button);
                                                return Ok(());
                                            }
                                        };

                                        match CGEvent::new_mouse_event(source, event_type, point, mouse_button) {
                                            Ok(button_event) => {
                                                button_event.post(core_graphics::event::CGEventTapLocation::HID);
                                                log::info!("Posted mouse button event: button={}, is_down={}", button, is_down);
                                            }
                                            Err(_) => log::error!("Failed to create mouse button event"),
                                        }
                                    }
                                    Err(_) => log::error!("Failed to get current mouse event"),
                                }
                            }
                            Err(_) => log::error!("Failed to create event source"),
                        }
                    }
                    KvmEvent::Key { keycode, is_down } => {
                        log::debug!("Received keyboard event: keycode={}, is_down={}", keycode, is_down);

                        match custom_kvm::keycodes::linux_to_macos_keycode(keycode) {
                            Some(mac_keycode) => {
                                match CGEventSource::new(CGEventSourceStateID::CombinedSessionState) {
                                    Ok(source) => {
                                        // Convert u32 to u16 for CGEvent::new_keyboard_event
                                        let keycode_u16 = if mac_keycode <= u16::MAX as u32 {
                                            mac_keycode as u16
                                        } else {
                                            log::warn!("Keycode {} out of u16 range", mac_keycode);
                                            return Ok(());
                                        };

                                        match CGEvent::new_keyboard_event(source, keycode_u16, is_down) {
                                            Ok(key_event) => {
                                                key_event.post(core_graphics::event::CGEventTapLocation::HID);
                                                log::info!("Posted keyboard event: keycode={}, mac_keycode={}, is_down={}", 
                                                    keycode, mac_keycode, is_down);
                                            }
                                            Err(_) => log::error!("Failed to create keyboard event"),
                                        }
                                    }
                                    Err(_) => log::error!("Failed to create event source"),
                                }
                            }
                            None => {
                                log::warn!("Unknown Linux keycode: {}", keycode);
                            }
                        }
                    }
                    KvmEvent::MouseScroll { delta } => {
                        log::debug!("MouseScroll event received: delta={}", delta);
                        // Scroll support not yet implemented
                    }
                    KvmEvent::Heartbeat | KvmEvent::DisplayReport { .. } | KvmEvent::LogMessage { .. } => {}
                }
            }
            Err(e) => {
                log::error!("Failed to deserialize event: {}", e);
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn run_client(config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Kill any existing kvm-client processes to avoid "address already in use" errors
    cleanup_existing_clients();

    log::info!("KVM Client (non-macOS) - running in no-op listener mode");
    log::info!("This client only supports macOS for input injection");

    // Load configuration
    let config = custom_kvm::config::ClientConfig::load(config_path)
        .unwrap_or_else(|_| custom_kvm::config::ClientConfig::default());

    // Initialize logging
    custom_kvm::logging::init_logging(&config.log_level)?;

    log::info!("Binding to: {}", config.bind_addr);

    let socket = UdpSocket::bind(&config.bind_addr)?;
    let mut buf = [0u8; 1024];

    log::info!("Listening on {} (events received but not injected - macOS only)", config.bind_addr);

    loop {
        match socket.recv_from(&mut buf) {
            Ok((amt, src)) => {
                log::debug!("Received {} bytes from {} (not processing on non-macOS)", amt, src);
            }
            Err(e) => {
                log::error!("Socket error: {}", e);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let config_path = if args.len() > 2 && args[1] == "--config" {
        &args[2]
    } else {
        "kvm-client.toml"
    };

    run_client(config_path)
}
