# KVM Configuration Guide

This document describes how to configure the custom-kvm system for your setup.

## Overview

The KVM system consists of two components:
- **Server (Linux)**: Captures keyboard/mouse input and sends to client
- **Client (macOS)**: Receives events and injects them into the system

Both components support configuration via TOML files for network addresses, display dimensions, and logging levels.

## Server Configuration

Create a file named `kvm-server.toml` in the working directory to configure the server.

### Server Configuration Options

#### `bind_addr` (string)
- **Default:** `"0.0.0.0:8080"`
- **Description:** IP address and port the server binds to for receiving local input events and for listening to network connections
- **Examples:**
  - `"0.0.0.0:8080"` - Listen on all interfaces, port 8080
  - `"127.0.0.1:8080"` - Listen only on localhost
  - `"192.168.1.50:9000"` - Listen on specific IP and port

#### `client_addr` (string)
- **Default:** `"127.0.0.1:8080"`
- **Description:** IP address and port of the macOS client to send events to
- **Important:** This MUST match the IP address and port the client is listening on
- **Examples:**
  - `"192.168.1.100:8080"` - Send to macOS machine on local network
  - `"mac-machine.local:8080"` - Use hostname instead of IP
  - `"10.0.0.50:9000"` - Send to specific IP and custom port

#### `screen_width` (integer)
- **Default:** `1920`
- **Description:** Horizontal resolution of the Linux display in pixels
- **Used for:** Mouse edge detection (moving past right edge switches to macOS control)
- **Examples:**
  - `1920` - Standard 1080p width
  - `2560` - 1440p width
  - `3840` - 4K width

#### `screen_height` (integer)
- **Default:** `1080`
- **Description:** Vertical resolution of the Linux display in pixels
- **Used for:** Clamping mouse Y coordinate to valid range
- **Examples:**
  - `1080` - Standard 1080p height
  - `1440` - 1440p height
  - `2160` - 4K height

#### `log_level` (string)
- **Default:** `"info"`
- **Options:** `"debug"`, `"info"`, `"warn"`, `"error"`
- **Description:** Minimum logging level to output
- **Examples:**
  - `"debug"` - Verbose logging, shows all events (useful for troubleshooting)
  - `"info"` - Standard logging, shows important events
  - `"warn"` - Only show warnings and errors
  - `"error"` - Only show errors

### Server Configuration Example

```toml
# kvm-server.toml
bind_addr = "0.0.0.0:8080"
client_addr = "192.168.1.100:8080"
screen_width = 2560
screen_height = 1440
log_level = "debug"
```

## Client Configuration

Create a file named `kvm-client.toml` in the working directory to configure the client.

### Client Configuration Options

#### `bind_addr` (string)
- **Default:** `"0.0.0.0:8080"`
- **Description:** IP address and port the client binds to for receiving events from the server
- **Should typically match:** The server's `client_addr` setting
- **Examples:**
  - `"0.0.0.0:8080"` - Listen on all interfaces, port 8080
  - `"127.0.0.1:8080"` - Listen only on localhost
  - `"192.168.1.100:8080"` - Listen on specific IP

#### `log_level` (string)
- **Default:** `"info"`
- **Options:** `"debug"`, `"info"`, `"warn"`, `"error"`
- **Description:** Minimum logging level to output

### Client Configuration Example

```toml
# kvm-client.toml
bind_addr = "0.0.0.0:8080"
log_level = "info"
```

## Using Configuration Files

### Starting with Defaults

If no configuration file is present, the system uses built-in defaults:

```bash
# Server uses defaults (0.0.0.0:8080 → 127.0.0.1:8080 at 1920x1080)
# Please use .example.toml files to configure actual network addresses.
./target/release/kvm-server

# Client uses defaults (0.0.0.0:8080)
./target/release/kvm-client
```

### With Configuration Files

1. Copy the example files:
   ```bash
   cp kvm-server.toml.example kvm-server.toml
   cp kvm-client.toml.example kvm-client.toml
   ```

2. Edit the configuration files for your setup:
   ```bash
   vim kvm-server.toml   # Update client_addr, screen dimensions
   vim kvm-client.toml   # Update bind_addr if needed
   ```

3. Run the binaries from the directory containing the config files:
   ```bash
   ./target/release/kvm-server   # Loads kvm-server.toml
   ./target/release/kvm-client   # Loads kvm-client.toml
   ```

### Environment Variables for Logging

Override the log level using the `RUST_LOG` environment variable:

```bash
# Show all debug messages
RUST_LOG=debug ./target/release/kvm-server

# Show only warnings and errors
RUST_LOG=warn ./target/release/kvm-client

# Show debug for custom_kvm module only
RUST_LOG=custom_kvm=debug ./target/release/kvm-server
```

## Common Configuration Scenarios

### Local Network Setup

Linux server and macOS client on the same network:

**Server (192.168.1.50):**
```toml
bind_addr = "0.0.0.0:8080"
client_addr = "192.168.1.100:8080"  # macOS IP
screen_width = 1920
screen_height = 1080
log_level = "info"
```

**Client (192.168.1.100):**
```toml
bind_addr = "0.0.0.0:8080"
log_level = "info"
```

### Different Screen Resolutions

If you have a 4K Linux display (3840x2160):

```toml
# kvm-server.toml
screen_width = 3840
screen_height = 2160
client_addr = "192.168.1.100:8080"
```

### Debugging Issues

Enable verbose logging for troubleshooting:

**Server:**
```bash
RUST_LOG=debug ./target/release/kvm-server
```

**Client:**
```bash
RUST_LOG=debug ./target/release/kvm-client
```

This will show:
- Device detection and initialization
- Every mouse/keyboard input captured
- Every network packet sent/received
- Cursor position updates
- Event serialization/deserialization details

## Troubleshooting

### "No mouse or keyboard device found"
- Ensure input devices are properly connected
- Check Linux permissions: may need to run with `sudo` or add user to `input` group
- On macOS, you may need to grant Input Monitoring permissions

### "Connection refused" or network errors
- Verify the `client_addr` in server config matches the macOS machine's IP
- Verify the `bind_addr` in client config is the same port as server's `client_addr`
- Check firewall rules on both machines
- Ensure both machines can ping each other

### No events received on client
- Check that server is running and shows "KVM Edge Detection active"
- Verify config files have correct IP addresses
- Try moving mouse - should show debug logs if RUST_LOG=debug is set
- Check that devices were detected (look for "Device found" messages)

### Events received but not working
- Verify you have proper permissions on macOS for input injection
- Try running client with `sudo` on macOS (may be required for event injection)
- Increase log level to debug to see what events are being sent/received
