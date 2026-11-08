//! Probe chain of the Cortex-M target drivers: which driver gets to look at a target.
//!
//! `cortexm.c` used to hold this as a `switch (target->designer_code)` with a `PROBE()` per
//! candidate driver. It is a table here now, in the same order, so that adding a driver is one
//! line next to its family and the C target layer only makes one call
//! ([`cortexm_probe_chain`]).
//!
//! This is also the only code that names the concrete Rust driver crates: a ported family is
//! a local probe function that runs `rsbmp::arm::driver_registry::probe` on its driver, and the
//! family handle it installs through lives here too. A handle is zero-sized — the driver
//! instance of the family that probed goes into the one shared `DRIVER_STORAGE` cell, tagged
//! with the family name, so a family that never probes costs no RAM. Everything after a probe —
//! turning the driver into the C `target_s`, the flash/attach/CRC callbacks and the one active
//! driver — is architecture code in `rsbmp::arm::driver_registry`, next to the
//! `RustArmTargetDriver` trait (the driver crates depend on `rsbmp`, so naming them from there
//! would be a cycle).
//!
//! A family whose C driver is still compiled keeps its C probe: the externs live in
//! [`c_probes`]. When the Rust driver of a ported family is enabled (`cargo` features
//! `rust_stm32f1`, `rust_stm32f4`, `rust_stm32g0`, `rust_rp2040`, `rust_rp2350`, set from
//! `SWINDLE_RUST_ARM_TARGETS`) the local probe function of the same name takes over and the C
//! one becomes dead code the linker drops; when it is disabled, the `use` of the alias below
//! wires the C entry point into the table instead. This is the ARM sibling of
//! [`crate::riscv_shim`].

#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::c_void;
use rsbmp::arm::driver_registry::probe as arm_probe;
use rsbmp::target::{DriverSlot, DriverTag, Target, TargetView, jep106, target_check_error};

// Fallback stubs when a Rust driver feature is disabled.
#[cfg(not(feature = "rust_stm32f1"))]
unsafe fn stm32f1_probe(_target: *mut Target) -> bool { false }
#[cfg(not(feature = "rust_stm32f1"))]
unsafe fn gd32f1_probe(_target: *mut Target) -> bool { false }
#[cfg(not(feature = "rust_stm32f1"))]
unsafe fn at32f40x_probe(_target: *mut Target) -> bool { false }
#[cfg(not(feature = "rust_stm32f1"))]
unsafe fn mm32f3xx_probe(_target: *mut Target) -> bool { false }
#[cfg(not(feature = "rust_stm32f1"))]
unsafe fn mm32l0xx_probe(_target: *mut Target) -> bool { false }
#[cfg(not(feature = "rust_stm32f1"))]
unsafe fn ch32f1_probe(_target: *mut Target) -> bool { false }
#[cfg(not(feature = "rust_stm32f4"))]
unsafe fn stm32f4_probe(_target: *mut Target) -> bool { false }
#[cfg(not(feature = "rust_stm32f4"))]
unsafe fn gd32f4_probe(_target: *mut Target) -> bool { false }
#[cfg(not(feature = "rust_stm32g0"))]
unsafe fn stm32g0_probe(_target: *mut Target) -> bool { false }
#[cfg(not(feature = "rust_rp2040"))]
unsafe fn rp2040_probe(_target: *mut Target) -> bool { false }
#[cfg(not(feature = "rust_rp2350"))]
unsafe fn rp2350_probe(_target: *mut Target) -> bool { false }
#[cfg(not(feature = "rust_nrf51"))]
unsafe fn nrf51_probe(_target: *mut Target) -> bool { false }
#[cfg(not(feature = "rust_nrf54l"))]
unsafe fn nrf54l_probe(_target: *mut Target) -> bool { false }
#[cfg(not(feature = "rust_nrf91"))]
unsafe fn nrf91_probe(_target: *mut Target) -> bool { false }

