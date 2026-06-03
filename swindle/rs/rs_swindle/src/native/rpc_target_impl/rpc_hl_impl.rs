// rpc_hl_impl.rs — HL class command implementations
// Called by rpc_target_generated.rs

/// Check protocol version
pub fn check() -> u8 {
    3 // Force version 3
}
/// Alias for backward compatibility
pub fn check_version() -> u8 {
    check()
}

/// Get acceleration capabilities
pub fn accel() -> u32 {
    1 << 0 // ADIV5 supported
}

