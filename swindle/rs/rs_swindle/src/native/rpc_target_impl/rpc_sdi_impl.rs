// rpc_sdi_impl.rs — SDI (WCH CH32V0xx single-wire) class, protocol side.
// Called by rpc_target_generated.rs, which holds the generated dispatch: the
// class itself stays in the RPC protocol (rpc_protocol.toml) like every other
// class, so this file is the adapter between that dispatch and the SDI seam
// (crate::riscv_extra) rather than SDI code. Everything the adapter calls is
// optional: with the `sdi` cargo feature off (SWINDLE_WITH_SDI=OFF) the seam is
// riscv_extra_sdi_stubs.rs, which answers 'reset ok, accesses fail' - the same
// thing the C stubs give the hosted side. See
// swindle/include/bmp_riscv_extra.h.

use crate::riscv_extra;

/// Enter SDI debug mode (upload PIO, NRST pulse, 0x7e/0x7d unlock)
pub fn reset() -> (bool, u32) {
    riscv_extra::sdi_dm_start();
    (true, 0)
}

/// Read one SDI DM register
pub fn dm_read(address: u32) -> (bool, u32) {
    riscv_extra::bmp_sdi_read(address as u8)
}

/// Write one SDI DM register
pub fn dm_write(address: u32, value: u32) -> (bool, u32) {
    let ok = riscv_extra::bmp_sdi_write(address as u8, value);
    (ok, 0)
}

/// Set the SDI bit-bang wire timing ('mon sdi_wire' knobs).
///
/// The numbers end up in the tap's globals, which it reads on its next mode
/// entry: a following `mon sdi_scan` is what applies them. Adding the command
/// to the protocol is what makes 'mon sdi_wire' reach the probe from a hosted
/// (Blackmagic-app) build, where there is no local tap to tune.
pub fn set_wire(tbit_ns: u32, low1_ns: u32, low0_ns: u32, sample_ns: u32, no_trim: bool) {
    riscv_extra::bmp_set_sdi_wire(tbit_ns, low1_ns, low0_ns, sample_ns, no_trim);
}
