//! `goDfu` - the probe's "reboot into the bootloader" action (`usb.md`, P5/P7).
//!
//! This was the C++ `goDfu()` in `swindle/src/usb/bmp_usb.cpp`. The DFU-runtime request
//! from the host (`tud_dfu_runtime_reboot_to_dfu_cb`, in `rs_usb`) calls it through a
//! registered hook, so the probe never needs the C++ object graph for it.
//!
//! Two recipes: on the bluepill family the bootloader is entered through its RAM marker
//! (pull DP low so the host sees a disconnect, hand PA1 to its alternate function, write
//! the marker where the bootloader looks for it, reset); on RP2040/RP2350 the ROM does
//! the work and `Rp2040ResetToFwUpload()` is the whole action.

#![allow(dead_code)]

/// Install `go_dfu` as the DFU-runtime hook. Called once at start-up.
pub(crate) fn init() {
    rust_esprit::rs_usb::rn_usb_set_dfu_hook(Some(go_dfu));
}

/// `rn_usb_dfu_init()` - called by `bmp_usb.cpp` where the C++ build called
/// `lnUsbDFURT::addDFURTCb(goDfu)`.
#[unsafe(no_mangle)]
pub extern "C" fn rn_usb_dfu_init() {
    init();
}

#[cfg(feature = "bluepill")]
mod bootloader {
    use rust_esprit::rs_gpio_bluepill::{Mode, Pin, set_mode, write};

    #[cfg(target_arch = "arm")]
    mod registers {
        /// System Control Block base address (ARMv7-M Architecture Reference Manual §B3.2.2).
        pub const SCB_BASE: u32 = 0xE000_ED00;
        /// `AIRCR.VECTKEY` write key (`0x05FA` in bits [31:16]).
        pub const AIRCR_VECTKEY: u32 = 0x05FA_0000;
        /// `AIRCR.SYSRESETREQ` (bit 2): request a system reset.
        pub const AIRCR_SYSRESETREQ: u32 = 1 << 2;

        #[repr(C)]
        pub struct ScbRegisters {
            pub cpuid: u32,
            pub icsr: u32,
            pub vtor: u32,
            pub aircr: u32,
        }

        const _: () = assert!(core::mem::offset_of!(ScbRegisters, vtor) == 0x08);
        const _: () = assert!(core::mem::offset_of!(ScbRegisters, aircr) == 0x0C);

        macro_rules! scb_reg {
            ($member:ident) => {
                (SCB_BASE + core::mem::offset_of!(ScbRegisters, $member) as u32) as *mut u32
            };
        }
        pub(super) use scb_reg;
    }

    /// Where the probe's DFU bootloader looks for its "enter DFU" marker
    /// (`bootloader/gd32_swindle_bootloader2.elf`: the word at 0x20000020 points here).
    const DFU_MARKER_ADDRESS: usize = 0x2000_0000;
    /// The two words the bootloader compares (`rebooted_into_dfu()`): 0xCC00FFEE then
    /// 0xDEADBEEF, i.e. the u64 0xDEADBEEF_CC00_FFEE little-endian.
    const DFU_MARKER: u64 = 0xDEAD_BEEF_CC00_FFEE;
    /// USB D+ (PA12) - pulled low to detach from the host.
    const DFU_DETACH_PIN: Pin = Pin::PA12;
    /// The bootloader's "stay in DFU" pin (PA1), driven as an alternate function.
    const DFU_BOOT_PIN: Pin = Pin::PA1;
    /// Time the host needs to notice the disconnect.
    const DFU_DETACH_DELAY_MS: u32 = 50;

    unsafe extern "C" {
        /// `void lnSoftSystemReset()` - esprit's soft reset. Real on CH32; on the ARM
        /// parts it writes `AIRCR` with VECTRESET, which they do not implement, and then
        /// spins - hence [`reset_to_bootloader`] below.
        #[link_name = "_Z17lnSoftSystemResetv"]
        fn ln_soft_system_reset();
    }

    /// Request the reset that lands in the bootloader.
    ///
    /// ARM: `AIRCR` is written with `SYSRESETREQ` (bit 2) after clearing `VTOR`, so the
    /// core comes back through whatever is aliased at 0 - the DFU bootloader.
    /// VECTRESET (`lnSoftSystemReset`) is not implemented on STM32F1/GD32F30x: it leaves
    /// the core spinning, which is what a `dfu-util -e` used to do to the probe.
    ///
    /// CH32: the RISC-V side of `lnSoftSystemReset` does perform a reset.
    #[cfg(target_arch = "arm")]
    fn reset_to_bootloader() -> ! {
        use registers::{AIRCR_SYSRESETREQ, AIRCR_VECTKEY, SCB_BASE, ScbRegisters, scb_reg};
        // SAFETY: `scb_reg!` points to the architectural SCB VTOR/AIRCR registers;
        // DSB ensures the RAM marker and VTOR write complete before SYSRESETREQ and
        // ensures the reset request is observed before spinning.
        unsafe {
            scb_reg!(vtor).write_volatile(0);
            core::arch::asm!("dsb sy", options(nostack, preserves_flags));
            scb_reg!(aircr).write_volatile(AIRCR_VECTKEY | AIRCR_SYSRESETREQ);
            core::arch::asm!("dsb sy", options(nostack, preserves_flags));
        }
        loop {
            core::hint::spin_loop();
        }
    }

    #[cfg(not(target_arch = "arm"))]
    fn reset_to_bootloader() -> ! {
        unsafe { ln_soft_system_reset() };
        loop {
            core::hint::spin_loop();
        }
    }

    pub(super) fn go() -> ! {
        set_mode(DFU_DETACH_PIN, Mode::Output, 0);
        write(DFU_DETACH_PIN, false);
        rust_esprit::delay_ms(DFU_DETACH_DELAY_MS);
        set_mode(DFU_BOOT_PIN, Mode::AlternatePushPull, 0);
        // SAFETY: the marker sits at the start of RAM, which the bootloader documents;
        // the core resets right after.
        unsafe { (DFU_MARKER_ADDRESS as *mut u64).write_volatile(DFU_MARKER) };
        reset_to_bootloader();
    }
}

/// Reboot the probe into its DFU bootloader (bluepill family).
#[cfg(feature = "bluepill")]
pub fn go_dfu() {
    bootloader::go();
}

/// Reboot the probe into its DFU bootloader (RP2040/RP2350): the ROM's own
/// `Rp2040ResetToFwUpload()`, a C++ symbol hence the mangled name.
#[cfg(any(feature = "rp2040", feature = "rp2350"))]
pub fn go_dfu() {
    unsafe extern "C" {
        #[link_name = "_Z21Rp2040ResetToFwUploadv"]
        fn rp2040_reset_to_fw_upload();
    }
    unsafe { rp2040_reset_to_fw_upload() };
    loop {
        core::hint::spin_loop();
    }
}
