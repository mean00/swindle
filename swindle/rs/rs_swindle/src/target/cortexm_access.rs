//! Cortex-M target access for pure Rust ARM target drivers.
//!
//! This is the ARM counterpart of [`crate::riscv::RiscvHart`]: the small, driver-facing
//! API the drivers are written against (`mem_read32`, `mem_write32`, sized and aligned
//! writes, `check_error`, `reset`, `cortexm_attach`, `cortexm_detach`). It is a thin
//! wrapper over the C `target_s` hooks, so a Rust driver goes through exactly the same
//! memory/reset paths as the C drivers it replaces.
//!
//! Unlike `RiscvHart` there is no DM/hart concept: a Cortex-M target has a single core,
//! reached through its access port, and the C layer already owns the `target_s` handle.

use super::{Target, TargetView};
use super::target::bmp_get_temporary_buffer;
use super::driver::{HaltReason, TargetError};

/// Program the flash with 8-bit bus beats (`mon psize x8` on the F4-style controllers).
pub const ALIGN_8BIT: u8 = 0;
/// Program the flash with 16-bit bus beats (STM32F1/G0/F4 flash controllers latch per halfword).
pub const ALIGN_16BIT: u8 = 1;
/// Program the flash with 32-bit bus beats (GD32/AT32/MM32 parts that accept word programming).
pub const ALIGN_32BIT: u8 = 2;
/// Program the flash with 64-bit bus beats (`mon psize x64` on the F4-style controllers).
pub const ALIGN_64BIT: u8 = 3;

/// How many times [`CortexMAccess::with_halted`] polls before it gives up waiting for the
/// core to report halted. A halt request that lands takes a handful of bus cycles.
const HALT_POLL_ATTEMPTS: usize = 1000;

/// Access to one Cortex-M target.
#[derive(Debug, Clone, Copy)]
pub struct CortexMAccess {
    target: *mut Target,
}

impl CortexMAccess {
    /// Wrap a target handle.
    pub fn new(target: *mut Target) -> Self {
        Self { target }
    }

    /// The wrapped target handle.
    pub fn target(&self) -> *mut Target {
        self.target
    }

    /// `TargetC::cpuid` — the Cortex-M CPUID register read during `cortexm_probe()`.
    pub fn cpuid(&self) -> u32 {
        TargetView::new(self.target).cpuid()
    }

    /// `TargetC::part_id` — the part number the probe identified.
    pub fn part_id(&self) -> u32 {
        TargetView::new(self.target).part_id()
    }

    /// IDR of the access port: the top nibble is the AP/core revision, which tells
    /// genuine STM32F1 parts (Core rev 0/1) from clones (Core rev 2+).
    pub fn ap_idr(&self) -> u32 {
        if self.target.is_null() {
            return 0;
        }
        let ap = crate::arm::cortex::ap(self.target);
        if ap.is_null() {
            return 0;
        }
        unsafe { (*ap).idr }
    }

    /// Install the MM32 broadcast-write mode on target AP.
    pub fn install_mm32l0_broadcast_write(&self) {
        let ap = crate::arm::cortex::ap(self.target);
        if !ap.is_null() {
            unsafe {
                (*ap).flags |= crate::arm::adi::regs::ADIV5_AP_FLAG_BROADCAST_LANES;
            }
        }
    }

    /// DP protocol version (`0` = DPv0, `1` = DPv1, `2` = DPv2).
    pub fn dp_version(&self) -> u8 {
        let ap = crate::arm::cortex::ap(self.target);
        crate::arm::c_seam::dp_version(ap) as u8
    }

    /// Target part number the DP identified (e.g. `ID_NRF54L`, `ID_NRF91`).
    pub fn dp_target_partno(&self) -> u16 {
        let ap = crate::arm::cortex::ap(self.target);
        crate::arm::c_seam::dp_target_partno(ap) as u16
    }

    /// Read a 32-bit word, without checking for a bus fault (matches `target_mem32_read32`).
    #[inline(never)]
    pub fn read32(&self, addr: u32) -> u32 {
        let mut buf = [0u8; 4];
        let _ = self.mem_read(&mut buf, addr);
        u32::from_le_bytes(buf)
    }

    /// Write a 32-bit word (matches `target_mem32_write32`).
    #[inline(never)]
    pub fn write32(&self, addr: u32, value: u32) {
        let _ = self.mem_write(addr, &value.to_le_bytes());
    }

    /// Read a 16-bit halfword.
    #[inline(never)]
    pub fn read16(&self, addr: u32) -> u16 {
        let mut buf = [0u8; 2];
        let _ = self.mem_read(&mut buf, addr);
        u16::from_le_bytes(buf)
    }

