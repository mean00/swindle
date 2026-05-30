// rpc_swdp_impl.rs — SWDP class command implementations
// Called by rpc_target_generated.rs

use crate::bmp;

/// Initialize SWD
pub fn init() {
    bmp::rpc_init_swd();
}