//! Client/host RPC stubs for hosted (bmpa) mode.
//!
//! Exclusively used by the host runner when forwarding debug requests
//! across the RPC link to a physical probe.

use crate::rpc_host_generated::*;

#[unsafe(no_mangle)]
pub fn remote_bmp_set_frequency_rs(fq: u32) -> bool {
    remote_swindle_set_fq(fq)
}

#[unsafe(no_mangle)]
pub fn remote_bmp_get_frequency_rs(fq: &mut u32) -> bool {
    let (ok, val) = remote_swindle_get_fq();
    if ok {
        *fq = val;
    }
    ok
}

#[unsafe(no_mangle)]
pub fn bmp_set_frequency_c(fq: u32) {
    remote_bmp_set_frequency_rs(fq);
}

#[unsafe(no_mangle)]
pub fn bmp_get_frequency_c() -> u32 {
    let mut fq: u32 = 0;
    remote_bmp_get_frequency_rs(&mut fq);
    fq
}

#[unsafe(no_mangle)]
pub fn remote_ch32_riscv_dmi_reset_rs() -> bool {
    let (ok, _val) = remote_rv_reset();
    ok
}

#[unsafe(no_mangle)]
pub fn remote_ch32_riscv_dmi_read_rs(address: u32, value: &mut u32) -> bool {
    unsafe {
        crate::hosted::c_stubs::ln_rv_tx_count = crate::hosted::c_stubs::ln_rv_tx_count.wrapping_add(1);
    }
    let (ok, val) = remote_rv_dm_read(address);
    if ok {
        *value = val;
    }
    ok
}

#[unsafe(no_mangle)]
pub fn remote_ch32_riscv_dmi_write_rs(address: u32, value: u32) -> bool {
    unsafe {
        crate::hosted::c_stubs::ln_rv_tx_count = crate::hosted::c_stubs::ln_rv_tx_count.wrapping_add(1);
    }
    let (ok, _val) = remote_rv_dm_write(address, value);
    ok
}

#[unsafe(no_mangle)]
pub fn remote_sdi_reset_rs() -> bool {
    let (ok, _val) = remote_sdi_reset();
    ok
}

#[unsafe(no_mangle)]
pub fn remote_sdi_dm_read_rs(address: u32, value: &mut u32) -> bool {
    unsafe {
        crate::hosted::c_stubs::ln_rv_tx_count = crate::hosted::c_stubs::ln_rv_tx_count.wrapping_add(1);
    }
    let (ok, v) = remote_sdi_dm_read(address);
    if ok {
        *value = v;
    }
    ok
}

#[unsafe(no_mangle)]
pub fn remote_sdi_dm_write_rs(address: u32, value: u32) -> bool {
    unsafe {
        crate::hosted::c_stubs::ln_rv_tx_count = crate::hosted::c_stubs::ln_rv_tx_count.wrapping_add(1);
    }
    let (ok, _val) = remote_sdi_dm_write(address, value);
    ok
}

#[unsafe(no_mangle)]
pub fn remote_sdi_set_wire_rs(tbit_ns: u32, low1_ns: u32, low0_ns: u32, sample_ns: u32, no_trim: bool) -> bool {
    remote_sdi_set_wire(tbit_ns, low1_ns, low0_ns, sample_ns, no_trim)
}

#[unsafe(no_mangle)]
pub fn remote_crc32(address: u32, length: u32, out_crc: &mut u32) -> bool {
    let (ok, val) = remote_swindle_crc32(address, length);
    if ok {
        *out_crc = val;
    }
    ok
}

#[unsafe(no_mangle)]
pub fn remote_adiv5_swd_write_no_check_rs(addr: u16, data: u32) -> bool {
    let (ok, _) = remote_lnadiv_write(addr as u32, data);
    ok
}

#[unsafe(no_mangle)]
pub fn remote_raw_swd_write_rs(tick: u32, value: u32) {
    let _ = remote_lnadiv_raw_write(tick, value);
}

#[unsafe(no_mangle)]
pub fn remote_adiv5_swd_read_no_check_rs(addr: u16) -> u32 {
    let (_ok, val) = remote_lnadiv_read(addr as u32);
    val
}

#[unsafe(no_mangle)]
pub fn remote_adiv5_swd_raw_access_rs(rnw: u8, addr: u16, value: u32, fault: &mut u32) -> u32 {
    let (resp, f) = remote_lnadiv_lowlevel(rnw as u32, addr as u32, value);
    *fault = f;
    resp
}

#[unsafe(no_mangle)]
pub fn bmp_gpio_reset_c() {
    let _ = remote_gen_gpio_reset();
}
