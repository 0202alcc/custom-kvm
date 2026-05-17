use crate::error::Result;

/// Initialize the logging system with the specified log level
///
/// This function sets up env_logger to read the RUST_LOG environment variable.
/// If RUST_LOG is not set, it defaults to the provided level.
///
/// # Arguments
/// * `default_level` - Default log level if RUST_LOG is not set (e.g., "info", "debug")
///
/// # Example
/// ```ignore
/// init_logging("info")?;
/// log::info!("This is an info message");
/// ```
pub fn init_logging(default_level: &str) -> Result<()> {
    // Set default log level if RUST_LOG is not already set
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", default_level);
    }

    env_logger::builder()
        .format_timestamp_millis()
        .try_init()
        .map_err(|e| crate::Error::ConfigError(format!("Failed to initialize logging: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_logging() {
        // This test just verifies that init_logging doesn't panic
        // In real usage, it would only be called once
        let _ = init_logging("debug");
    }
}
