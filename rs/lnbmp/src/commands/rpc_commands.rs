/*
    Converted from BMP remote.h
 */
pub const RPC_JTAG_PACKET  : u8 = b'J';
pub const RPC_SWDP_PACKET  : u8 = b'S';
pub const RPC_GEN_PACKET   : u8 = b'G';
pub const RPC_HL_PACKET    : u8 = b'H';
pub const RPC_ADIV5_PACKET : u8 = b'A';

/* Generic protocol elements */
pub const RPC_START        : u8 = b'A';
pub const RPC_TDITDO_TMS   : u8 = b'D';
pub const RPC_TDITDO_NOTMS : u8 = b'd';
pub const RPC_CYCLE        : u8 = b'c';
pub const RPC_IN_PAR       : u8 = b'I';
pub const RPC_TARGET_CLK_OE: u8 = b'E';
pub const RPC_FREQ_SET     : u8 = b'F';
pub const RPC_FREQ_GET     : u8 = b'f';
pub const RPC_IN           : u8 = b'i';
pub const RPC_NEXT         : u8 = b'N';
pub const RPC_OUT_PAR      : u8 = b'O';
pub const RPC_OUT          : u8 = b'o';
pub const RPC_PWR_SET      : u8 = b'P';
pub const RPC_PWR_GET      : u8 = b'p';
pub const RPC_RESET        : u8 = b'R';
pub const RPC_INIT         : u8 = b'S';
pub const RPC_TMS          : u8 = b'T';
pub const RPC_VOLTAGE      : u8 = b'V';
pub const RPC_NRST_SET     : u8 = b'Z';
pub const RPC_NRST_GET     : u8 = b'z';
pub const RPC_ADD_JTAG_DEV : u8 = b'J';

/* Protocol response options */
pub const RPC_RESP_OK       : u8 = b'K';
pub const RPC_RESP_PARERR   : u8 = b'P';
pub const RPC_RESP_ERR      : u8 = b'E';
pub const RPC_RESP_NOTSUP   : u8 = b'N';
/**
 * Error code
 */
pub const RPC_ERROR_UNRECOGNISED : u8 = 1;
pub const RPC_ERROR_WRONGLEN     : u8 = 2;
pub const RPC_ERROR_FAULT        : u8 = 3;
pub const RPC_ERROR_EXCEPTION    : u8 = 4;


/* High level protocol elements */
pub const RPC_HL_VERSION        : u8 = 3;
pub const RPC_HL_CHECK          : u8 = b'C';
pub const RPC_DP_READ           : u8 = b'd';
pub const RPC_LOW_ACCESS        : u8 = b'L';
pub const RPC_AP_READ           : u8 = b'a';
pub const RPC_AP_WRITE          : u8 = b'A';
pub const RPC_AP_MEM_READ       : u8 = b'M';
pub const RPC_MEM_READ          : u8 = b'h';
pub const RPC_MEM_WRITE_SIZED   : u8 = b'H';
pub const RPC_AP_MEM_WRITE_SIZED: u8 = b'm';

pub const RPC_REMOTE_ERROR_UNRECOGNISED   : u8 = 1;
pub const RPC_REMOTE_RESP         : u8 = b'&';
pub const RPC_REMOTE_RESP_OK      : u8 = b'K';
pub const RPC_REMOTE_RESP_PARERR  : u8 = b'P';
pub const RPC_REMOTE_RESP_ERR     : u8 = b'E';
pub const RPC_REMOTE_RESP_NOTSUP  : u8 = b'N';

/* Protocol v2 ADIV5 */
pub const RPC_REMOTE_DP_READ           : u8 = b'd';
pub const RPC_REMOTE_AP_READ           : u8 = b'a';
pub const RPC_REMOTE_AP_WRITE          : u8 = b'A';
pub const RPC_REMOTE_ADIV5_RAW_ACCESS  : u8 = b'R';
pub const RPC_REMOTE_MEM_READ          : u8 = b'm';
pub const RPC_REMOTE_MEM_WRITE         : u8 = b'M';

