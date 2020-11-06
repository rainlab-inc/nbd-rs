// Oldstyle negotiation
//S: 64 bits, 0x4e42444d41474943 (ASCII 'const NBD_AGIC') (also known as the INIT_PASSWD)
//S: 64 bits, 0x00420281861253 (cliserv_magic, a magic number)
//S: 64 bits, size of the export in bytes (unsigned)
//S: 32 bits, flags
//S: 124 bytes, zeroes (reserved).
const NBD_INIT_PASSWD: u64 = b'NBD_MAGIC';

// Newstyle negotiation
//S: 64 bits, 0x4e42444d41474943 (ASCII 'const NBD_AGIC') (as in the old style handshake)
//S: 64 bits, 0x49484156454F5054 (ASCII 'IHAVEOPT_) (note different magic number)
//S: 16 bits, handshake flags
//C: 32 bits, client flags

const NBD_OPT_EXPORT_NAME = 1;
const NBD_OPT_ABORT = 2;
const NBD_OPT_LIST = 3;
const NBD_OPT_PEEK_EXPORT = 4;
const NBD_OPT_STARTTLS = 5;

const NBD_OPT_INFO = 6;
const NBD_OPT_GO = 7; // ( same as const NBD_OPT_INFO )
	// Data (both commands, const NBD_OPT_INFO, const NBD_OPT_GO):
	//
	// 32 bits, length of name (unsigned); MUST be no larger than the OPT_on data length - 6
	// String: name of the export
	// 16 bits, number of information requests
	// 16 bits x n - list of const NBD_INFO information requests
const NBD_OPT_STRUCTURED_REPLY = 8;
const NBD_OPT_LIST_META_CONTEXT = 9;
	// Data:
	//
	// 32 bits, length of export name.
	// String, name of export for which we wish to list metadata contexts.
	// 32 bits, number of queries
	// Zero or more queries, each being:
	// 32 bits, length of query.
	// String, query to list a subset of the available metadata contexts. The syntax of this query is implementation-defined, except that it MUST start with a namespace and a colon.

const NBD_OPT_SET_META_CONTEXT = 10;

const NBD_INFO_EXPORT = 0;
const NBD_INFO_NAME = 1;
const NBD_INFO_DESCRIPTION = 2;
const NBD_INFO_BLOCK_SIZE = 3;

const NBD_CMD_READ = 0;
const NBD_CMD_WRITE = 1;
const NBD_CMD_DISC = 2; // disconnect
const NBD_CMD_FLUSH = 3;
const NBD_CMD_TRIM = 4;
const NBD_CMD_CACHE = 5;
const NBD_CMD_WRITE_ZEROES = 6;
const NBD_CMD_BLOCK_STATUS = 7;
const NBD_CMD_RESIZE = 8;

// command flags
const NBD_CMD_FLAG_FUA = 1<<0;
const NBD_CMD_FLAG_NO_HOLE = 1<<1;
const NBD_CMD_FLAG_DF = 1<<2;
const NBD_CMD_FLAG_REQ_ONE = 1<<3;
// structured reply flags
const NBD_REPLY_FLAG_DONE = 1<<0;

const NBD_EPERM = 1;
const NBD_EIO = 5;
const NBD_ENOMEM = 12;
const NBD_EINVAL = 22;
const NBD_ENOSPC = 28;
const NBD_EOVERFLOW = 75;
const NBD_ESHUTDOWN = 108;

const NBD_REPLY_TYPE = {}
const NBD_REPLY_TYPE.NONE = 0;
const NBD_REPLY_TYPE.OFFSET_DATA = 1;
const NBD_REPLY_TYPE.OFFSET_HOLE = 2;
const NBD_REPLY_TYPE.BLOCK_STATUS = 5;
const NBD_REPLY_TYPE.ERROR = 2^15 + 1;
const NBD_REPLY_TYPE.ERROR_OFFSET = 2^15 + 2;

const NBD_REP_ACK = 1;
const NBD_REP_SERVER = 2;
const NBD_REP_INFO = 3;
const NBD_REP_META_CONTEXT = 4;

const NBD_INFO_EXPORT = 0;
const NBD_INFO_NAME = 1;
const NBD_INFO_DESCRIPTION = 2;
const NBD_INFO_BLOCK_SIZE = 3;

const NBD_REP_ERR = 2**31;
const NBD_REP_ERR_UNSUP = NBD_REP_ERR + 1;
const NBD_REP_ERR_POLICY = NBD_REP_ERR + 2;
const NBD_REP_ERR_INVALID = NBD_REP_ERR + 3;
const NBD_REP_ERR_PLATFORM = NBD_REP_ERR + 4;
const NBD_REP_ERR_TLS_REQD = NBD_REP_ERR + 5;
const NBD_REP_ERR_UNKNOWN = NBD_REP_ERR + 6;
const NBD_REP_ERR_SHUTDOWN = NBD_REP_ERR + 7;
const NBD_REP_ERR_BLOCK_SIZE_REQD = NBD_REP_ERR + 8;
const NBD_REP_ERR_TOO_BIG = NBD_REP_ERR + 9;

const NBD_STATE_HOLE = 1<<0;
const NBD_STATE_ZERO = 1<<1;

// Handshake flag bits
const NBD_FLAG_FIXED_NEWSTYLE = 1<<0;
const NBD_FLAG_C_FIXED_NEWSTYLE = 1<<0;
const NBD_FLAG_NO_ZEROES = 1<<1;
const NBD_FLAG_C_NO_ZEROES = 1<<1;

// Transmission Flag bits
const NBD_FLAG_HAS_FLAGS = 1<<0;
const NBD_FLAG_READ_ONLY = 1<<1;
const NBD_FLAG_SEND_FLUSH = 1<<2;
const NBD_FLAG_SEND_FUA = 1<<3;
const NBD_FLAG_ROTATIONAL = 1<<4;
const NBD_FLAG_SEND_TRIM = 1<<5;
const NBD_FLAG_SEND_WRITE_ZEROES = 1<<6;
const NBD_FLAG_SEND_DF = 1<<7;
const NBD_FLAG_CAN_MULTI_CONN = 1<<8;
const NBD_FLAG_SEND_RESIZE = 1<<9;
const NBD_FLAG_SEND_CACHE = 1<<10;
