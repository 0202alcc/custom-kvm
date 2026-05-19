use serde::{Serialize, Deserialize};
use std::fs;
use crate::error::Result;

/// Server-side configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Address to bind the UDP socket to
    pub bind_addr: String,
    /// Address of the client to send events to
    pub client_addr: String,
    /// Screen width in pixels
    pub screen_width: i32,
    /// Screen height in pixels
    pub screen_height: i32,
    /// Log level (debug, info, warn, error)
    pub log_level: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            bind_addr: "0.0.0.0:8080".to_string(),
            client_addr: "0.0.0.0:8080".to_string(),
            screen_width: 1920,
            screen_height: 1080,
            log_level: "info".to_string(),
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
    /// Address to bind the UDP socket to
    pub bind_addr: String,
    /// Log level (debug, info, warn, error)
    pub log_level: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig {
            bind_addr: "0.0.0.0:8080".to_string(),
            log_level: "info".to_string(),
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
        assert_eq!(config.bind_addr, "0.0.0.0:8080");
        assert_eq!(config.client_addr, "0.0.0.0:8080");
        assert_eq!(config.screen_width, 1920);
        assert_eq!(config.screen_height, 1080);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.bind_addr, "0.0.0.0:8080");
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
