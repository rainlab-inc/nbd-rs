#![allow(dead_code)]
pub const NBD_OPT_EXPORT_NAME: u32 = 1;
pub const NBD_OPT_ABORT: u32 = 2;
pub const NBD_OPT_LIST: u32 = 3;
pub const NBD_OPT_PEEK_EXPORT: u32 = 4;
pub const NBD_OPT_STARTTLS: u32 = 5;
pub const NBD_OPT_INFO: u32 = 6;
pub const NBD_OPT_GO: u32 = 7;
pub const NBD_OPT_STRUCTURED_REPLY: u32 = 8;
pub const NBD_OPT_LIST_META_CONTEXT: u32 = 9;
pub const NBD_OPT_SET_META_CONTEXT: u32 = 10;

pub const NBD_INFO_EXPORT: u16 = 0;
pub const NBD_INFO_NAME: u16 = 1;
pub const NBD_INFO_DESCRIPTION: u16 = 2;
pub const NBD_INFO_BLOCK_SIZE: u16 = 3;

pub const NBD_CMD_READ: u16 = 0;
pub const NBD_CMD_WRITE: u16 = 1;
pub const NBD_CMD_DISC: u16 = 2;
pub const NBD_CMD_FLUSH: u16 = 3;
pub const NBD_CMD_TRIM: u16 = 4;
pub const NBD_CMD_CACHE: u16 = 5;
pub const NBD_CMD_WRITE_ZEROES: u16 = 6;
pub const NBD_CMD_BLOCK_STATUS: u16 = 7;
pub const NBD_CMD_RESIZE: u16 = 8;

// command flags
pub const NBD_CMD_FLAG_FUA: u8 = 1 << 0;
pub const NBD_CMD_FLAG_NO_HOLE: u8 = 1 << 1;
pub const NBD_CMD_FLAG_DF: u8 = 1 << 2;
pub const NBD_CMD_FLAG_REQ_ONE: u8 = 1 << 3;
// structured reply flags
pub const NBD_REPLY_FLAG_DONE: u8 = 1 << 0;

pub const NBD_EPERM: u8 = 1;
pub const NBD_EIO: u8 = 5;
pub const NBD_ENOMEM: u8 = 12;
pub const NBD_EINVAL: u8 = 22;
pub const NBD_ENOSPC: u8 = 28;
pub const NBD_EOVERFLOW: u8 = 75;
pub const NBD_ESHUTDOWN: u8 = 108;

pub const NBD_REPLY_TYPE_NONE: u8 = 0;
pub const NBD_REPLY_TYPE_OFFSET_DATA: u8 = 1;
pub const NBD_REPLY_TYPE_OFFSET_HOLE: u8 = 2;
pub const NBD_REPLY_TYPE_BLOCK_STATUS: u8 = 5;
pub const NBD_REPLY_TYPE_ERROR: u8 = 2 ^ 15 + 1;
pub const NBD_REPLY_TYPE_ERROR_OFFSET: u8 = 2 ^ 15 + 2;

pub const NBD_REP_ACK: u32 = 1;
pub const NBD_REP_SERVER: u32 = 2;
pub const NBD_REP_INFO: u32 = 3;
pub const NBD_REP_META_CONTEXT: u32 = 4;

pub const NBD_REP_ERR: u32 = 1 << 31;
pub const NBD_REP_ERR_UNSUP: u32 = NBD_REP_ERR + 1;
pub const NBD_REP_ERR_POLICY: u32 = NBD_REP_ERR + 2;
pub const NBD_REP_ERR_INVALID: u32 = NBD_REP_ERR + 3;
pub const NBD_REP_ERR_PLATFORM: u32 = NBD_REP_ERR + 4;
pub const NBD_REP_ERR_TLS_REQD: u32 = NBD_REP_ERR + 5;
pub const NBD_REP_ERR_UNKNOWN: u32 = NBD_REP_ERR + 6;
pub const NBD_REP_ERR_SHUTDOWN: u32 = NBD_REP_ERR + 7;
pub const NBD_REP_ERR_BLOCK_SIZE_REQD: u32 = NBD_REP_ERR + 8;
pub const NBD_REP_ERR_TOO_BIG: u32 = NBD_REP_ERR + 9;

pub const NBD_STATE_HOLE: u8 = 1 << 0;
pub const NBD_STATE_ZERO: u8 = 1 << 1;

// Handshake flag bits
pub const NBD_FLAG_FIXED_NEWSTYLE: u8 = 1 << 0;
pub const NBD_FLAG_C_FIXED_NEWSTYLE: u8 = 1 << 0;
pub const NBD_FLAG_NO_ZEROES: u8 = 1 << 1;
pub const NBD_FLAG_C_NO_ZEROES: u8 = 1 << 1;

// Transmission Flag bits
pub const NBD_FLAG_HAS_FLAGS: u16 = 1 << 0;
pub const NBD_FLAG_READ_ONLY: u16 = 1 << 1;
pub const NBD_FLAG_SEND_FLUSH: u16 = 1 << 2;
pub const NBD_FLAG_SEND_FUA: u16 = 1 << 3;
pub const NBD_FLAG_ROTATIONAL: u16 = 1 << 4;
pub const NBD_FLAG_SEND_TRIM: u16 = 1 << 5;
pub const NBD_FLAG_SEND_WRITE_ZEROES: u16 = 1 << 6;
pub const NBD_FLAG_SEND_DF: u16 = 1 << 7;
pub const NBD_FLAG_CAN_MULTI_CONN: u16 = 1 << 8;
pub const NBD_FLAG_SEND_RESIZE: u16 = 1 << 9;
pub const NBD_FLAG_SEND_CACHE: u16 = 1 << 10;
