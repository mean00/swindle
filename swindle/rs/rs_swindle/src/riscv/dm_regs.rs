//! RISC-V Debug Module register map: addresses, bit fields and abstract
//! command encodings (Debug Spec 0.13 / 1.0).
//!
//! These are bare spec constants, shared by every layer that talks to a DM:
//! `dm.rs` (activation, abstract commands), `hart.rs` (run control),
//! `mem.rs` (abstract/sysbus access), `c_adapter.rs` (C shims) and the
//! Tap-level transports. Nothing here depends on the DM controller, so the
//! values can be used by any DTM.

// --- DM Register Addresses ---
pub const RV_DM_DATA0: u8 = 0x04;
pub const RV_DM_DATA1: u8 = 0x05;
pub const RV_DM_CONTROL: u8 = 0x10;
pub const RV_DM_STATUS: u8 = 0x11;
pub const RV_DM_HART_INFO: u8 = 0x12;
pub const RV_DM_ABSTRACTCS: u8 = 0x16;
pub const RV_DM_COMMAND: u8 = 0x17;
pub const RV_DM_ABSTRACTAUTO: u8 = 0x18;
pub const RV_DM_PROGBUF0: u8 = 0x20;
pub const RV_DM_SBCS: u8 = 0x38;
pub const RV_DM_SBADDRESS0: u8 = 0x39;
pub const RV_DM_SBDATA0: u8 = 0x3c;

// --- DM Control Bits ---
pub const RV_DM_CTRL_ACTIVE: u32 = 1 << 0;
pub const RV_DM_CTRL_NDM_RESET: u32 = 1 << 1;
pub const RV_DM_CTRL_CLR_RESET_HALT_REQ: u32 = 1 << 2;
pub const RV_DM_CTRL_SET_RESET_HALT_REQ: u32 = 1 << 3;
pub const RV_DM_CTRL_HART_SEL_LO_SHIFT: u32 = 16;
pub const RV_DM_CTRL_HART_SEL_LO_MASK: u32 = 0x3ff << 16;
pub const RV_DM_CTRL_HART_SEL_HI_SHIFT: u32 = 6;
pub const RV_DM_CTRL_HART_SEL_HI_MASK: u32 = 0x3ff << 6;
pub const RV_DM_CTRL_HART_SEL_MASK: u32 = RV_DM_CTRL_HART_SEL_LO_MASK | RV_DM_CTRL_HART_SEL_HI_MASK;
pub const RV_DM_CTRL_HART_ACK_RESET: u32 = 1 << 28;
pub const RV_DM_CTRL_HART_RESUME_REQ: u32 = 1 << 30;
pub const RV_DM_CTRL_HALT_REQ: u32 = 1 << 31;

// --- DM Status Bits ---
pub const RV_DM_STAT_VERSION_MASK: u32 = 0x0f;
pub const RV_DM_STAT_VERSION_0_13: u32 = 2;
pub const RV_DM_STAT_VERSION_1_0: u32 = 3;
pub const RV_DM_STAT_AUTHENTICATED: u32 = 1 << 7;
pub const RV_DM_STAT_ANY_HALTED: u32 = 1 << 8;
pub const RV_DM_STAT_ALL_HALTED: u32 = 1 << 9;
pub const RV_DM_STAT_ANY_RUNNING: u32 = 1 << 10;
pub const RV_DM_STAT_ALL_RUNNING: u32 = 1 << 11;
pub const RV_DM_STAT_ANY_UNAVAIL: u32 = 1 << 12;
pub const RV_DM_STAT_ALL_UNAVAIL: u32 = 1 << 13;
pub const RV_DM_STAT_ANY_NONEXIST: u32 = 1 << 14;
pub const RV_DM_STAT_ALL_NONEXIST: u32 = 1 << 15;
pub const RV_DM_STAT_ANY_RESUMEACK: u32 = 1 << 16;
pub const RV_DM_STAT_ALL_RESUMEACK: u32 = 1 << 17;
pub const RV_DM_STAT_ANY_HAVERESET: u32 = 1 << 18;
pub const RV_DM_STAT_ALL_HAVERESET: u32 = 1 << 19;

// --- Abstract CS Bits ---
pub const RV_DM_ABSTCS_DATACOUNT_MASK: u32 = 0x0f;
pub const RV_DM_ABSTCS_CMDERR_SHIFT: u32 = 8;
pub const RV_DM_ABSTCS_CMDERR_MASK: u32 = 0x07 << 8;
pub const RV_DM_ABSTCS_BUSY: u32 = 1 << 12;
pub const RV_DM_ABSTCS_PROGBUF_SIZE_SHIFT: u32 = 24;
pub const RV_DM_ABSTCS_PROGBUF_SIZE_MASK: u32 = 0x1f << 24;

// --- Abstract Command Error Codes (`cmderr`) ---
pub const RV_ABST_CMDERR_NONE: u32 = 0;
pub const RV_ABST_CMDERR_BUSY: u32 = 1;
pub const RV_ABST_CMDERR_NOT_SUPP: u32 = 2;
pub const RV_ABST_CMDERR_EXCEPTION: u32 = 3;
pub const RV_ABST_CMDERR_WRONG_STATE: u32 = 4;
pub const RV_ABST_CMDERR_BUS_ERROR: u32 = 5;
pub const RV_ABST_CMDERR_OTHER: u32 = 7;

// --- Abstract Command Construction ---
pub const RV_CMD_TYPE_ACCESS_REG: u32 = 0 << 24;
pub const RV_CMD_AARSIZE_32: u32 = 2 << 20;
pub const RV_CMD_POSTEXEC: u32 = 1 << 18;
pub const RV_CMD_TRANSFER: u32 = 1 << 17;
pub const RV_CMD_WRITE: u32 = 1 << 16;
