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
