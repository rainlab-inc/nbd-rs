// use std::io;
// use std::io::prelude::*;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
//use std::collections::HashMap;

// https://github.com/NetworkBlockDevice/nbd/blob/master/nbd-server.c#L2362-L2468
// NBD_OPT_GO | NBD_OPT_INFO: https://github.com/NetworkBlockDevice/nbd/blob/master/nbd-server.c#L2276-L2353

mod proto;
mod util;

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

#[derive(Debug)]
struct NBDSession {
    socket: TcpStream,
    flags: [bool; 2],
    //request: Option<NBDRequest>,
    //option: Option<NBDOption>, // addr: SocketAddr,
                               // socket
                               // inputQueue
                               // remote
}

#[derive(Debug)]
struct NBDServer {
    addr: SocketAddr,
    socket: TcpListener,
    session: Option<NBDSession>,
    host: String,
    port: u16,
}

impl NBDServer {
    fn new(host: String, port: u16) -> NBDServer {
        let addr: SocketAddr = format!("{}:{}", host, port).parse().unwrap();
        let socket_addr = addr.clone();

        NBDServer {
            addr,
            socket: TcpListener::bind(socket_addr).unwrap(),
            session: None,
            host,
            port,
        }
    }

    fn listen(&mut self) {
        let hostport = format!("{}:{}", self.host, self.port);
        println!("Listening on {}", hostport);
        // let listener = self.socket;

        loop {
            // This part can be simplified with returning a result type and using `?` at the
            // end of .accept()?
            let (stream, _) = match self.socket.accept() {
                Ok(conn) => conn,
                Err(e) => {
                    eprintln!("failed to accept: {:?}", e);
                    break;
                }
            };
            self.handle_connection(clone_stream!(stream));
            self.transmission(stream);
        }

        println!("Done");
    }

    fn handle_connection(&mut self, socket: TcpStream /*, addr: SocketAddr*/) {
        // TODO: Process socket
        let flags = self.handshake(clone_stream!(socket));
        let session = NBDSession {
            socket: socket,
            flags,
            //request: None,
            //option: None,
            // addr,
        };
        self.session = Some(session);
        println!("Connection established!");
    }

    fn handshake(&mut self, mut socket: TcpStream) -> [bool; 2] {
        println!("Handshake started...");
        let newstyle = proto::NBD_FLAG_FIXED_NEWSTYLE;
        let no_zeroes = proto::NBD_FLAG_NO_ZEROES;
        let handshake_flags = (newstyle | no_zeroes) as u16;

        write!(b"NBDMAGIC", socket);
        write!(b"IHAVEOPT", socket);
        util::write_u16(handshake_flags, clone_stream!(socket));
        println!("Initial message sent");

        let client_flags = util::read_u32(clone_stream!(socket));
        let flags_list = [
            client_flags & (proto::NBD_FLAG_C_FIXED_NEWSTYLE as u32) != 0,
            client_flags & (proto::NBD_FLAG_C_NO_ZEROES as u32) != 0,
        ];

        println!(" -> fixedNewStyle: {}", flags_list[0]);
        println!(" -> noZeroes: {}", flags_list[1]);
        println!("Handshake done successfully!");
        flags_list
    }

    fn transmission(&mut self, socket: TcpStream) {
        println!("Transmission started...");
        loop {
            let req = match util::read_u32(clone_stream!(socket)) {
                0x25609513 => 0x25609513, // NBD_REQUEST_MAGIC
                0x49484156 => match util::read_u32(clone_stream!(socket)) {
                    // "IHAV"
                    0x454F5054 => 0x49484156454F5054 as u64, // "IHAV + EOPT"
                    e => {
                        eprintln!(
                            "Error at NBD_IHAVEOPT. Expected: 0x25609513 Got: {:?}",
                            format!("{:#X}", e)
                        );
                        break;
                    }
                },
                e => {
                    eprintln!(
                        "Error at NBD_REQUEST_MAGIC. Expected: 0x25609513 Got: {:?}",
                        format!("{:#X}", e)
                    );
                    break;
                }
            };
            match req {
                0x25609513 => {
                    // NBD_REQUEST_MAGIC
                    self.handle_request(clone_stream!(socket));
                }
                0x49484156454F5054 => {
                    // IHAVEOPT
                    self.handle_option(clone_stream!(socket));
                }
                _ => (),
            }
        }
        println!("Transmission ended");
    }

