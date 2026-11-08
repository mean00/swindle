//! JEP-106 manufacturer codes, encoded the way the C target layer does it
//! (`blackmagic/src/target/jep106.h`) so they can be compared with
//! `target_s::designer_code`.
//!
//! A code is 16 bits: bit 15 flags a legacy (ASCII, not JEP-106) code, bits 11-14 hold the
//! number of continuation codes and the low 8 bits the code itself, without its parity bit.

/// Flags a designer code as a legacy ASCII code instead of a JEP-106 one.
///
/// Devices that enumerate an access port with an empty ASCII code and report no designer
/// code anywhere else are matched on this value.
pub const ASCII_CODE_FLAG: u32 = 1 << 15;

pub const FREESCALE: u32 = 0x00e;
pub const NXP: u32 = 0x015;
pub const TEXAS: u32 = 0x017;
pub const ATMEL: u32 = 0x01f;
pub const STM: u32 = 0x020;
pub const CYPRESS: u32 = 0x034;
pub const INFINEON: u32 = 0x041;
pub const NORDIC: u32 = 0x244;
pub const RENESAS: u32 = 0x423;
pub const ARM: u32 = 0x43b;
pub const SPECULAR: u32 = 0x501;
pub const ENERGY_MICRO: u32 = 0x673;
pub const WCH: u32 = 0x72a;
pub const GIGADEVICE: u32 = 0x751;
pub const RASPBERRY: u32 = 0x913;
pub const ARM_CHINA: u32 = 0xa75;