/// Probe entry point of one Cortex-M driver: `true` when it claimed the target.
type ProbeFn = unsafe fn(target: *mut Target) -> bool;

// ---------------------------------------------------------------------------
// Rust driver storage
// ---------------------------------------------------------------------------

// One handle per driver family: a probe installs at most one driver and the dispatcher
// (`rsbmp::arm::driver_registry`) keeps the pointer to whichever installed last. The handles
// are zero-sized — the drivers themselves share the one `DRIVER_STORAGE` cell, which is why a
// firmware that can probe seven families pays for one driver, not seven.
#[cfg(feature = "rust_stm32f1")]
static STM32F1_DRIVER: DriverSlot<rs_stm32f1::F1Driver> = DriverSlot::new();
#[cfg(feature = "rust_stm32f4")]
static STM32F4_DRIVER: DriverSlot<rs_stm32f4::F4Driver> = DriverSlot::new();
#[cfg(feature = "rust_stm32g0")]
static STM32G0_DRIVER: DriverSlot<rs_stm32g0::G0Driver> = DriverSlot::new();
#[cfg(feature = "rust_rp2040")]
static RP2040_DRIVER: DriverSlot<rs_rp2040::Rp2040Driver> = DriverSlot::new();
#[cfg(feature = "rust_rp2350")]
static RP2350_DRIVER: DriverSlot<rs_rp2350_arm::Rp2350Driver> = DriverSlot::new();
#[cfg(feature = "rust_nrf51")]
static NRF51_DRIVER: DriverSlot<rs_nrf::Nrf51Driver> = DriverSlot::new();
#[cfg(feature = "rust_nrf54l")]
static NRF54L_DRIVER: DriverSlot<rs_nrf::Nrf54lDriver> = DriverSlot::new();
#[cfg(feature = "rust_nrf91")]
static NRF91_DRIVER: DriverSlot<rs_nrf::Nrf91Driver> = DriverSlot::new();

// ---------------------------------------------------------------------------
// Probes of the families that are Rust
// ---------------------------------------------------------------------------

/// Probe the STM32F1/F0/F3 family with the Rust driver (else C `stm32f1_probe`).
#[cfg(feature = "rust_stm32f1")]
unsafe fn stm32f1_probe(target: *mut Target) -> bool {
    arm_probe(
        target,
        &STM32F1_DRIVER,
        DriverTag("STM32F1"),
        rs_stm32f1::families::f1::probe_stm32,
    )
}

/// Probe the GD32F1/F2/F3/E230/E5 family with the Rust driver (else C `gd32f1_probe`).
#[cfg(feature = "rust_stm32f1")]
unsafe fn gd32f1_probe(target: *mut Target) -> bool {
    arm_probe(
        target,
        &STM32F1_DRIVER,
        DriverTag("STM32F1"),
        rs_stm32f1::families::f1::probe_gd32,
    )
}

/// Probe the Artery AT32F40x line with the Rust driver (else C `at32f40x_probe`).
#[cfg(feature = "rust_stm32f1")]
unsafe fn at32f40x_probe(target: *mut Target) -> bool {
    arm_probe(
        target,
        &STM32F1_DRIVER,
        DriverTag("STM32F1"),
        rs_stm32f1::families::f1_clones::probe_at32f40x,
    )
}

/// Probe the MM32F3 line with the Rust driver (else C `mm32f3xx_probe`).
#[cfg(feature = "rust_stm32f1")]
unsafe fn mm32f3xx_probe(target: *mut Target) -> bool {
    arm_probe(
        target,
        &STM32F1_DRIVER,
        DriverTag("STM32F1"),
        rs_stm32f1::families::f1_clones::probe_mm32f3,
    )
}

/// Probe the MM32L0 line with the Rust driver (else C `mm32l0xx_probe`).
#[cfg(feature = "rust_stm32f1")]
unsafe fn mm32l0xx_probe(target: *mut Target) -> bool {
    arm_probe(
        target,
        &STM32F1_DRIVER,
        DriverTag("STM32F1"),
        rs_stm32f1::families::f1_clones::probe_mm32l0,
    )
}

