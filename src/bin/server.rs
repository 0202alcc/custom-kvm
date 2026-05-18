use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::panic;
use custom_kvm::{KvmEvent, config::ServerConfig, logging};
use evdev::{InputEventKind, RelativeAxisType, EventType};

/// Holds both mouse and keyboard input devices
struct InputDevices {
    mouse: Option<evdev::Device>,
    keyboard: Option<evdev::Device>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    logging::init_logging("info")?;

    // Load configuration
    let config = ServerConfig::load("kvm-server.toml").unwrap_or_else(|e| {
        log::warn!("Config file not found or invalid ({}), using defaults", e);
        ServerConfig::default()
    });

    log::info!("Starting KVM server");
    log::info!("Server config: bind_addr={}, client_addr={}, screen={}x{}",
        config.bind_addr, config.client_addr, config.screen_width, config.screen_height);

    let socket = UdpSocket::bind(&config.bind_addr)?;
    let client_address = &config.client_addr;

    let input_devices = find_input_devices()
        .ok_or("No mouse or keyboard device found. Please ensure they are connected.")?;

    if input_devices.mouse.is_some() {
        log::info!("Mouse device found and initialized");
    }
    if input_devices.keyboard.is_some() {
        log::info!("Keyboard device found and initialized");
    }

    // Wrap the devices so they can be shared safely with the crash/signal handlers
    let shared_devices = Arc::new(Mutex::new(input_devices));

    // Create a shutdown flag to signal clean exit (avoids deadlock in signal handler)
    let shutdown = Arc::new(AtomicBool::new(false));

    // --- 1. SET UP THE CTRL+C SIGNAL HANDLER ---
    let shutdown_flag = Arc::clone(&shutdown);
    let d_graceful = Arc::clone(&shared_devices);
    ctrlc::set_handler(move || {
        log::info!("Received CTRL+C signal, initiating graceful shutdown...");
        shutdown_flag.store(true, Ordering::SeqCst);

        // Try to ungrab devices, but don't wait if we can't get the lock
        // (The OS will clean up anyway when the process exits)
        if let Ok(mut devices) = d_graceful.try_lock() {
            if let Some(ref mut dev) = devices.mouse {
                let _ = dev.ungrab();
            }
            if let Some(ref mut dev) = devices.keyboard {
                let _ = dev.ungrab();
            }
        }

        // Exit immediately - don't wait for main loop to notice the flag
        std::process::exit(0);
    })?;

    // --- 2. SET UP THE PANIC HOOK (FOR CODE CRASHES) ---
    let d_panic = Arc::clone(&shared_devices);
    panic::set_hook(Box::new(move |panic_info| {
        log::error!("Code panicked: {:?}", panic_info);
        log::info!("Safely releasing devices before exit...");
        if let Ok(mut devices) = d_panic.lock() {
            if let Some(ref mut dev) = devices.mouse {
                let _ = dev.ungrab();
            }
            if let Some(ref mut dev) = devices.keyboard {
                let _ = dev.ungrab();
            }
        }
    }));

    // --- KVM STATE VARIABLES ---
    let mut virtual_x = config.screen_width / 2;
    let mut virtual_y = config.screen_height / 2;
    let mut is_controlling_mac = false;

    log::info!("KVM Edge Detection active. Move mouse past right edge to switch to Mac.");

