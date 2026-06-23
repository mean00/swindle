//! Settings key constants.
//!
//! Defines the string keys used to store/retrieve configuration values
//! in the persistent settings store (`settings.rs`).

/// Maximum number of key-value entries in the settings store.
pub const MAX_SETTING_ENTRIES: usize = 4;
/// Maximum token (key) length in bytes.
pub const SW_TOKEN_SIZE: usize = 24;

/// Key for the RTT control block address.
pub const RTT_SETTING_KEY: &str = "RTT_ADDR";
/// Key for the RTT polling period in milliseconds.
pub const RTT_PERIOD_KEY: &str = "RTT_PERIOD";

/// Key for the reset holdoff duration.
pub const RESET_HOLDOFF_DURATION: &str = "RESET_HOLDOFF";
/// Key for the reset pulse duration.
pub const RESET_PULSE_DURATION: &str = "RESET_DURATION";
