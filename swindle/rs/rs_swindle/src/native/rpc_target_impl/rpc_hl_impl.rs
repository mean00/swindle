// rpc_hl_impl.rs — HL class command implementations
// Called by rpc_target_generated.rs

/// Get the RPC protocol version.
///
/// 0..=4 are the upstream Black Magic remote protocol numbers. This probe
/// answered 3 as long as its ADIv5 payloads used the upstream v3 layout
/// (big-endian ASCII hex), then 5 when the payloads moved to little-endian
/// ASCII hex. The payloads are CBOR now, which matches no upstream version at
/// all, so answering 6 keeps an upstream host rejecting us loudly instead of
/// silently mis-decoding. See doc/rpc.md.
pub fn check() -> u8 {
    6
}
/// Alias for backward compatibility
pub fn check_version() -> u8 {
    check()
}

/// Get acceleration capabilities
pub fn accel() -> u32 {
    1 << 0 // ADIV5 supported
}

