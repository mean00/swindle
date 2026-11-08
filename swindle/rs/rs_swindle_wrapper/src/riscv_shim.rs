//! C-to-Rust shim for the pure Rust RISC-V target drivers.
//!
//! This is the only code that names the concrete driver crates: it exports the probe entry
//! point the C target layer calls (`riscv_rust_target_probe`) and names one [`DriverSlot`] per
//! family, in probe priority order. A slot is zero-sized: the driver instance of the family
//! that probed goes into the one shared `DRIVER_STORAGE` cell, tagged with the family name, so
//! a family that never probes costs no RAM. Everything after a probe — turning the driver into
//! the C `target_s`, the flash/software-breakpoint/CRC callbacks and the one active driver — is
//! architecture code in `rsbmp::riscv::driver_registry`, next to the `RustRiscvTargetDriver`
//! trait (the driver crates depend on `rsbmp`, so naming them from there would be a cycle).
//!
//! The driver crates are always built, so unlike [`crate::arm_probe_chain`] this module is not
//! feature gated — except for the RP2350, whose two drivers are enabled together by
//! `rust_rp2350` (see [`crate::arm_probe_chain`] for the Arm half).

#![allow(unsafe_op_in_unsafe_fn)]

use rs_ch32v00x::Ch32v00xDriver;
use rs_ch32v123xx::Ch32v123xxDriver;
use rsbmp::riscv::driver_registry::probe as riscv_probe;
use rsbmp::target::{DriverSlot, DriverTag, Target, TargetView, jep106};

/// Storage handle for the CH32V00x/CH32V006 driver (probed first for a WCH part).
static CH32V00X_DRIVER: DriverSlot<Ch32v00xDriver> = DriverSlot::new();

/// Storage handle for the CH32V1xx/CH32V2xx/CH32V3xx driver (probed second for a WCH part).
static CH32V123XX_DRIVER: DriverSlot<Ch32v123xxDriver> = DriverSlot::new();

/// Storage handle for the RP2350 Hazard3 driver (probed first for a Raspberry Pi part).
#[cfg(feature = "rust_rp2350")]
static RP2350_DRIVER: DriverSlot<rs_rp2350_riscv::Rp2350RiscvDriver> = DriverSlot::new();

/// The WCH drivers, in probe order.
unsafe fn wch_probe(target: *mut Target) -> bool {
    riscv_probe(
        target,
        &CH32V00X_DRIVER,
        DriverTag("CH32V00x"),
        Ch32v00xDriver::probe,
    ) || riscv_probe(
        target,
        &CH32V123XX_DRIVER,
        DriverTag("CH32V123xx"),
        Ch32v123xxDriver::probe,
    )
}

/// The RP2350 Hazard3 driver, when it is built into this firmware.
unsafe fn rp2350_probe(target: *mut Target) -> bool {
    #[cfg(feature = "rust_rp2350")]
    {
        riscv_probe(
            target,
            &RP2350_DRIVER,
            DriverTag("RP2350 Hazard3"),
            rs_rp2350_riscv::driver::probe,
        )
    }
    #[cfg(not(feature = "rust_rp2350"))]
    {
        let _ = target;
        false
    }
}

/// RISC-V probe entry point called from `riscv32_silicon_probe_c` in `bmp_helpers.cpp`.
///
/// Two kinds of core can end up here: a WCH part, or the Hazard3 cores of an RP2350. The
/// family the designer code points at is therefore tried first — WCH parts must not be fed to
/// the RP2350 probe and vice versa — and the other one is the fallback for a core behind a
/// designer code that is not known here. The dispatcher
/// (`rsbmp::riscv::driver_registry`) keeps whichever driver recognized the target.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn riscv_rust_target_probe(target: *mut Target) -> bool {
    if TargetView::new(target).designer_code() == jep106::RASPBERRY {
        rp2350_probe(target) || wch_probe(target)
    } else {
        wch_probe(target) || rp2350_probe(target)
    }
}

