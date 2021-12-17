pub mod proto;

mod server;
pub use self::server::{NBDServer, NBDExport};

mod session;
pub use self::session::NBDSession;

/*
#[derive(Debug)]
struct NBDRequest {
    flags: u16,
    req_type: u16,
    handle: u8,
    offset_h: u32,
    offset_l: u32,
    offset: u64,
    datalen: u32,
    //data: [u8]
}

#[derive(Debug)]
struct NBDOption {
    option: u32,
    datalen: u32,
    name: String,
    info_reqs: Vec<u16>, //data: [u8]
}
*/
