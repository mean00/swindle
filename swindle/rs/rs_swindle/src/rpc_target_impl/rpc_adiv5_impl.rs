// rpc_adiv5_impl.rs — ADIV5 class command implementations
// Called by rpc_target_generated.rs

use crate::bmp;

/// Raw DP access (low-level)
pub fn raw_access_v3(device_index: u32, ap_selection: u32, address: u32, value: u32) -> (u32, u32) {
    let (fault, outvalue) = bmp::bmp_adiv5_full_dp_low_level(
        device_index,
        ap_selection,
        address as u16,
        value,
    );
    (outvalue, fault as u32)
}

/// DP read
pub fn dp_read(device_index: u32, ap_selection: u32, address: u32) -> (i32, u32) {
    bmp::bmp_adiv5_full_dp_read(device_index, ap_selection, address as u16)
}

/// AP read
pub fn ap_read(device_index: u32, ap_selection: u32, address: u32) -> (i32, u32) {
    let value = bmp::bmp_adiv5_ap_read(device_index, ap_selection, address);
    (0, value)
}

/// AP write
pub fn ap_write(device_index: u32, ap_selection: u32, address: u32, value: u32) -> (i32, u32) {
    bmp::bmp_adiv5_ap_write(device_index, ap_selection, address, value);
    (0, 0)
}

/// Memory read — writes into `buffer` and returns (fault, bytes_written)
pub fn mem_read(device_index: u32, ap_selection: u32, csw: u32, address: u32, length: u32, buffer: &mut [u8]) -> (i32, usize) {
    if length > 1024 {
        return (-1, 0);
    }
    let l = length as usize;
    let fault = bmp::bmp_adiv5_mem_read(
        device_index,
        ap_selection,
        csw,
        address,
        &mut buffer[0..l],
    );
    (fault, l)
}

/// Memory write
pub fn mem_write(device_index: u32, ap_selection: u32, csw: u32, align: u32, address: u32, length: u32, data: &[u8]) -> (i32, u32) {
    if length > 1024 {
        return (-1, 0);
    }
    let buffer = crate::rpc_target::rpc_reply::get_temp_buffer();
    let decoded = crate::parsing_util::u8_hex_string_to_u8s(data, buffer);
    let fault = bmp::bmp_adiv5_mem_write(
        device_index,
        ap_selection,
        csw,
        address,
        align,
        decoded,
    );
    (fault, 0)
}