    loop {
        // Check for shutdown signal before processing
        if shutdown.load(Ordering::SeqCst) {
            log::info!("Shutdown signal received, cleaning up...");
            // Gracefully ungrab devices before exit
            if let Ok(mut devices) = shared_devices.lock() {
                if let Some(ref mut dev) = devices.mouse {
                    let _ = dev.ungrab();
                }
                if let Some(ref mut dev) = devices.keyboard {
                    let _ = dev.ungrab();
                }
            }
            log::info!("Devices released, exiting cleanly.");
            break;
        }
        // Lock the devices briefly to fetch the incoming events
        let mut all_events: Vec<_> = Vec::new();
        {
            let mut devices = match shared_devices.lock() {
                Ok(d) => d,
                Err(e) => {
                    log::error!("Failed to acquire device lock: {}", e);
                    continue;
                }
            };

            // Fetch events from mouse device
            if let Some(ref mut mouse) = devices.mouse {
                match mouse.fetch_events() {
                    Ok(iter) => {
                        all_events.extend(iter);
                    }
                    Err(e) => {
                        log::debug!("Failed to fetch mouse events: {}", e);
                    }
                };
            }

            // Fetch events from keyboard device
            if let Some(ref mut keyboard) = devices.keyboard {
                match keyboard.fetch_events() {
                    Ok(iter) => {
                        all_events.extend(iter);
                    }
                    Err(e) => {
                        log::debug!("Failed to fetch keyboard events: {}", e);
                    }
                };
            }
        }

        for event in all_events {
            let mut dx = 0;
            let mut dy = 0;

            match event.kind() {
                InputEventKind::RelAxis(RelativeAxisType::REL_X) => dx = event.value(),
                InputEventKind::RelAxis(RelativeAxisType::REL_Y) => dy = event.value(),
                InputEventKind::Key(key) => {
                    let keycode = key.code() as u16;
                    let is_down = event.value() != 0;

                    // Check if this is a mouse button event
                    if let Some(button) = map_keycode_to_button(keycode) {
                        log::debug!("Captured mouse button event: button={}, is_down={}", button, is_down);

                        let button_event = KvmEvent::MouseButton { button, is_down };
                        match bincode::serialize(&button_event) {
                            Ok(serialized) => {
                                if let Err(e) = socket.send_to(&serialized, client_address) {
                                    log::error!("Failed to send mouse button event to client: {}", e);
                                } else {
                                    log::info!("Sent mouse button event: button={}, is_down={}", button, is_down);
                                }
                            }
                            Err(e) => {
                                log::error!("Serialization error for mouse button event: {}", e);
                            }
                        }
                        continue;
                    }

                    // If not a mouse button, treat as keyboard event
                    log::debug!("Captured keyboard event: key={:?}, is_down={}", keycode, is_down);

                    let key_event = KvmEvent::Key { keycode, is_down };
                    match bincode::serialize(&key_event) {
                        Ok(serialized) => {
                            if let Err(e) = socket.send_to(&serialized, client_address) {
                                log::error!("Failed to send keyboard event to client: {}", e);
                            } else {
                                log::info!("Sent keyboard event to client: keycode={}, is_down={}", keycode, is_down);
                            }
                        }
                        Err(e) => {
                            log::error!("Serialization error for keyboard event: {}", e);
                        }
                    }
                    continue;
                }
                _ => {}
            }

            if dx == 0 && dy == 0 { continue; }

            virtual_x += dx;
            virtual_y += dy;
            virtual_y = virtual_y.clamp(0, config.screen_height);

            if !is_controlling_mac {
                // Check boundary BEFORE clamping to allow proper transition detection
                if virtual_x > config.screen_width {
                    log::info!("Transitioning control to Mac! (virtual_x={} > screen_width={})", virtual_x, config.screen_width);

                    // Lock and Grab
                    {
                        match shared_devices.lock() {
                            Ok(mut devices) => {
                                if let Some(ref mut dev) = devices.mouse {
                                    if let Err(e) = dev.grab() {
                                        log::error!("Failed to grab mouse: {}", e);
                                        continue;
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to acquire device lock: {}", e);
                                continue;
                            }
                        }
                    }
                    is_controlling_mac = true;

                    // Send cursor to far right edge of Mac screen (0, y) or slightly left to allow movement back
                    let edge_entry = KvmEvent::MouseAbsMove { x: 1, y: virtual_y.clamp(0, config.screen_height) };
                    match bincode::serialize(&edge_entry) {
                        Ok(serialized) => {
                            if let Err(e) = socket.send_to(&serialized, client_address) {
                                log::error!("Failed to send event to client: {}", e);
                            } else {
                                log::debug!("Sent absolute mouse move to client at edge: x=1, y={}", virtual_y);
                            }
                        }
                        Err(e) => {
                            log::error!("Serialization error: {}", e);
                        }
                    }
                    // Don't reset virtual_x - let it stay high so we can detect return movement
                } else {
                    // Normal operation within Linux screen bounds
                    virtual_x = virtual_x.clamp(0, config.screen_width);
                }
            } else {
                let move_ev = KvmEvent::MouseMove { dx, dy };
                match bincode::serialize(&move_ev) {
                    Ok(serialized) => {
                        if let Err(e) = socket.send_to(&serialized, client_address) {
                            log::error!("Failed to send event to client: {}", e);
                        } else {
                            log::debug!("Sent relative mouse move to client: dx={}, dy={}", dx, dy);
                        }
                    }
                    Err(e) => {
                        log::error!("Serialization error: {}", e);
                    }
                }

                if virtual_x < 0 {
                    log::info!("Returning control to Linux!");

                    // Lock and Ungrab
                    {
                        match shared_devices.lock() {
                            Ok(mut devices) => {
                                if let Some(ref mut dev) = devices.mouse {
                                    if let Err(e) = dev.ungrab() {
                                        log::error!("Failed to ungrab mouse: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to acquire device lock: {}", e);
                            }
                        }
                    }
                    is_controlling_mac = false;
                    virtual_x = config.screen_width - 10;
                }
            }
        }
    }
    Ok(())
}

/// Maps Linux mouse button keycodes to button IDs
/// BTN_LEFT = 0x110 → button 0
/// BTN_RIGHT = 0x111 → button 1
/// BTN_MIDDLE = 0x112 → button 2
fn map_keycode_to_button(keycode: u16) -> Option<u8> {
    match keycode {
        0x110 => Some(0), // BTN_LEFT
        0x111 => Some(1), // BTN_RIGHT
        0x112 => Some(2), // BTN_MIDDLE
        _ => None,
    }
}

fn find_input_devices() -> Option<InputDevices> {
    let mut mouse_device = None;
    let mut keyboard_device = None;

    // Enumerate all input devices
    for (_path, device) in evdev::enumerate() {
        let device_name = device.name().unwrap_or("Unknown Device");
        let supported_events = device.supported_events();

        // Check if this device has both RELATIVE (mouse) and KEY (keyboard) events
        let has_relative = supported_events.contains(EventType::RELATIVE);
        let has_key = supported_events.contains(EventType::KEY);

        // Check for mouse devices first (RELATIVE with REL_X, REL_Y)
        if mouse_device.is_none() && has_relative {
            if let Some(rel_axes) = device.supported_relative_axes() {
                if rel_axes.contains(RelativeAxisType::REL_X) && rel_axes.contains(RelativeAxisType::REL_Y) {
                    log::debug!("Auto-detected mouse device: \"{}\"", device_name);
                    mouse_device = Some(device);
                    continue;
                }
            }
        }

        // Check for keyboard devices (KEY events) - independent of mouse search
        if keyboard_device.is_none() && has_key {
            // Skip if this device is also our mouse (it will have both RELATIVE and KEY)
            if has_relative && mouse_device.is_some() {
                // This is likely a combo device we already grabbed as mouse, skip it
                continue;
            }

            if let Some(_keys) = device.supported_keys() {
                log::debug!("Auto-detected keyboard device: \"{}\"", device_name);
                keyboard_device = Some(device);
            }
        }
    }

    // Return Some if we found at least one device
    if mouse_device.is_some() || keyboard_device.is_some() {
        Some(InputDevices {
            mouse: mouse_device,
            keyboard: keyboard_device,
        })
    } else {
        None
    }
}