/// Probe the CH32F103 with the Rust driver (else C `ch32f1_probe`).
#[cfg(feature = "rust_stm32f1")]
unsafe fn ch32f1_probe(target: *mut Target) -> bool {
    arm_probe(
        target,
        &STM32F1_DRIVER,
        DriverTag("STM32F1"),
        rs_stm32f1::families::ch32f1::probe,
    )
}

/// Probe the STM32F2/F4/F7 family with the Rust driver (else C `stm32f4_probe`).
#[cfg(feature = "rust_stm32f4")]
unsafe fn stm32f4_probe(target: *mut Target) -> bool {
    arm_probe(
        target,
        &STM32F4_DRIVER,
        DriverTag("STM32F4"),
        rs_stm32f4::probe_stm32,
    )
}

/// Probe the GD32F4 family with the Rust driver (else C `gd32f4_probe`).
#[cfg(feature = "rust_stm32f4")]
unsafe fn gd32f4_probe(target: *mut Target) -> bool {
    arm_probe(
        target,
        &STM32F4_DRIVER,
        DriverTag("STM32F4"),
        rs_stm32f4::probe_gd32,
    )
}

/// Probe the STM32G0/C0/U0 family with the Rust driver (else C `stm32g0_probe`).
#[cfg(feature = "rust_stm32g0")]
unsafe fn stm32g0_probe(target: *mut Target) -> bool {
    arm_probe(
        target,
        &STM32G0_DRIVER,
        DriverTag("STM32G0"),
        rs_stm32g0::driver::probe,
    )
}

/// Probe the RP2040 with the Rust driver (else C `rp2040_probe`).
#[cfg(feature = "rust_rp2040")]
unsafe fn rp2040_probe(target: *mut Target) -> bool {
    arm_probe(
        target,
        &RP2040_DRIVER,
        DriverTag("RP2040"),
        rs_rp2040::driver::probe,
    )
}

/// The ADIv5 rescue DP of an RP2040/RP2350 (C `rp2040_rescue_probe`).
///
/// Not a target probe: `adiv5.c` calls it with a fabricated AP when the DP answers but has no
/// AP, i.e. when the chip's cores are wedged. It is exported here because it is the same
/// cargo feature that keeps the C `rp2040.c` out of the build.
#[cfg(feature = "rust_rp2040")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rp2040_rescue_probe(ap: *mut c_void) -> bool {
    rs_rp2040::rescue::probe(ap.cast())
}

/// Probe the RP2350 Cortex-M33 cores with the Rust driver (else C `rp2350_probe`).
///
/// The Hazard3 RISC-V cores of the same chip are probed from `crate::riscv_shim` with
/// `rs_rp2350_riscv`, which is enabled by the same cargo feature.
#[cfg(feature = "rust_rp2350")]
unsafe fn rp2350_probe(target: *mut Target) -> bool {
    arm_probe(
        target,
        &RP2350_DRIVER,
        DriverTag("RP2350"),
        rs_rp2350_arm::driver::probe,
    )
}

/// Probe the nRF51 / nRF52 family with the Rust driver (else C `nrf51_probe`).
#[cfg(feature = "rust_nrf51")]
unsafe fn nrf51_probe(target: *mut Target) -> bool {
    arm_probe(
        target,
        &NRF51_DRIVER,
        DriverTag("NRF51"),
        rs_nrf::nrf51::probe,
    )
}

/// The ADIv5 nRF52 CTRL-AP recovery probe (C `nrf51_ctrl_ap_probe`).
#[cfg(feature = "rust_nrf51")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nrf51_ctrl_ap_probe(ap: *mut c_void) -> bool {
    rs_nrf::ctrl_ap::nrf51_ctrl_ap_probe(ap.cast())
}