    /// Write a 16-bit halfword (matches `target_mem32_write16`).
    #[inline(never)]
    pub fn write16(&self, addr: u32, value: u16) {
        let _ = self.mem_write(addr, &value.to_le_bytes());
    }

    /// Read `dest.len()` bytes from `addr`.
    #[inline(never)]
    pub fn mem_read(&self, dest: &mut [u8], addr: u32) {
        if self.target.is_null() {
            dest.fill(0);
            return;
        }
        let _ = crate::arm::cortexm::mem_read(self.target, dest, addr);
    }

    /// Write `src` to `addr`.
    #[inline(never)]
    pub fn mem_write(&self, addr: u32, src: &[u8]) {
        if self.target.is_null() || src.is_empty() {
            return;
        }
        let _ = crate::arm::cortexm::mem_write(self.target, addr, src);
    }

    /// Read `dest.len()` bytes from `addr`, returning a Result.
    #[inline(never)]
    pub fn try_mem_read(&self, dest: &mut [u8], addr: u32) -> Result<(), TargetError> {
        if self.target.is_null() {
            dest.fill(0);
            return Err(TargetError::NotAttached);
        }
        crate::arm::cortexm::mem_read(self.target, dest, addr)
    }

    /// Write `src` to `addr`, returning a Result.
    #[inline(never)]
    pub fn try_mem_write(&self, addr: u32, src: &[u8]) -> Result<(), TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        if src.is_empty() {
            return Ok(());
        }
        crate::arm::cortexm::mem_write(self.target, addr, src)
    }

    /// Flash programming write with the bus beat width forced to 8, 16, 32 or 64 bits
    /// (`cortexm_mem_write_aligned()`, which returns non-zero on a bus fault);
    /// `align` must be one of the `ALIGN_*BIT` values.
    #[inline(never)]
    pub fn write_aligned(&self, addr: u32, src: &[u8], align: u8) -> bool {
        if self.target.is_null() || src.is_empty() {
            return false;
        }
        crate::arm::cortexm::mem_write_aligned(
            self.target,
            addr,
            src.as_ptr().cast(),
            src.len(),
            u32::from(align),
        ) == 0
    }

    /// True if the last memory access raised a fault (`target_check_error()`).
    #[inline(never)]
    pub fn check_error(&self) -> bool {
        if self.target.is_null() {
            return true;
        }
        crate::arm::cortexm::check_error(self.target)
    }

    /// Read `buf.len()` bytes of core register `reg` (`target_s::reg_read`), which is how
    /// a stub run reads its result back; `false` if it could not be read.
    pub fn reg_read(&self, reg: u32, buf: &mut [u8]) -> bool {
        if self.target.is_null() || buf.is_empty() {
            return false;
        }
        crate::arm::cortexm::reg_read(self.target, reg, buf.as_mut_ptr().cast(), buf.len()) == buf.len()
    }

    /// Read the whole register file in one go (`target_regs_read()`).
    pub fn regs_read(&self, data: &mut [u8]) -> bool {
        if self.target.is_null() || data.is_empty() {
            return false;
        }
        crate::arm::cortexm::regs_read_hook(self.target, data.as_mut_ptr().cast());
        true
    }

    /// Write the whole register file in one go (`target_regs_write()`), the layout being
    /// the Cortex-M one (`r0..r12`, `sp`, `lr`, `pc`, `xpsr`, `msp`).
    pub fn regs_write(&self, data: &[u8]) -> bool {
        if self.target.is_null() || data.is_empty() {
            return false;
        }
        crate::arm::cortexm::regs_write_hook(self.target, data.as_ptr().cast());
        true
    }

    /// Whether the core is halted (`target_s::halt_poll`).
    pub fn halted(&self) -> bool {
        if self.target.is_null() {
            return false;
        }
        self.halt_poll().0 != HaltReason::Running
    }

    /// Poll the core's execution state (`target_s::halt_poll`), returning the halt reason and
    /// the watchpoint address when the core stopped on a watchpoint.
    pub fn halt_poll(&self) -> (HaltReason, u32) {
        if self.target.is_null() {
            return (HaltReason::Running, 0);
        }
        crate::arm::cortexm::halt_poll(self.target)
    }

    /// Ask the core to halt (`target_s::halt_request`).
    pub fn halt_request(&self) {
        if !self.target.is_null() {
            crate::arm::cortexm::halt_request(self.target);
        }
    }

    /// Run `work` with the core halted, resuming it afterwards when it was running.
    ///
    /// A core executing from external flash faults while the flash controller is being
    /// reconfigured, which is why the C drivers only do that from their attach hook (where
    /// the core is halted already). A probe that has to read the flash geometry uses this
    /// so a plain scan leaves a running target running.
    pub fn with_halted<T>(&self, work: impl FnOnce(&Self) -> T) -> T {
        let was_running = !self.halted();
        if was_running {
            self.halt_request();
            // Bounded wait: once the request lands the core reports halted within a few
            // bus cycles, and a timeout means it never will (reset held, no power, ...).
            for _ in 0..HALT_POLL_ATTEMPTS {
                if self.halted() {
                    break;
                }
            }
        }
        let result = work(self);
        if was_running {
            self.halt_resume(false);
        }
        result
    }

    /// Resume the halted core (`target_halt_resume()`), optionally single-stepping.
    pub fn halt_resume(&self, step: bool) {
        if !self.target.is_null() {
            crate::arm::cortexm::halt_resume(self.target, step);
        }
    }

    /// Run a Cortex-M stub ([`crate::arm::stub::run_stub()`], the Rust port of
    /// `cortexm_run_stub()`): set `r0..r3`, continue at `loadaddr` and report success.
    ///
    /// `true` means the stub published `STUB_EXIT_OK` (`bkpt #1`) at its trap instruction once
    /// it was done; `false` covers a failed run (bad register write, timeout, a halt that is not
    /// a `bkpt`) as well as a stub that reported failure. Stubs that return a value have
    /// [`Self::run_stub_result`].
    pub fn run_stub(&self, loadaddr: u32, r0: u32, r1: u32, r2: u32, r3: u32) -> bool {
        crate::arm::stub::run_stub(self, loadaddr, r0, r1, r2, r3)
    }

    /// Run a Cortex-M stub with an explicit stack pointer.
    pub fn run_stub_with_stack(
        &self,
        loadaddr: u32,
        stack: u32,
        r0: u32,
        r1: u32,
        r2: u32,
        r3: u32,
    ) -> bool {
        crate::arm::stub::run_stub_with_stack(self, loadaddr, stack, r0, r1, r2, r3)
    }

    /// Run a Cortex-M stub and return the value it published in `result_reg` (usually
    /// `arm::stub::STUB_RESULT_REG`, i.e. `r1`); `None` means the stub reported failure.
    pub fn run_stub_result(
        &self,
        loadaddr: u32,
        result_reg: u32,
        r0: u32,
        r1: u32,
        r2: u32,
        r3: u32,
    ) -> Option<u32> {
        crate::arm::stub::run_stub_result(self, loadaddr, result_reg, r0, r1, r2, r3)
    }

    /// Run a Cortex-M stub with an explicit stack pointer and return its result register.
    pub fn run_stub_with_stack_result(
        &self,
        loadaddr: u32,
        stack: u32,
        result_reg: u32,
        r0: u32,
        r1: u32,
        r2: u32,
        r3: u32,
    ) -> Option<u32> {
        crate::arm::stub::run_stub_with_stack_result(self, loadaddr, stack, result_reg, r0, r1, r2, r3)
    }

    /// Reset the target (`target_s::reset`).
    pub fn reset(&self) {
        if !self.target.is_null() {
            crate::arm::cortexm::reset(self.target);
        }
    }

    /// The plain Cortex-M attach routine (`cortexm_attach()`): halt the core, set up
    /// DVCR/breakpoints/watchpoints. Drivers wrap this with their own peripheral setup.
    pub fn cortexm_attach(&self) -> bool {
        if self.target.is_null() {
            return false;
        }
        crate::arm::cortexm::attach(self.target)
    }

    /// The plain Cortex-M detach routine (`cortexm_detach()`).
    pub fn cortexm_detach(&self) {
        if !self.target.is_null() {
            crate::arm::cortexm::detach(self.target);
        }
    }

    /// Read a 32-bit word, reporting a bus fault as [`TargetError::AccessFault`].
    pub fn try_read32(&self, addr: u32) -> Result<u32, TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        let value = self.read32(addr);
        if self.check_error() {
            Err(TargetError::AccessFault)
        } else {
            Ok(value)
        }
    }
}

/// The probe's shared scratch buffer (C `bmp_get_temporary_buffer()`), for stashing target
/// RAM around a stub run without spending driver storage or stack on it. Returns `None`
/// when the request does not fit the buffer.
pub fn temporary_buffer(len: usize) -> Option<&'static mut [u8]> {
    if len == 0 || len > u32::MAX as usize {
        return None;
    }
    let ptr = unsafe { bmp_get_temporary_buffer(len as u32) } as *mut u8;
    if ptr.is_null() {
        return None;
    }
    // SAFETY: the C buffer lives for the whole program and the target subsystem is
    // single-threaded, so only one stub run borrows it at a time (the C drivers use it
    // with the same contract).
    Some(unsafe { core::slice::from_raw_parts_mut(ptr, len) })
}
