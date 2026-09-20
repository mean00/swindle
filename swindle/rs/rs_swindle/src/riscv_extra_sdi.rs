//! WCH CH32V0xx single-wire debug (SDI) — the Rust half of the optional SDI
//! feature.
//!
//! This file is one side of the seam described in
//! `swindle/include/bmp_riscv_extra.h`: it holds every SDI name the rest of the
//! firmware may use — the `sdi_scan` / `sdi_wire` monitor commands, the SDI DM
//! wrappers and the C FFI they sit on — and no generic file does. `lib.rs`
//! selects it as the `crate::riscv_extra` module when the `sdi` cargo feature is
//! on (`SWINDLE_WITH_SDI=ON`, wired in `swindle/rs/CMakeLists.txt`); with the
//! feature off `riscv_extra_sdi_stubs.rs` takes its place, so the option off and
//! this file deleted leaves a plain SWD/RVSWD probe whose `mon sdi_scan` reports
//! the failure it always reports, exactly like the C stubs behind it.
//!
//! A hosted (Blackmagic-app) build always takes this file: its C side
//! (`blackmagic_addon/hosted/remote_sdi_protocol.c`) always links the native leg,
//! and the RPC leaves it drives live in `hosted/rpc_host/remote_rpc.rs`. The
//! `mon sdi_wire` knobs go the same way, over the SDI class's `SET_WIRE` command,
//! because the tap they tune is the probe's, not this process's.

use crate::encoder::encoder;
use crate::freertos::os_detach;
use crate::parsing_util::convert_param_to_integer;

// bmplog! is a compile-time toggle: setup_log!(false) makes it a no-op in this
// file (see bmplogger.rs).
setup_log!(false);

/// The SDI part of the C FFI — the generic bindings are in `rn_bmp_cmd_c.rs`.
///
/// Declared next to the code that uses them, so that no SDI name is left in a
/// generic file. On a native build these are the SDI transport (`sdi_template.h`
/// plus the platform tap) and the `mon sdi_wire` knob setter
/// (`bmp_riscv_extra_sdi_c.cpp`); with `SWINDLE_WITH_SDI=OFF` the same symbols
/// come from the stubs (`bmp_riscv_extra_stubs.cpp` /
/// `bmp_riscv_extra_sdi_stubs_c.cpp`), so these declarations stay valid.
///
/// A hosted build declares none of them: the transport lives on the probe and
/// is reached over the class-I SDI RPC, so the local C leg is not linked at all
/// (its `bmp_set_sdi_wire_c` would only write knobs no tap on this PC reads).
mod c {
    #[cfg(not(feature = "hosted"))]
    unsafe extern "C" {
        pub fn bmp_set_sdi_wire_c(
            tbit_ns: cty::c_uint,
            low1_ns: cty::c_uint,
            low0_ns: cty::c_uint,
            sample_ns: cty::c_uint,
            no_trim: bool,
        );
        pub fn sdi_scan() -> bool;
        pub fn bmp_sdi_dm_reset_c() -> bool;
        pub fn bmp_sdi_dm_read_c(adr: u8, value: *mut cty::c_uint) -> bool;
        pub fn bmp_sdi_dm_write_c(adr: u8, value: cty::c_uint) -> bool;
    }
}

/// Perform a WCH CH32V0xx single-wire debug (SDI) scan.
///
/// Native firmware runs the SDI probe directly (`sdi_scan` in sdiTap);
/// hosted builds run the same stage-1 sequence in Rust over the class-I
/// RPC primitives (see the `hosted` variant below).
#[cfg(not(feature = "hosted"))]
pub fn sdi_scan() -> bool {
    unsafe { c::sdi_scan() }
}
/// Hosted variant of [`sdi_scan`].
///
/// The scan + full RISC-V target attach is orchestrated in C
/// (`blackmagic_addon/hosted/remote_sdi_protocol.c::bmda_sdi_scan2`, mirroring
/// `remote_rv_protocol.c::bmda_rvswd_scan2` for the RVSWD leg): clear the
/// blackmagic target list, gate on DMSTATUS, then hand `riscv_dmi_init()` a
/// real, header-typed `riscv_dmi_s` (designer = WCH) so the C RISC-V framework
/// (riscv32 + the CH32V0xx driver) discovers the hart — exactly like native
/// `sdi_scan()` attaches. Only the DM-access leaf functions live in Rust
/// (`remote_sdi_reset_rs` / `remote_sdi_dm_read_rs` / `remote_sdi_dm_write_rs`
/// in `hosted/rpc_host/remote_rpc.rs`); there is no Rust-side layout mirror of
/// `struct riscv_dmi` to keep in sync with riscv_debug.h.
#[cfg(feature = "hosted")]
pub fn sdi_scan() -> bool {
    unsafe extern "C" {
        fn bmda_sdi_scan2() -> bool;
    }
    unsafe { bmda_sdi_scan2() }
}
/// Override the SDI wire timing ('mon sdi_wire'), in ns.
///
/// `0` leaves that number as the SDI tap's own default: the cell, the LOW that
/// carries a 1, the LOW that carries a 0 and the sample point. `no_trim` leaves
/// the counts the model built instead of trimming them onto the cell. The SDI tap
/// re-calibrates on its next mode entry, so `mon sdi_scan` applies a new set.
#[cfg(not(feature = "hosted"))]
pub fn bmp_set_sdi_wire(tbit_ns: u32, low1_ns: u32, low0_ns: u32, sample_ns: u32, no_trim: bool) {
    unsafe { c::bmp_set_sdi_wire_c(tbit_ns, low1_ns, low0_ns, sample_ns, no_trim) }
}

