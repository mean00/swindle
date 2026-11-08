//! Wire-level error codes shared across the C/Rust FFI boundary for SWD, RVSWD, and SDI.
//!
//! Matches `bmp_wire_status_t` and `BMP_WIRE_*` in `swindle/private_include/platform_support.h`.
//! `0` (`BMP_WIRE_OK`) is always success; `1` (`SWD_ACK_OK`) is never used as an error value so
//! sticky fault latches never collide with a valid SWD ACK.

use crate::arm::adi::adiv5::AdiError;
use crate::riscv::dmi::DmiError;

/// Raw C `bmp_wire_status_t` constants.
pub const BMP_WIRE_OK: u8 = 0x00;
pub const BMP_WIRE_ACK_WAIT: u8 = 0x02;
pub const BMP_WIRE_ACK_FAULT: u8 = 0x04;
pub const BMP_WIRE_NO_RESPONSE: u8 = 0x07;
pub const BMP_WIRE_PARITY_ERR: u8 = 0x08;
pub const BMP_WIRE_PROTOCOL_ERR: u8 = 0x09;
pub const BMP_WIRE_HW_TIMEOUT: u8 = 0x0A;
pub const BMP_WIRE_DISABLED: u8 = 0x0B;

/// Wire-level error returned by the C++ frame drivers (SWD, RVSWD, SDI).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    /// Target answered WAIT (SWD `0x02`) or DMI busy.
    Wait = BMP_WIRE_ACK_WAIT,
    /// Target answered FAULT (SWD `0x04`) or DMI sticky fault.
    Fault = BMP_WIRE_ACK_FAULT,
    /// Target did not drive the wire (`0x07` / `0x0F` / `0xFFFFFFFF` with dead DMSTATUS).
    NoResponse = BMP_WIRE_NO_RESPONSE,
    /// Read frame parity check failed (SWD 1-bit or RVSWD 2-bit parity).
    Parity = BMP_WIRE_PARITY_ERR,
    /// Malformed ACK/status nibble or invalid register address (> 7 bits).
    Protocol = BMP_WIRE_PROTOCOL_ERR,
    /// Probe peripheral timeout (RP2040 PIO FIFO stall or shorted open-drain pad).
    HardwareTimeout = BMP_WIRE_HW_TIMEOUT,
    /// Transport disabled at build time (`SWINDLE_WITH_SDI=OFF` or unsupported stub).
    Disabled = BMP_WIRE_DISABLED,
}

// Compile-time ABI guard: `WireError` must be a single byte and never collide with 0 or 1.
const _: () = assert!(core::mem::size_of::<WireError>() == 1);
const _: () = assert!(BMP_WIRE_OK == 0);
const _: () = assert!(WireError::Wait as u8 != 1);
const _: () = assert!(WireError::Parity as u8 != 1);

/// Idiomatic Rust result for a low-level wire frame exchange.
pub type WireResult<T> = Result<T, WireError>;

impl WireError {
    /// Convert a raw `bmp_wire_status_t` (`u8`) from C FFI into `WireResult<()>`.
    #[inline]
    pub const fn check(status: u8) -> WireResult<()> {
        match status {
            BMP_WIRE_OK => Ok(()),
            BMP_WIRE_ACK_WAIT => Err(Self::Wait),
            BMP_WIRE_ACK_FAULT => Err(Self::Fault),
            BMP_WIRE_NO_RESPONSE => Err(Self::NoResponse),
            BMP_WIRE_PARITY_ERR => Err(Self::Parity),
            BMP_WIRE_PROTOCOL_ERR => Err(Self::Protocol),
            BMP_WIRE_HW_TIMEOUT => Err(Self::HardwareTimeout),
            BMP_WIRE_DISABLED => Err(Self::Disabled),
            _ => Err(Self::Protocol),
        }
    }

    /// Convert a raw `bmp_wire_status_t` and read value into `WireResult<u32>`.
    #[inline]
    pub const fn into_read_result(status: u8, value: u32) -> WireResult<u32> {
        match Self::check(status) {
            Ok(()) => Ok(value),
            Err(e) => Err(e),
        }
    }

    /// Raw `bmp_wire_status_t` byte representation.
    #[inline]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }
}

impl From<WireError> for DmiError {
    #[inline]
    fn from(err: WireError) -> Self {
        match err {
            WireError::Wait => DmiError::Busy,
            WireError::Fault => DmiError::Fault,
            WireError::HardwareTimeout => DmiError::Timeout,
            WireError::NoResponse | WireError::Parity | WireError::Protocol => DmiError::BusError,
            WireError::Disabled => DmiError::NotSupported,
        }
    }
}

impl From<WireError> for AdiError {
    #[inline]
    fn from(err: WireError) -> Self {
        match err {
            WireError::Wait | WireError::HardwareTimeout => AdiError::Timeout,
            WireError::Fault | WireError::NoResponse => AdiError::Fault,
            WireError::Parity | WireError::Protocol => AdiError::ProtocolError,
            WireError::Disabled => AdiError::Unsupported,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_error_roundtrip_and_mapping() {
        assert_eq!(WireError::check(BMP_WIRE_OK), Ok(()));
        let cases = [
            (BMP_WIRE_ACK_WAIT, WireError::Wait, DmiError::Busy, AdiError::Timeout),
            (BMP_WIRE_ACK_FAULT, WireError::Fault, DmiError::Fault, AdiError::Fault),
            (BMP_WIRE_NO_RESPONSE, WireError::NoResponse, DmiError::BusError, AdiError::Fault),
            (BMP_WIRE_PARITY_ERR, WireError::Parity, DmiError::BusError, AdiError::ProtocolError),
            (BMP_WIRE_PROTOCOL_ERR, WireError::Protocol, DmiError::BusError, AdiError::ProtocolError),
            (BMP_WIRE_HW_TIMEOUT, WireError::HardwareTimeout, DmiError::Timeout, AdiError::Timeout),
            (BMP_WIRE_DISABLED, WireError::Disabled, DmiError::NotSupported, AdiError::Unsupported),
        ];
        for (raw, expected_wire, expected_dmi, expected_adi) in cases {
            let err = WireError::check(raw).unwrap_err();
            assert_eq!(err, expected_wire);
            assert_eq!(err.as_raw(), raw);
            assert_eq!(DmiError::from(err), expected_dmi);
            assert_eq!(AdiError::from(err), expected_adi);
        }
        // Any unknown non-zero byte (including 0x01) maps safely to Protocol
        assert_eq!(WireError::check(0x01), Err(WireError::Protocol));
        assert_eq!(WireError::check(0xFF), Err(WireError::Protocol));
    }
}
