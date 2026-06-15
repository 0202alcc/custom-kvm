# Agent Context: custom-kvm

## Project Purpose
`custom-kvm` is a network-based KVM (Keyboard, Video, Mouse) switch that allows a user to share a single set of input devices between a Linux machine (Server) and a macOS machine (Client) over UDP.

## Architecture

### 1. Server (Linux)
- **Role**: Input capture and orchestration.
- **Mechanism**: Uses the `evdev` crate to read raw events from the kernel.
- **Key Feature: Edge Detection**: Implements a virtual coordinate system. When the cursor moves past the right edge of the configured Linux screen, the server "grabs" the devices (exclusive access) and forwards all input to the client.
- **Safety**: Uses `ctrlc` signal handlers and `std::panic::set_hook` to ensure devices are ungrabbed if the process terminates unexpectedly.

### 2. Client (macOS)
- **Role**: Input injection.
- **Mechanism**: Uses the `core-graphics` crate to simulate OS-level HID events.
- **Key Feature: Translation**: Translates Linux-native keycodes into macOS-compatible keycodes via a translation layer.

### 3. Communication Protocol
- **Transport**: UDP for low latency.
- **Serialization**: Binary serialization via `bincode` based on the `KvmEvent` enum.

## Component Map

| File | Responsibility |
| :--- | :--- |
| `src/lib.rs` | Protocol definition (`KvmEvent`) and module declarations. |
| `src/bin/server.rs` | Linux server logic: device discovery, grabbing, edge detection, and UDP sender. |
| `src/bin/client.rs` | macOS client logic: UDP listener, button state tracking, and event injection. |
| `src/keycodes.rs` | Logic for mapping Linux keycodes -> macOS keycodes. |
| `src/config.rs` | Configuration loading (TOML) for server and client. |

## Critical Technical Constraints

- **Device Grabbing**: The server must call `.grab()` on `evdev` devices to prevent the local Linux OS from receiving input while the Mac is being controlled. Failure to `.ungrab()` can leave the host system unresponsive.
- **Platform Gating**: Implementation is heavily gated using `#[cfg(target_os = "linux")]` and `#[cfg(target_os = "macos")]`.
- **Coordinate Space**: The server maintains a `virtual_x` and `virtual_y` to handle transitions between screens regardless of the actual local cursor position.