/// Hosted variant of [`bmp_set_sdi_wire`].
///
/// The tap these numbers tune runs on the probe, so they are forwarded over the
/// class-I SDI RPC (`SDI.SET_WIRE` -> `rpc_sdi_impl::set_wire` ->
/// `bmp_set_sdi_wire_c` on the probe) instead of being written into this PC's
/// memory, where nothing would read them. A failed leg means the wire was left
/// as it was - no probe answering, or probe firmware from before this command
/// existed - so say so rather than let the echo above imply otherwise.
#[cfg(feature = "hosted")]
pub fn bmp_set_sdi_wire(tbit_ns: u32, low1_ns: u32, low0_ns: u32, sample_ns: u32, no_trim: bool) {
    if !crate::hosted::rpc_host::remote_rpc::remote_sdi_set_wire_rs(tbit_ns, low1_ns, low0_ns, sample_ns, no_trim) {
        bmpwarning!("SDI wire : the probe did not take the timing (no answer, or no SDI support)\n");
    }
}

/// Enter SDI debug mode: reset the SDI debug module (upload the transport on
/// RP2040, NRST pulse and the 0x7e/0x7d unlock on the LN bit-bang).
#[cfg(not(feature = "hosted"))]
pub fn sdi_dm_start() {
    unsafe {
        c::bmp_sdi_dm_reset_c();
    }
}

/// Read one SDI debug-module register. Returns `(ok, value)`.
#[cfg(not(feature = "hosted"))]
pub fn bmp_sdi_read(adr: u8) -> (bool, u32) {
    unsafe {
        let mut ret: u32 = 0;
        let ret_ptr: *mut u32 = &mut ret;

        let status: bool = c::bmp_sdi_dm_read_c(adr, ret_ptr);
        (status, ret)
    }
}

/// Write one SDI debug-module register.
#[cfg(not(feature = "hosted"))]
pub fn bmp_sdi_write(adr: u8, data: u32) -> bool {
    unsafe { c::bmp_sdi_dm_write_c(adr, data) }
}

/// Handle `mon sdi_scan` — probe for WCH CH32V0xx SDI devices.
pub fn _sdi_scan(_command: &str, _args: &[&str]) -> bool {
    bmplog!("sdi_scan:\n");
    // The scan may detach from the current target and attach a different one:
    // cached lines are target-specific and must be dropped.
    crate::mem_cache::invalidate();

    if !sdi_scan() {
        bmpwarning!("sdi_scan failed!\n");
        return false;
    }
    os_detach();
    encoder::reply_ok();
    true
}
/**
 * SDI bit-bang wire timing: `mon sdi_wire <tbit> <low1> <low0> <sample> [notrim]`,
 * all in ns, `0` keeping the tap's own default for that number.
 *
 * The four numbers §2.2 builds the transport from: the cell, the LOW that carries
 * a 1, the LOW that carries a 0, and the point a response bit is sampled at. The
 * tap calibrates on its next mode entry, so `mon sdi_wire ...` followed by
 * `mon sdi_scan` measures the new waveform: a sweep of the wire needs neither a
 * rebuild nor a reboot. `notrim` leaves the counts the model built instead of
 * trimming them onto the cell - the LOW/HIGH fractions stay exact, the cell does
 * not - which is what tells the two apart on the wire.
 */
pub fn _sdi_wire(_command: &str, args: &[&str]) -> bool {
    if args.len() < 4 {
        gdb_print!("usage: mon sdi_wire <tbit_ns> <low1_ns> <low0_ns> <sample_ns> [notrim], 0 = tap default\n");
        encoder::reply_ok();
        return true;
    }
    let mut ns = [0u32; 4];
    for (slot, arg) in ns.iter_mut().zip(args.iter()) {
        let (ok, value) = convert_param_to_integer(arg);
        if !ok {
            return false;
        }
        *slot = value;
    }
    let no_trim = args.len() > 4 && args[4] == "notrim";
    bmp_set_sdi_wire(ns[0], ns[1], ns[2], ns[3], no_trim);
    gdb_println!(
        "SDI wire : tbit=",
        ns[0],
        " low1=",
        ns[1],
        " low0=",
        ns[2],
        " sample=",
        ns[3],
        " (0 = the tap's default)"
    );
    if no_trim {
        gdb_print!("SDI wire : notrim (counts are the model's, untrimmed)\n");
    }
    gdb_print!("run 'mon sdi_scan' to calibrate against it\n");
    encoder::reply_ok();
    true
}
// EOF
