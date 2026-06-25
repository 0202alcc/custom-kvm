use serde::{Serialize, Deserialize};
use std::fs;
use crate::error::Result;

/// Server-side configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,
    #[serde(default = "default_client_addr")]
    pub client_addr: String,
    #[serde(default = "default_screen_width")]
    pub screen_width: i32,
    #[serde(default = "default_screen_height")]
    pub screen_height: i32,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_bind_addr() -> String { "0.0.0.0:49152".to_string() }
fn default_client_addr() -> String { "127.0.0.1:49152".to_string() }
fn default_screen_width() -> i32 { 1920 }
fn default_screen_height() -> i32 { 1080 }
fn default_log_level() -> String { "debug".to_string() }

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            bind_addr: default_bind_addr(),
            client_addr: default_client_addr(),
            screen_width: default_screen_width(),
            screen_height: default_screen_height(),
            log_level: default_log_level(),
        }
    }
}

impl ServerConfig {
    /// Load server configuration from a TOML file
    /// Falls back to defaults if file doesn't exist
    pub fn load(path: &str) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(content) => {
                let config: ServerConfig = toml::from_str(&content)
                    .map_err(|e| crate::Error::ConfigError(format!("Failed to parse config: {}", e)))?;
                Ok(config)
            }
            Err(_) => {
                // File doesn't exist, return defaults
                Ok(ServerConfig::default())
            }
        }
    }
}

/// Client-side configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,
    #[serde(default = "default_server_addr")]
    pub server_addr: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_server_addr() -> String { "127.0.0.1:49152".to_string() }

impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig {
            bind_addr: default_bind_addr(),
            server_addr: default_server_addr(),
            log_level: default_log_level(),
        }
    }
}

impl ClientConfig {
    /// Load client configuration from a TOML file
    /// Falls back to defaults if file doesn't exist
    pub fn load(path: &str) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(content) => {
                let config: ClientConfig = toml::from_str(&content)
                    .map_err(|e| crate::Error::ConfigError(format!("Failed to parse config: {}", e)))?;
                Ok(config)
            }
            Err(_) => {
                // File doesn't exist, return defaults
                Ok(ClientConfig::default())
            }
        }
    }
}

pub fn default_config() -> ServerConfig {
    ServerConfig::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_addr, "0.0.0.0:49152");
        assert_eq!(config.client_addr, "127.0.0.1:49152");
        assert_eq!(config.screen_width, 1920);
        assert_eq!(config.screen_height, 1080);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.bind_addr, "0.0.0.0:49152");
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_server_config_load_nonexistent() {
        // Should return defaults if file doesn't exist
        let config = ServerConfig::load("/tmp/nonexistent_kvm_config_12345.toml").unwrap();
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_client_config_load_nonexistent() {
        // Should return defaults if file doesn't exist
        let config = ClientConfig::load("/tmp/nonexistent_kvm_config_client_12345.toml").unwrap();
        assert_eq!(config.log_level, "info");
    }
}
