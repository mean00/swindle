//! The probe's USB device (`usb.md` P4).
//!
//! Only the device itself: how many CDC-ACM interfaces this build exposes and which
//! MCU endpoint budget they have to fit in. The interface *roles* (GDB, UART bridge,
//! logger) stay in `usb_gdb.rs` / `usb_serial_bridge.rs` / `cdc_logger.rs`, and the
//! descriptors stay in `swindle/private_include/lnBMP_usb_descriptor*.h`.
//!
//! The instantiation below is the compile-time check: `rs_usb::budget` compares
//! `1 + 2 * CDC_COUNT` with the platform's endpoint count, so a build that asks for
//! more CDC interfaces than the MCU can host fails here instead of enumerating badly
//! on a probe.

use rust_esprit::rs_usb::CdcAcmDevice;

/// Endpoint numbers of this MCU's USB controller, EP0 included - from the platform
/// definition (`rs_bluepill::USB_MAX_ENDPOINT`, 16 on RP2xxx).
const MAX_ENDPOINT: usize = rust_esprit::USB_MAX_ENDPOINT;

/// The probe's device. Never dereferenced at run time: the interfaces are claimed by
/// the modules next door, which index them the same way the descriptors do.
#[allow(dead_code)]
pub(crate) static DEVICE: CdcAcmDevice<{ rust_esprit::rs_usb::CDC_COUNT }, MAX_ENDPOINT> = CdcAcmDevice::new();
