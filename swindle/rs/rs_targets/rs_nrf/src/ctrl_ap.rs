//! Nordic Semiconductor Access Port (CTRL-AP) recovery drivers.

use core::ffi::c_void;
use crate::registers::*;
use rsbmp::arm::c_seam;
use rsbmp::arm::cortex::AccessPort;
use rsbmp::target::{Target, TargetView, target_new};

unsafe fn release_ap(ap: *mut c_void) {
    c_seam::ap_unref(ap.cast());
}

/// Mass erase handler for nRF52 CTRL-AP.
unsafe fn nrf51_ctrl_ap_mass_erase(target: *mut Target, _timeout: *mut c_void) -> bool {
    let ap: *mut AccessPort = TargetView::new(target).priv_ptr().cast();
    if ap.is_null() {
        return false;
    }

    c_seam::dp_write(ap, CTRL_AP_POWER_EN as u32, 0x5000_0000);
    c_seam::dp_write(ap, CTRL_AP_SELECT_AP as u32, 0x0100_0000);
    c_seam::ap_write(ap, CTRL_AP_CONTROL, 0x0000_0001);

    let mut count = 0;
    while c_seam::ap_read(ap, CTRL_AP_STATUS) != 0 {
        count += 1;
        if count > 5_000_000 {
            return false;
        }
    }

    // Two reads to settle protection status
    let _ = c_seam::ap_read(ap, CTRL_AP_PROT_EN);
    let _ = c_seam::ap_read(ap, CTRL_AP_PROT_EN);

    true
}

/// Probe an nRF52 CTRL-AP access port (C `nrf51_ctrl_ap_probe`).
pub unsafe fn nrf51_ctrl_ap_probe(ap: *mut AccessPort) -> bool {
    if ap.is_null() {
        return false;
    }
    if unsafe { (*ap).idr } != NRF52_CTRL_AP_IDR {
        return false;
    }

    let target = unsafe { target_new() };
    if target.is_null() {
        return false;
    }

    let view = TargetView::new(target);
    view.set_enter_flash_mode_stub();
    view.set_mass_erase(nrf51_ctrl_ap_mass_erase);
    c_seam::ap_ref(ap);
    view.set_priv(ap);
    view.set_priv_free(release_ap);

    // Initial read to clear latch, second read gives status
    c_seam::ap_read(ap, CTRL_AP_PROT_EN);
    let status = c_seam::ap_read(ap, CTRL_AP_PROT_EN);
    if status != 0 {
        view.set_driver("nRF52 Access Port");
    } else {
        view.set_driver("nRF52 Access Port (protected)");
    }
    view.set_regs_size(0);

    true
}

/// Mass erase handler for nRF54L CTRL-AP.
unsafe fn nrf54l_ctrl_ap_mass_erase(target: *mut Target, _timeout: *mut c_void) -> bool {
    let ap: *mut AccessPort = TargetView::new(target).priv_ptr().cast();
    if ap.is_null() {
        return false;
    }

    let ctrl = c_seam::dp_read(ap, 0x04);
    c_seam::dp_write(ap, 0x04, ctrl | 0x1000_0000); // CDBGPWRUPREQ

    c_seam::ap_write(ap, NRF54L_CTRL_AP_ERASEALL, NRF54L_CTRL_AP_ERASEALL_ERASE);

    let mut count = 0;
    while c_seam::ap_read(ap, NRF54L_CTRL_AP_ERASEALLSTATUS) == NRF54L_CTRL_AP_ERASEALLSTATUS_BUSY {
        count += 1;
        if count > 5_000_000 {
            return false;
        }
    }

    // Hard reset cycle
    c_seam::ap_write(ap, NRF54L_CTRL_AP_RESET, NRF54L_CTRL_AP_RESET_HARDRESET);
    c_seam::ap_write(ap, NRF54L_CTRL_AP_RESET, NRF54L_CTRL_AP_RESET_NORESET);

    true
}

/// Probe an nRF54L CTRL-AP access port (C `nrf54l_ctrl_ap_probe`).
pub unsafe fn nrf54l_ctrl_ap_probe(ap: *mut AccessPort) -> bool {
    if ap.is_null() {
        return false;
    }
    if unsafe { (*ap).idr } != NRF54L_CTRL_AP_IDR_VALUE {
        return false;
    }

    let target = unsafe { target_new() };
    if target.is_null() {
        return false;
    }

    let view = TargetView::new(target);
    view.set_mass_erase(nrf54l_ctrl_ap_mass_erase);
    c_seam::ap_ref(ap);
    view.set_priv(ap);
    view.set_priv_free(release_ap);

    let status = c_seam::ap_read(ap, NRF54L_CTRL_AP_APPROTECT_STATUS);
    if (status
        & (NRF54L_CTRL_AP_APPROTECT_STATUS_APPROTECT_ENABLED
            | NRF54L_CTRL_AP_APPROTECT_STATUS_SECUREAPPROTECT_ENABLED))
        == 0
    {
        view.set_driver("nRF54L Access Port");
    } else {
        view.set_driver("nRF54L Access Port (protected)");
    }
    view.set_regs_size(0);

    true
}
