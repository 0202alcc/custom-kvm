use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::panic;
use std::time::{Instant, Duration};
use std::thread;
use std::sync::mpsc;
use custom_kvm::{KvmEvent, EdgeDetector, EdgeDetectionResult, config::ServerConfig, logging, DisplayInfo, DisplayOrientation};
use evdev::{InputEventKind, RelativeAxisType, EventType};

struct InputDevices {
    mouse: Option<evdev::Device>,
    keyboard: Option<evdev::Device>,
}

fn find_input_devices() -> Option<InputDevices> {
    let mut mouse_device = None;
    let mut keyboard_device = None;

    let devices = evdev::enumerate();
    for (path, device_info) in devices {
        let device_name = device_info.name().unwrap_or("Unknown").to_string();
        let device = evdev::Device::open(&path).ok()?;
        
        let mut is_mouse_candidate = false;
        let mut is_keyboard_candidate = false;

        if let Some(abs) = device.supported_absolute_axes() {
            if abs.iter().any(|axis| axis.0 == 0 || axis.0 == 1) { // 0: ABS_X, 1: ABS_Y
                is_mouse_candidate = true;
            }
        }
        if let Some(rel) = device.supported_relative_axes() {
            if rel.iter().any(|axis| axis.0 == 0 || axis.0 == 1) { // 0: REL_X, 1: REL_Y
                is_mouse_candidate = true;
            }
        }

        let has_key = device.supported_keys().is_some();
        if has_key {
            if let Some(keys) = device.supported_keys() {
                let key_count = keys.iter().count();
                if key_count >= 20 {
                    is_keyboard_candidate = true;
                }
            }
        }

        if is_mouse_candidate && mouse_device.is_none() && !device_name.to_lowercase().contains("passthrough") {
            log::debug!("Auto-detected mouse device: {:?} (path={:?})", device_name, path);
            mouse_device = Some(device);
        } else if is_keyboard_candidate && keyboard_device.is_none() {
            log::debug!("Auto-detected keyboard device: \"{}\" (path={:?})", device_name, path);
            keyboard_device = Some(device);
        }
    }

    if mouse_device.is_some() || keyboard_device.is_some() {
        Some(InputDevices { mouse: mouse_device, keyboard: keyboard_device })
    } else {
        None
    }
}

fn get_linux_displays() -> Vec<DisplayInfo> {
    let output = std::process::Command::new("xrandr").arg("--query").output();
    let mut displays = Vec::new();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for (i, line) in stdout.lines().enumerate() {
            if line.contains(" connected ") {
                let orientation = if line.contains("rotate left") {
                    DisplayOrientation::Left
                } else if line.contains("rotate right") {
                    DisplayOrientation::Right
                } else if line.contains("rotate inverted") {
                    DisplayOrientation::Inverted
                } else {
                    DisplayOrientation::Normal
                };

                let (mut width, mut height, mut x, mut y) = (0, 0, 0, 0);
                if let Some(geo_part) = line.split_whitespace().find(|s| s.contains('x') && s.contains('+')) {
                    let parts: Vec<&str> = geo_part.split(|c| c == 'x' || c == '+').collect();
                    if parts.len() == 4 {
                        width = parts[0].parse().unwrap_or(0);
                        height = parts[1].parse().unwrap_or(0);
                        x = parts[2].parse().unwrap_or(0);
                        y = parts[3].parse().unwrap_or(0);
                    }
                }

                displays.push(DisplayInfo { id: i as u32, orientation, x, y, width, height });
            }
        }
    }
    displays
}

