//! Shared RPC protocol definitions, re-exporting generated protocol definitions.

pub use crate::rpc_common_generated::*;

// Backward-compatibility aliases for legacy code
pub const RPC_HL_VERSION: u8 = 3;
pub const RPC_DP_READ: u8 = RPC_ADIV5_DP_READ;
pub const RPC_AP_READ: u8 = RPC_ADIV5_AP_READ;
pub const RPC_AP_WRITE: u8 = RPC_ADIV5_AP_WRITE;
pub const RPC_LOW_ACCESS: u8 = RPC_LNADIV_LOWLEVEL;
pub const RPC_RAW_ACCESS_V3: u8 = RPC_ADIV5_RAW_ACCESS_V3;
pub const RPC_MEM_READ: u8 = RPC_ADIV5_MEM_READ;
pub const RPC_MEM_WRITE: u8 = RPC_ADIV5_MEM_WRITE;
pub const RPC_MEM_READ_V3: u8 = RPC_ADIV5_MEM_READ;
pub const RPC_MEM_WRITE_V3: u8 = RPC_ADIV5_MEM_WRITE;

pub const RPC_REMOTE_ERROR_UNRECOGNISED: u8 = RPC_ERROR_UNRECOGNISED;
pub const RPC_REMOTE_RESP_OK: u8 = RPC_RESP_OK;
pub const RPC_REMOTE_RESP_PARERR: u8 = RPC_RESP_PARERR;
pub const RPC_REMOTE_RESP_ERR: u8 = RPC_RESP_ERR;
pub const RPC_REMOTE_RESP_NOTSUP: u8 = RPC_RESP_NOTSUP;

pub const RPC_REMOTE_DP_READ: u8 = RPC_ADIV5_DP_READ;
pub const RPC_REMOTE_AP_READ: u8 = RPC_ADIV5_AP_READ;
pub const RPC_REMOTE_AP_WRITE: u8 = RPC_ADIV5_AP_WRITE;
pub const RPC_REMOTE_ADIV5_RAW_ACCESS: u8 = RPC_ADIV5_RAW_ACCESS_V3;
pub const RPC_REMOTE_MEM_READ: u8 = RPC_ADIV5_MEM_READ;
pub const RPC_REMOTE_MEM_WRITE: u8 = RPC_ADIV5_MEM_WRITE;
