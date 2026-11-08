// rpc_adiv5_impl.rs — ADIV5 class command implementations
// Called by rpc_target_generated.rs

use crate::arm::adi::adiv5::{AccessPort, DebugPort};
use crate::arm::adi::swd::{HardwareSwd, SwdTransport};

/// Raw DP access (low-level)
pub fn raw_access_v3(device_index: u32, ap_selection: u32, address: u32, value: u32) -> (u32, u32) {
    let mut dp = DebugPort::new(HardwareSwd);
    dp.dev_index = device_index as u8;
    let out = dp.transport.raw_access(ap_selection as u8, address as u16, value);
    (out, dp.wire_fault())
}

/// DP read
pub fn dp_read(device_index: u32, _ap_selection: u32, address: u32) -> (i32, u32) {
    let mut dp = DebugPort::new(HardwareSwd);
    dp.dev_index = device_index as u8;
    let val = dp.transport.dp_read(address as u16);
    (dp.wire_fault() as i32, val)
}

/// AP read
pub fn ap_read(device_index: u32, ap_selection: u32, address: u32) -> (i32, u32) {
    let mut dp = DebugPort::new(HardwareSwd);
    dp.dev_index = device_index as u8;
    let val = dp.ap_read(ap_selection as u8, address as u16).unwrap_or(0);
    (dp.wire_fault() as i32, val)
}

/// AP write
pub fn ap_write(device_index: u32, ap_selection: u32, address: u32, value: u32) -> (i32, u32) {
    let mut dp = DebugPort::new(HardwareSwd);
    dp.dev_index = device_index as u8;
    let _ = dp.ap_write(ap_selection as u8, address as u16, value);
    (dp.wire_fault() as i32, 0)
}

/// Memory read — writes into `buffer` and returns (fault, bytes_written)
pub fn mem_read(device_index: u32, ap_selection: u32, csw: u32, address: u32, length: u32, buffer: &mut [u8]) -> (i32, usize) {
    if length > 1024 {
        return (-1, 0);
    }
    let mut dp = DebugPort::new(HardwareSwd);
    dp.dev_index = device_index as u8;
    let mut ap = AccessPort::new(&mut dp, ap_selection as u8);
    ap.csw = csw;
    let l = length as usize;
    let res = ap.mem_read_bytes(&mut buffer[0..l], address);
    let fault = if res.is_err() && dp.fault == 0 { -1 } else { dp.fault as i32 };
    (fault, l)
}

/// Memory write — `data` arrives as a CBOR byte string, i.e. already decoded
pub fn mem_write(device_index: u32, ap_selection: u32, csw: u32, align: u32, address: u32, length: u32, data: &[u8]) -> (i32, u32) {
    if length > 1024 || data.len() > 1024 {
        return (-1, 0);
    }
    let mut dp = DebugPort::new(HardwareSwd);
    dp.dev_index = device_index as u8;
    let mut ap = AccessPort::new(&mut dp, ap_selection as u8);
    ap.csw = csw;
    let res = ap.mem_write_aligned(address, data, align);
    let fault = if res.is_err() && dp.fault == 0 { -1 } else { dp.fault as i32 };
    (fault, 0)
}