    fn handle_request(&mut self, socket: TcpStream) {
        let _flags = util::read_u16(clone_stream!(socket));
        let _req_type = util::read_u16(clone_stream!(socket));
        let _handle = util::read_u8(clone_stream!(socket));
        let _offset_h = util::read_u32(clone_stream!(socket));
        let _offset_l = util::read_u32(clone_stream!(socket));
        let _offset = _offset_h * (2 << 31) + _offset_l;
        let _datalen = util::read_u32(clone_stream!(socket));
    }

    ///    fn structured_reply() {
    ///        util::write_u32(0x668e33ef_u32, clone_stream!(socket)); // NBD_STRUCTURED_REPLY_MAGIC
    ///        let reply_flag = if is_final { proto::NBD_REPLY_FLAG_DONE as u16 } else { 0 };
    ///        util::write_u16(reply_flag, clone_stream!(socket));
    ///        util::write_u16(rep_type, clone_stream!(socket));
    ///        util::write_u64(handle, clone_stream!(socket));
    ///        util::write_u32(length, clone_stream!(socket));
    ///    }

    fn handle_option(&mut self, socket: TcpStream) {
        let option = util::read_u32(clone_stream!(socket));
        let len = util::read_u32(clone_stream!(socket));
        let mut data = Vec::with_capacity(len as usize);
        clone_stream!(socket)
            .read_exact(&mut data)
            .expect("Error on reading Option Data!");
        match option {
            /*
            proto::NBD_OPT_GO | proto::NBD_OPT_INFO => {
                // 7, 6
                self.handle_opt_info(clone_stream!(socket), option);
            }
            proto::NBD_OPT_EXPORT_NAME => {
                // 1
                self.handle_opt_export(clone_stream!(socket));
            }
            proto::NBD_OPT_ABORT => {
                // 2
                self.handle_opt_abort(clone_stream!(socket));
            }
            proto::NBD_OPT_LIST => {
                // 3
                self.handle_opt_list(clone_stream!(socket));
            }
            */
            proto::NBD_OPT_STRUCTURED_REPLY => {
                // 8
                self.handle_opt_structured_reply(clone_stream!(socket), len);
            }
            proto::NBD_OPT_SET_META_CONTEXT => {
                self.handle_opt_set_meta_context(clone_stream!(socket));
            }
            1..=7 | 9 => eprintln!("Unimplemented OPT: {:?}", option),
            _ => eprintln!("Option req not found."),
        }
        println!("Option {:?}", option);
    }

    fn reply_opt(
        &mut self,
        mut socket: TcpStream,
        opt: u32,
        reply_type: u32,
        len: u32,
        data: Vec<u8>,
    ) {
        util::write_u64(0x3e889045565a9, clone_stream!(socket)); // REPLY MAGIC
        util::write_u32(opt, clone_stream!(socket));
        util::write_u32(reply_type, clone_stream!(socket));
        util::write_u32(len, clone_stream!(socket));
        if !data.is_empty() {
            write!(&data, socket);
        }
    }
    /*
    fn handle_opt_info(&mut self, socket: TcpStream, option: u32) {
        let datalen = util::read_u32(clone_stream!(socket));
        let namelen = util::read_u32(clone_stream!(socket));
        let name = util::read_string(namelen as usize, clone_stream!(socket));
        let info_req_count = util::read_u16(clone_stream!(socket));
        let mut info_reqs = Vec::new();
        for _ in 0..info_req_count {
            info_reqs.push(util::read_u16(clone_stream!(socket)));
        }
        let _nbd_opt_obj = NBDOption {
            option,
            datalen,
            name,
            info_reqs,
        };
    }
    */

    /*
    fn handle_opt_export(&mut self, socket: TcpStream) {
        //let name = u32::to_string(&datalen);
        println!("Working on it...");
    }
    */

    /*
    fn handle_opt_abort(&mut self, socket: TcpStream) {
        println!("Working on it...");
    }
    */

    /*
    fn handle_opt_list(&mut self, socket: TcpStream) {
        println!("Working on it...");
    }
    */

    fn handle_opt_structured_reply(&mut self, socket: TcpStream, len: u32) {
        println!("Sending STRUCTURED REPLY ACK...");
        self.reply_opt(
            clone_stream!(socket),
            proto::NBD_OPT_STRUCTURED_REPLY as u32,
            proto::NBD_REP_ACK as u32,
            len,
            Vec::new(), // Empty Data
        );
        println!("Sent ACK.");
    }

    fn handle_opt_set_meta_context(&mut self, _socket: TcpStream) {
        println!("Implementing...");
    }
}

fn main() {
    let mut server = NBDServer::new("0.0.0.0".to_string(), 10891);
    server.listen();
}