fn map_keycode_to_button(keycode: u16) -> Option<u8> {
    match keycode {
        0x110 => Some(0), 0x111 => Some(1), 0x112 => Some(2),
        _ => None,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::load("kvm-server.toml")?;
    logging::init_logging(&config.log_level).ok();

    log::info!("Starting KVM Server...");

    let socket = Arc::new(UdpSocket::bind(&config.bind_addr)?);
    let client_address = config.client_addr.as_str();

    let input_devices = find_input_devices().ok_or("No input devices found")?;
    if input_devices.keyboard.is_some() {
        log::info!("Keyboard device found and initialized");
    }

    let shared_devices = Arc::new(Mutex::new(input_devices));
    let shutdown = Arc::new(AtomicBool::new(false));

    let shutdown_flag = Arc::clone(&shutdown);
    ctrlc::set_handler(move || {
        log::info!("Received CTRL+C signal, initiating graceful shutdown...");
        shutdown_flag.store(true, Ordering::SeqCst);
        std::process::exit(0);
    })?;

    let d_panic = Arc::clone(&shared_devices);
    panic::set_hook(Box::new(move |panic_info| {
        log::error!("Code panicked: {:?}", panic_info);
        if let Ok(mut devices) = d_panic.try_lock() {
            if let Some(ref mut dev) = devices.mouse { let _ = dev.ungrab(); }
            if let Some(ref mut dev) = devices.keyboard { let _ = dev.ungrab(); }
        }
    }));

    let edge_detector = Arc::new(Mutex::new(EdgeDetector::new(config.screen_width, config.screen_height)));
    let last_heartbeat = Arc::new(Mutex::new(Instant::now()));
    let is_client_connected = Arc::new(AtomicBool::new(false));

    let (mouse_tx, mouse_rx) = mpsc::channel::<evdev::InputEvent>();
    let (kb_tx, kb_rx) = mpsc::channel::<evdev::InputEvent>();

    let mut devices_lock = shared_devices.lock().unwrap();
    let mouse_for_thread = devices_lock.mouse.take();
    let keyboard_for_thread = devices_lock.keyboard.take();
    drop(devices_lock);

    let mouse_tx_clone = mouse_tx.clone();
    thread::spawn(move || {
        if let Some(mut mouse) = mouse_for_thread {
            loop {
                match mouse.fetch_events() {
                    Ok(events) => {
                        for event in events {
                            if mouse_tx_clone.send(event).is_err() { return; }
                        }
                    }
                    Err(_) => thread::sleep(Duration::from_millis(10)),
                }
            }
        }
    });

    let kb_tx_clone = kb_tx.clone();
    thread::spawn(move || {
        if let Some(mut keyboard) = keyboard_for_thread {
            loop {
                match keyboard.fetch_events() {
                    Ok(events) => {
                        for event in events {
                            if kb_tx_clone.send(event).is_err() { return; }
                        }
                    }
                    Err(_) => thread::sleep(Duration::from_millis(10)),
                }
            }
        }
    });

    let net_socket = Arc::clone(&socket);
    let net_connected = Arc::clone(&is_client_connected);
    let net_heartbeat = Arc::clone(&last_heartbeat);
    thread::spawn(move || {
        let mut buf = [0u8; 1024];
        loop {
            match net_socket.recv_from(&mut buf) {
                Ok((amt, src)) => {
                    if let Ok(event) = bincode::deserialize::<KvmEvent>(&buf[..amt]) {
                        match event {
                            KvmEvent::Heartbeat => {
                                if !net_connected.load(Ordering::SeqCst) {
                                    log::info!("Client connected from {}. KVM Edge Detection active.", src);
                                    let displays = get_linux_displays();
                                    let report = KvmEvent::DisplayReport { displays };
                                    if let Ok(ser) = bincode::serialize(&report) {
                                        let _ = net_socket.send_to(&ser, &src);
                                    }
                                }
                                net_connected.store(true, Ordering::SeqCst);
                                if let Ok(mut hb) = net_heartbeat.lock() { *hb = Instant::now(); }
                            }
                            KvmEvent::DisplayReport { displays } => {
                                log::info!("Received display report from client {}: {} displays", src, displays.len());
                            }
                            KvmEvent::LogMessage { message } => {
                                log::info!("[CLIENT LOG] {}", message);
                            }
                            _ => {}
                        }
                    }
                }
                Err(_) => thread::sleep(Duration::from_millis(100)),
            }
        }
    });

    loop {
        if is_client_connected.load(Ordering::SeqCst) {
            if let Ok(hb) = last_heartbeat.lock() {
                if hb.elapsed() > Duration::from_secs(5) {
                    log::warn!("WARN: Client disconnected (timeout)");
                    is_client_connected.store(false, Ordering::SeqCst);
                }
            }
        }

        if shutdown.load(Ordering::SeqCst) { break; }

        let mut all_events = Vec::new();
        while let Ok(event) = mouse_rx.try_recv() { all_events.push(event); }
        while let Ok(event) = kb_rx.try_recv() { all_events.push(event); }

        if all_events.is_empty() {
            std::hint::spin_loop();
            continue;
        }

        for event in all_events {
            let (mut dx, mut dy) = (0, 0);
            match event.kind() {
                InputEventKind::RelAxis(RelativeAxisType::REL_X) => dx = event.value(),
                InputEventKind::RelAxis(RelativeAxisType::REL_Y) => dy = event.value(),
                InputEventKind::Key(key) => {
                    let keycode = key.code() as u16;
                    let is_down = event.value() != 0;
                    if let Some(button) = map_keycode_to_button(keycode) {
                        let ev = KvmEvent::MouseButton { button, is_down };
                        if let Ok(ser) = bincode::serialize(&ev) {
                            let _ = socket.send_to(&ser, client_address);
                        }
                        continue;
                    }
                    let ev = KvmEvent::Key { keycode, is_down };
                    if let Ok(ser) = bincode::serialize(&ev) {
                        let _ = socket.send_to(&ser, client_address);
                    }
                    continue;
                }
                _ => {}
            }

            let result = edge_detector.lock().unwrap().update(dx, dy);
            match result {
                EdgeDetectionResult::TransitionToMac => {
                    log::info!("Transitioning control to Mac!");
                    if let Ok(mut devices) = shared_devices.lock() {
                        if let Some(ref mut dev) = devices.mouse { let _ = dev.grab(); }
                    }
                    let vy = edge_detector.lock().unwrap().virtual_y();
                    let ev = KvmEvent::MouseAbsMove { x: 1, y: vy.clamp(0, config.screen_height) };
                    if let Ok(ser) = bincode::serialize(&ev) {
                        let _ = socket.send_to(&ser, client_address);
                    }
                }
                EdgeDetectionResult::TransitionToLinux => {
                    log::info!("Returning control to Linux!");
                    if let Ok(mut devices) = shared_devices.lock() {
                        if let Some(ref mut dev) = devices.mouse { let _ = dev.ungrab(); }
                    }
                }
                EdgeDetectionResult::None => {
                    if edge_detector.lock().unwrap().is_controlling_mac() {
                        let ev = KvmEvent::MouseMove { dx, dy };
                        if let Ok(ser) = bincode::serialize(&ev) {
                            let _ = socket.send_to(&ser, client_address);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
