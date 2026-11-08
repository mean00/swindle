// rpc_swdp_impl.rs — SWDP class command implementations
// Called by rpc_target_generated.rs

/// Initialize SWD
pub fn init() {
    crate::platform::swd::init();
}