/// Probe the nRF54L family with the Rust driver (else C `nrf54l_probe`).
#[cfg(feature = "rust_nrf54l")]
unsafe fn nrf54l_probe(target: *mut Target) -> bool {
    arm_probe(
        target,
        &NRF54L_DRIVER,
        DriverTag("NRF54L"),
        rs_nrf::nrf54l::probe,
    )
}

/// The ADIv5 nRF54L CTRL-AP recovery probe (C `nrf54l_ctrl_ap_probe`).
#[cfg(feature = "rust_nrf54l")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nrf54l_ctrl_ap_probe(ap: *mut c_void) -> bool {
    rs_nrf::ctrl_ap::nrf54l_ctrl_ap_probe(ap.cast())
}

/// Probe the nRF91 family with the Rust driver (else C `nrf91_probe`).
#[cfg(feature = "rust_nrf91")]
unsafe fn nrf91_probe(target: *mut Target) -> bool {
    arm_probe(
        target,
        &NRF91_DRIVER,
        DriverTag("NRF91"),
        rs_nrf::nrf91::probe,
    )
}

// ---------------------------------------------------------------------------
// Probes that are still C (Removed - all legacy C target drivers eliminated)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    /// Add the probe-side target options to `options` (`TOPT_INHIBIT_NRST` when the user
    /// asked for no reset). Kept in C: it reads the probe session state.
    fn bmp_tweak_options(options: *mut u32);
}

// ---------------------------------------------------------------------------
// Target identification constants (mirror `cortex.h`)
// ---------------------------------------------------------------------------

// A device that reports ARM as its designer reports the ROM table of its core as its part id,
// so these are core part numbers and not vendor ones.
const CORTEX_M0_ROM: u32 = 0x471;
const CORTEX_M3_ROM: u32 = 0x4c3;
const CORTEX_M4_ROM: u32 = 0x4c4;
const CORTEX_M23_ROM: u32 = 0x4cb;

// ---------------------------------------------------------------------------
// The chain
// ---------------------------------------------------------------------------

/// Run one probe with the semantics of the C `PROBE()` macro: a hit ends the chain after the
/// probe-side option tweaks, a miss clears the sticky target error so the next probe starts
/// from a clean slate.
unsafe fn step(target: *mut Target, probe: ProbeFn) -> bool {
    if probe(target) {
        let mut options = TargetView::new(target).options();
        bmp_tweak_options(&mut options);
        TargetView::new(target).set_options(options);
        true
    } else {
        target_check_error(target);
        false
    }
}

/// Probe the Cortex-M target `target` with the driver of its family.
///
/// Restricted exclusively to active Rust target drivers. Returns `false` when no driver claimed
/// the target.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cortexm_probe_chain(target: *mut Target) -> bool {
    if target.is_null() {
        return false;
    }

    match TargetView::new(target).designer_code() {
        jep106::GIGADEVICE => step(target, gd32f1_probe) || step(target, gd32f4_probe),
        jep106::STM => {
            step(target, stm32f1_probe)
                || step(target, stm32f4_probe)
                || step(target, stm32g0_probe)
        }
        jep106::RASPBERRY => step(target, rp2040_probe) || step(target, rp2350_probe),
        jep106::NORDIC => {
            step(target, nrf51_probe) || step(target, nrf54l_probe) || step(target, nrf91_probe)
        }
        jep106::ARM_CHINA => step(target, mm32f3xx_probe),
        jep106::ARM => probe_arm_rom_table(target, TargetView::new(target).part_id()),
        _ => false,
    }
}

/// Probes of an ARM-designer target, keyed on the ROM table part number of its core.
unsafe fn probe_arm_rom_table(target: *mut Target, part_id: u32) -> bool {
    match part_id {
        CORTEX_M3_ROM => {
            step(target, ch32f1_probe)
                || step(target, stm32f1_probe)
                || step(target, mm32f3xx_probe)
        }
        CORTEX_M0_ROM => step(target, mm32l0xx_probe),
        CORTEX_M4_ROM => step(target, at32f40x_probe),
        CORTEX_M23_ROM => step(target, gd32f1_probe),
        _ => false,
    }
}
