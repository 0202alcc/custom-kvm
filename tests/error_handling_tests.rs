use custom_kvm::{KvmEvent, config::{ServerConfig, ClientConfig}, keycodes};

/// Test that config loading returns defaults when file doesn't exist
#[test]
fn test_config_defaults_when_file_missing() {
    let config = ServerConfig::load("/tmp/nonexistent_kvm_12345.toml");
    assert!(config.is_ok(), "Config should not error on missing file");
    let cfg = config.unwrap();
    assert_eq!(cfg.bind_addr, "0.0.0.0:8080");
    assert_eq!(cfg.client_addr, "0.0.0.0:8080");
}

/// Test that client config returns defaults when file doesn't exist
#[test]
fn test_client_config_defaults_when_file_missing() {
    let config = ClientConfig::load("/tmp/nonexistent_kvm_client_12345.toml");
    assert!(config.is_ok(), "Client config should not error on missing file");
    let cfg = config.unwrap();
    assert_eq!(cfg.bind_addr, "0.0.0.0:8080");
    assert_eq!(cfg.log_level, "info");
}

/// Test that all common letter keys map successfully
#[test]
fn test_keycode_mapping_letter_keys() {
    // KEY_A through KEY_Z (Linux keycodes 30-55)
    let letter_codes: Vec<u16> = vec![30, 48, 46, 32, 18, 33, 34, 35, 23, 36, 37, 38, 50, 49, 24, 25, 16, 19, 31, 20, 22, 47, 17, 45, 21, 44];

    for code in letter_codes {
        let mapped = keycodes::linux_to_macos_keycode(code);
        assert!(mapped.is_some(), "Letter key {} should map to macOS keycode", code);
    }
}

/// Test that all number keys map successfully
#[test]
fn test_keycode_mapping_number_keys() {
    // KEY_0 through KEY_9
    let number_codes: Vec<u16> = vec![11, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    for code in number_codes {
        let mapped = keycodes::linux_to_macos_keycode(code);
        assert!(mapped.is_some(), "Number key {} should map to macOS keycode", code);
    }
}

/// Test that unknown keycodes return None gracefully (no panic)
#[test]
fn test_keycode_mapping_unknown_codes() {
    let unknown_codes = vec![9999, 5000, 200, 300, 400];

    for code in unknown_codes {
        let mapped = keycodes::linux_to_macos_keycode(code);
        assert!(mapped.is_none(), "Unknown keycode {} should return None", code);
    }
}

/// Test KvmEvent::MouseMove serialization roundtrip
#[test]
fn test_kvm_event_mouse_move_roundtrip() {
    let original = KvmEvent::MouseMove { dx: 50, dy: -100 };
    let serialized = bincode::serialize(&original).expect("Should serialize");
    let deserialized: KvmEvent = bincode::deserialize(&serialized).expect("Should deserialize");
    assert_eq!(original, deserialized);
}

/// Test KvmEvent::MouseAbsMove serialization roundtrip
#[test]
fn test_kvm_event_mouse_abs_move_roundtrip() {
    let original = KvmEvent::MouseAbsMove { x: 1920, y: 1080 };
    let serialized = bincode::serialize(&original).expect("Should serialize");
    let deserialized: KvmEvent = bincode::deserialize(&serialized).expect("Should deserialize");
    assert_eq!(original, deserialized);
}

/// Test KvmEvent::MouseButton serialization roundtrip
#[test]
fn test_kvm_event_mouse_button_roundtrip() {
    let original = KvmEvent::MouseButton { button: 0, is_down: true };
    let serialized = bincode::serialize(&original).expect("Should serialize");
    let deserialized: KvmEvent = bincode::deserialize(&serialized).expect("Should deserialize");
    assert_eq!(original, deserialized);
}

/// Test KvmEvent::Key serialization roundtrip
#[test]
fn test_kvm_event_key_roundtrip() {
    let original = KvmEvent::Key { keycode: 30, is_down: true };
    let serialized = bincode::serialize(&original).expect("Should serialize");
    let deserialized: KvmEvent = bincode::deserialize(&serialized).expect("Should deserialize");
    assert_eq!(original, deserialized);
}

/// Test all KvmEvent types can be serialized and deserialized
#[test]
fn test_all_kvm_event_types_roundtrip() {
    let events = vec![
        KvmEvent::MouseMove { dx: 10, dy: 20 },
        KvmEvent::MouseAbsMove { x: 1024, y: 768 },
        KvmEvent::MouseButton { button: 1, is_down: false },
        KvmEvent::Key { keycode: 42, is_down: false },
        KvmEvent::MouseScroll { delta: -3 },
    ];

    for event in events {
        let serialized = bincode::serialize(&event).expect("Should serialize");
        let deserialized: KvmEvent = bincode::deserialize(&serialized).expect("Should deserialize");
        assert_eq!(event, deserialized, "Event type should roundtrip correctly");
    }
}

/// Test ServerConfig can be created with defaults
#[test]
fn test_server_config_defaults() {
    let config = ServerConfig::default();
    assert_eq!(config.bind_addr, "0.0.0.0:8080");
    assert_eq!(config.client_addr, "0.0.0.0:8080");
    assert_eq!(config.screen_width, 1920);
    assert_eq!(config.screen_height, 1080);
    assert_eq!(config.log_level, "info");
}

/// Test ClientConfig can be created with defaults
#[test]
fn test_client_config_defaults() {
    let config = ClientConfig::default();
    assert_eq!(config.bind_addr, "0.0.0.0:8080");
    assert_eq!(config.log_level, "info");
}
