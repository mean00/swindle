//! The RP2040 rescue debug port: the DP of an RP2040 whose cores are locked up by a bad
//! flash image (C `rp2040_rescue_probe()` / `rp2040_rescue_do_reset()`).
//!
//! It is not a normal Cortex-M target probe. The chip's rescue DP has no AP at all, so
//! `adiv5.c` fabricates one for the DP and hands it over, and what comes back is a `target_s`
//! that carries nothing but an attach hook. Attaching to it requests a debug reset from the
//! DP and then reports failure: the reset *is* the operation, and starting a session from it
//! makes the probe rescan the bus and find the chip alive again.

use core::ffi::c_void;

use rsbmp::arm::c_seam;
use rsbmp::arm::cortex::AccessPort;
use rsbmp::target::{Target, TargetView, delay_ms, target_new};

/// `ADIV5_DP_CTRLSTAT`: the DP control/status register.
const ADIV5_DP_CTRLSTAT: u32 = 0x04;
/// `ADIV5_DP_CTRLSTAT_CDBGPWRUPREQ`: ask the DP for debug power.
const ADIV5_DP_CTRLSTAT_CDBGPWRUPREQ: u32 = 1 << 28;
/// `ADIV5_DP_CTRLSTAT_CDBGRSTACK`: stays set while a debug reset is acknowledged.
const ADIV5_DP_CTRLSTAT_CDBGRSTACK: u32 = 1 << 29;

/// How long the DP gets to acknowledge the debug reset (C `platform_timeout_set(…, 100)`).
const RESET_TIMEOUT_MS: u32 = 100;
/// Poll interval of the acknowledgement loop: where C spins on the DP for 100 ms, this reads it
/// once a millisecond for the same 100 ms.
const RESET_POLL_MS: u32 = 1;

/// Name this pseudo-target reports (C `target->driver`).
const DRIVER_NAME: &'static str = "RP2040 Rescue (Attach to reset)";

/// Ask the DP for debug power and a debug reset, and wait for it to be acknowledged.
///
/// Returns `false` on purpose: C `rp2040_rescue_do_reset()` fails the attach so that no session
/// is left on a target that has no memory and no core to talk to.
unsafe fn rescue_attach(target: *mut Target) -> bool {
    let ap: *mut AccessPort = TargetView::new(target).priv_ptr().cast();
    if ap.is_null() {
        return false;
    }

    let ctrl = c_seam::dp_read(ap, ADIV5_DP_CTRLSTAT);
    c_seam::dp_write(ap, ADIV5_DP_CTRLSTAT, ctrl | ADIV5_DP_CTRLSTAT_CDBGPWRUPREQ);

    let mut waited = 0;
    loop {
        let status = c_seam::dp_read(ap, ADIV5_DP_CTRLSTAT);
        if status & ADIV5_DP_CTRLSTAT_CDBGRSTACK == 0 {
            break;
        }
        if waited >= RESET_TIMEOUT_MS {
            break;
        }
        delay_ms(RESET_POLL_MS);
        waited += RESET_POLL_MS;
    }

    false
}

/// Drop the AP reference the rescue target took (C `adiv5_ap_unref` as `priv_free`).
unsafe fn release_ap(ap: *mut c_void) {
    c_seam::ap_unref(ap.cast());
}

/// Give the rescue DP a target of its own (C `rp2040_rescue_probe()`).
///
/// `ap` is the fabricated access port `adiv5.c` made for a DP that answers but has no AP. It
/// arrives with no reference held: [`probe()`] takes one on it, and [`release_ap()`] drops it
/// when the target list is torn down.
///
/// # Safety
///
/// `ap` must be a live, heap-allocated `adiv5_access_port_s` that nothing else releases.
pub unsafe fn probe(ap: *mut AccessPort) -> bool {
    unsafe {
        let target = target_new();
        if target.is_null() {
            return false;
        }

        let view = TargetView::new(target);
        c_seam::ap_ref(ap);
        view.set_attach(rescue_attach);
        view.set_priv(ap);
        view.set_priv_free(release_ap);
        view.set_driver(DRIVER_NAME);

        true
    }
}
