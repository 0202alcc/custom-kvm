// Platform-gated client implementation: macOS uses CoreGraphics for injection; other OSes run a no-op listener.

use std::net::UdpSocket;
use custom_kvm::KvmEvent;

#[cfg(target_os = "macos")]
use core_graphics::event::{CGEvent, CGMouseButton, CGEventType};
#[cfg(target_os = "macos")]
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

#[cfg(target_os = "macos")]
fn run_client() -> Result<(), Box<dyn std::error::Error>> {
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
                        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
                            .ok_or("Failed to create event source")?;
                        let current_event = CGEvent::new(source.clone())
                            .ok_or("Failed to get current mouse event")?;
                        let mut point = current_event.location();

                        point.x += dx as f64;
                        point.y += dy as f64;

                        let move_event = CGEvent::new_mouse_event(source, CGEventType::MouseMoved, point, CGMouseButton::Left)
                            .ok_or("Failed to create mouse move event")?;
                        move_event.post(core_graphics::event::CGEventTapLocation::HID);
                        log::debug!("Mouse moved relatively to: {}, {}", point.x, point.y);
                    }
                    KvmEvent::MouseAbsMove { x, y } => {
                        log::debug!("Processing MouseAbsMove: x={}, y={}", x, y);
                        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
                            .ok_or("Failed to create event source")?;
                        let point = core_graphics::geometry::CGPoint::new(x as f64, y as f64);

                        let move_event = CGEvent::new_mouse_event(source, CGEventType::MouseMoved, point, CGMouseButton::Left)
                            .ok_or("Failed to create mouse move event")?;
                        move_event.post(core_graphics::event::CGEventTapLocation::HID);
                        log::info!("Moved cursor to: {}, {}", x, y);
                    }
                    KvmEvent::MouseButton { button, is_down } => {
                        log::info!("MouseButton event received: button={}, is_down={}", button, is_down);

                        // Create event source for button injection
                        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
                            .ok_or("Failed to create event source")?;

                        // Get current mouse position
                        let current_event = CGEvent::new(source.clone())
                            .ok_or("Failed to get current mouse event")?;
                        let point = current_event.location();

                        // Map button ID to CGMouseButton and determine event type
                        let (mouse_button, event_type) = match button {
                            0 => {
                                // Left mouse button
                                let evt_type = if is_down {
                                    CGEventType::LeftMouseDown
                                } else {
                                    CGEventType::LeftMouseUp
                                };
                                (CGMouseButton::Left, evt_type)
                            }
                            1 => {
                                // Right mouse button
                                let evt_type = if is_down {
                                    CGEventType::RightMouseDown
                                } else {
                                    CGEventType::RightMouseUp
                                };
                                (CGMouseButton::Right, evt_type)
                            }
                            2 => {
                                // Middle mouse button
                                let evt_type = if is_down {
                                    CGEventType::OtherMouseDown
                                } else {
                                    CGEventType::OtherMouseUp
                                };
                                (CGMouseButton::Center, evt_type)
                            }
                            _ => {
                                // Unknown button ID
                                log::warn!("Unknown mouse button ID: {}", button);
                                continue;
                            }
                        };

                        // Create and post the button event
                        match CGEvent::new_mouse_event(source, event_type, point, mouse_button) {
                            Ok(button_event) => {
                                button_event.post(core_graphics::event::CGEventTapLocation::HID);
                                let button_name = match button {
                                    0 => "left",
                                    1 => "right",
                                    2 => "middle",
                                    _ => "unknown",
                                };
                                let action = if is_down { "pressed" } else { "released" };
                                log::info!("Mouse {} button {} at ({}, {})", button_name, action, point.x, point.y);
                            }
                            Err(e) => {
                                log::error!("Failed to create mouse button event: {:?}", e);
                            }
                        }
                    }
                    KvmEvent::Key { keycode, is_down } => {
                        log::debug!("Key event received: keycode={}, is_down={}", keycode, is_down);

                        // Translate Linux keycode to macOS virtual keycode
                        match custom_kvm::keycodes::linux_to_macos_keycode(keycode) {
                            Some(mac_keycode) => {
                                // Create event source for keyboard injection
                                match CGEventSource::new(CGEventSourceStateID::CombinedSessionState) {
                                    Ok(source) => {
                                        // Create keyboard event using CGEvent::new_keyboard_event()
                                        match CGEvent::new_keyboard_event(source, mac_keycode, is_down) {
                                            Ok(key_event) => {
                                                // Post event to the HID system
                                                key_event.post(core_graphics::event::CGEventTapLocation::HID);
                                                let action = if is_down { "pressed" } else { "released" };
                                                log::info!("Posted keyboard event: linux_keycode={}, mac_keycode={}, action={}",
                                                    keycode, mac_keycode, action);
                                            }
                                            Err(e) => {
                                                log::error!("Failed to create keyboard event for keycode {}: {:?}", keycode, e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("Failed to create event source for keyboard event: {:?}", e);
                                    }
                                }
                            }
                            None => {
                                log::warn!("Unknown or unsupported Linux keycode: {} (no macOS equivalent)", keycode);
                            }
                        }
                    }
                    KvmEvent::MouseScroll { delta } => {
                        log::debug!("MouseScroll event received: delta={}", delta);
                        // Silently ignore for now
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to decode packet from {}: {}", src, e);
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_client()
}

// Non-macOS stub: compile-time safe listener that logs received events but does no injection.
#[cfg(not(target_os = "macos"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    custom_kvm::logging::init_logging("info")?;

    // Load configuration
    let config = custom_kvm::config::ClientConfig::load("kvm-client.toml")
        .unwrap_or_else(|_| custom_kvm::config::ClientConfig::default());

    log::info!("KVM Client (non-macOS) starting");
    log::debug!("Binding to: {}", config.bind_addr);

    let socket = UdpSocket::bind(&config.bind_addr)?;
    let mut buf = [0u8; 1024];

    log::info!("KVM Client (non-macOS) listening for inputs from remote desktop (injection disabled)...");

    loop {
        let (amt, src) = socket.recv_from(&mut buf)?;
        log::debug!("Received {} bytes from {}", amt, src);

        match bincode::deserialize::<KvmEvent>(&buf[..amt]) {
            Ok(decoded) => {
                log::info!("Received event from {}: {:?}", src, decoded);
            }
            Err(e) => {
                log::error!("Failed to decode packet from {}: {}", src, e);
            }
        }
    }
}
