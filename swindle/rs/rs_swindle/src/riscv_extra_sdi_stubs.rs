//! No-SDI twin of `riscv_extra_sdi.rs` — the default definitions of the SDI
//! seam for a build without the SDI feature.
//!
//! `lib.rs` selects this file as the `crate::riscv_extra` module when the `sdi`
//! cargo feature is off (`SWINDLE_WITH_SDI=OFF`, wired in
//! `swindle/rs/CMakeLists.txt`). It keeps every reference to the SDI entry
//! points linked — the `mon sdi_scan` / `mon sdi_wire` commands and the native
//! RPC SDI class — with a body that fails or does nothing, so the firmware is a
//! plain SWD/RVSWD probe and the RISC-V stack is untouched. Same role as
//! `swindle/src/template/bmp_riscv_extra_stubs.cpp` on the C side (which is what
//! answers the C-level `sdi_scan()`/`bmp_sdi_dm_*_c()` calls this file no longer
//! makes).

use crate::encoder::encoder;

// bmplog! is a compile-time toggle: setup_log!(false) makes it a no-op in this
// file (see bmplogger.rs).
setup_log!(false);

/// No transport, so no probe: `mon sdi_scan` reports the failure it already
/// knows how to report. Unused here by construction (the handler below fails
/// without asking), kept so the seam keeps the same names in both builds.
#[allow(dead_code)]
pub fn sdi_scan() -> bool {
    false
}

/// No tap to re-tune: the wire knobs of `mon sdi_wire` are accepted and dropped
/// (the C-side setter is a no-op in this build too). Unused here by
/// construction: nothing in this build calls it.
#[allow(dead_code)]
pub fn bmp_set_sdi_wire(
    _tbit_ns: u32,
    _low1_ns: u32,
    _low0_ns: u32,
    _sample_ns: u32,
    _no_trim: bool,
) {
}

/// No SDI debug module to reset.
pub fn sdi_dm_start() {}

/// No SDI debug module: every access fails.
pub fn bmp_sdi_read(_adr: u8) -> (bool, u32) {
    (false, 0)
}

/// No SDI debug module: every access fails.
pub fn bmp_sdi_write(_adr: u8, _data: u32) -> bool {
    false
}

/// Handle `mon sdi_scan` — same output as the real handler hitting a dead
/// transport, which is what a build without SDI has.
pub fn _sdi_scan(_command: &str, _args: &[&str]) -> bool {
    bmplog!("sdi_scan:\n");
    // The scan may detach from the current target and attach a different one:
    // cached lines are target-specific and must be dropped.
    crate::mem_cache::invalidate();
    bmpwarning!("sdi_scan failed!\n");
    false
}

/// Handle `mon sdi_wire` — say why nothing happened instead of echoing numbers
/// that no tap would read.
pub fn _sdi_wire(_command: &str, _args: &[&str]) -> bool {
    gdb_print!("SDI support is not built in (SWINDLE_WITH_SDI=OFF)\n");
    encoder::reply_ok();
    true
}
// EOF
