// Platform-gated client implementation: macOS uses CoreGraphics for injection; other OSes run a no-op listener.

use std::net::UdpSocket;
use std::process::{Command, Stdio};
#[allow(unused_imports)]
use custom_kvm::KvmEvent;

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

#[cfg(target_os = "macos")]
fn run_client() -> Result<(), Box<dyn std::error::Error>> {
    // Kill any existing kvm-client processes to avoid "address already in use" errors
    cleanup_existing_clients();

    // Initialize logging
    custom_kvm::logging::init_logging("info")?;

    // Load configuration
    let config = custom_kvm::config::ClientConfig::load("kvm-client.toml")
        .unwrap_or_else(|_| custom_kvm::config::ClientConfig::default());

    log::info!("KVM Client (macOS) starting");
    log::debug!("Binding to: {}", config.bind_addr);

    // Bind client to its local port to listen
    let socket = UdpSocket::bind(&config.bind_addr)?;
    let mut buf = [0u8; 1024];

    log::info!("KVM Client (macOS) listening for inputs from remote desktop...");

    // Track pressed mouse buttons to send Drag events when appropriate
    let mut button_state = [false; 3]; // left, right, middle

    loop {
        let (amt, src) = socket.recv_from(&mut buf)?;
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
                }
            }
            Err(e) => {
                log::error!("Failed to deserialize event: {}", e);
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn run_client() -> Result<(), Box<dyn std::error::Error>> {
    // Kill any existing kvm-client processes to avoid "address already in use" errors
    cleanup_existing_clients();

    log::info!("KVM Client (non-macOS) - running in no-op listener mode");
    log::info!("This client only supports macOS for input injection");

    // Initialize logging
    custom_kvm::logging::init_logging("info")?;

    // Load configuration
    let config = custom_kvm::config::ClientConfig::load("kvm-client.toml")
        .unwrap_or_else(|_| custom_kvm::config::ClientConfig::default());

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
    run_client()
}
