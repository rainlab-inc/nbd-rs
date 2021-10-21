use mmap_safe::MappedFile;
// use std::io;
// use std::io::prelude::*;
use std::{
    io::{Read, Write},
    fs::{OpenOptions},
    net::{SocketAddr, TcpListener, TcpStream},
    time::{SystemTime, UNIX_EPOCH}
};
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

struct NBDExport {
    name: String,
    volume_size: u64,
    pointer: MappedFile
}

struct NBDSession {
    socket: TcpStream,
    flags: [bool; 2],
    structured_reply: bool,
    export: Option<NBDExport>,
    // TODO: contexts: list of active contexts with attached metadata_context_ids
    metadata_context_id: u32,
    //request: Option<NBDRequest>,
    //option: Option<NBDOption>, // addr: SocketAddr,
                               // socket
                               // inputQueue
                               // remote
}

impl NBDSession {
    fn set_structured_reply(self) -> Self {
        NBDSession {
            structured_reply: true,
            ..self
        }
    }

    fn set_metadata_context_id(self, metadata_context_id: u32) -> Self {
        NBDSession {
            metadata_context_id: metadata_context_id,
            ..self
        }
    }

    fn mmap_file(self, name: String) -> Option<NBDSession> {
        let f = OpenOptions::new()
            .read(true)
            .open(name.clone())
            .expect("Unable to open file");
        let volume_size = f.metadata().unwrap().len();
        println!("Volume Size of export {} is: <{}>", name.clone().to_lowercase(), volume_size);
        let data = MappedFile::new(f).unwrap();
        let export = NBDExport {
            name: name,
            volume_size: volume_size,
            pointer: data
        };
        Some(NBDSession{
            export: Some(export),
            ..self
        })
    }
}

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
            flags: flags,
            structured_reply: false,
            export: None,
            metadata_context_id: 0,
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
                            "Error at NBD_IHAVEOPT. Expected: 0x49484156454F5054 Got: {:?}",
                            format!("{:#X}", e)
                        );
                        break;
                    }
                },
                0x0 => {
                    println!("Terminating connection.");
                    break;
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

    fn handle_request(&mut self, mut socket: TcpStream) {
        let flags = util::read_u16(clone_stream!(socket));
        let req_type = util::read_u16(clone_stream!(socket));
        let handle = util::read_u64(clone_stream!(socket));
        let offset = util::read_u64(clone_stream!(socket));
        let datalen = util::read_u32(clone_stream!(socket));
        match req_type {
            proto::NBD_CMD_READ => { // 0
                println!("NBD_CMD_READ");
                println!("\t-->flags:{}, handle: {}, offset: {}, datalen: {}", flags, handle, offset, datalen);
                let session = self.session.as_ref().unwrap();
                println!("STRUCTURED REPLY: {}", session.structured_reply);
                let export = session.export.as_ref().unwrap();
                let buffer = export.pointer.map(offset, datalen as usize).unwrap();
                if session.structured_reply == true {
                    NBDServer::structured_reply(
                        clone_stream!(socket),
                        proto::NBD_REPLY_FLAG_DONE,
                        proto::NBD_REPLY_TYPE_OFFSET_DATA,
                        handle,
                        8 + datalen
                    );
                    util::write_u64(offset, clone_stream!(socket));
                } else {
                    NBDServer::simple_reply(clone_stream!(socket), 0_u32, handle);
                }
                socket.write(&buffer).expect("Couldn't send data.");
            }
            proto::NBD_CMD_DISC => { // 2
                // Terminate TLS
                println!("NBD_CMD_DISC");
            }
            _ => {
                eprintln!("Invalid/Unimplemented CMD: {:?}", req_type);
                NBDServer::simple_reply(clone_stream!(socket), proto::NBD_REP_ERR_UNSUP, handle);
            }
        }
    }

    fn simple_reply(socket: TcpStream, err_code: u32, handle: u64) {
        util::write_u32(0x67446698_u32, clone_stream!(socket)); // SIMPLE REPLY MAGIC
        util::write_u32(err_code, clone_stream!(socket));
        util::write_u64(handle, clone_stream!(socket));
    }

    fn structured_reply(socket: TcpStream, flags: u16, reply_type: u16, handle: u64, length_of_payload: u32) {
        util::write_u32(0x668e33ef_u32, clone_stream!(socket)); // STRUCTURED REPLY MAGIC
        util::write_u16(flags, clone_stream!(socket));
        util::write_u16(reply_type, clone_stream!(socket));
        util::write_u64(handle, clone_stream!(socket));
        util::write_u32(length_of_payload, clone_stream!(socket));
    }

    /*
    fn structured_reply() {
        util::write_u32(0x668e33ef_u32, clone_stream!(socket)); // NBD_STRUCTURED_REPLY_MAGIC
        let reply_flag = if is_final { proto::NBD_REPLY_FLAG_DONE as u16 } else { 0 };
        util::write_u16(reply_flag, clone_stream!(socket));
        util::write_u16(rep_type, clone_stream!(socket));
        util::write_u64(handle, clone_stream!(socket));
        util::write_u32(length, clone_stream!(socket));
    }
    */

    fn handle_option(&mut self, socket: TcpStream) {
        let option = util::read_u32(clone_stream!(socket));
        println!("Option: {}", option);
        match option {
            proto::NBD_OPT_ABORT => {// 2
                self.handle_opt_abort(clone_stream!(socket));
                //println!("Shutting down connection...")'
                //socket.shutdown(Shutdown::Both).expect("Shutdown failed");
                //break;
            }
            proto::NBD_OPT_INFO | proto::NBD_OPT_GO => {// 6, 7
                self.handle_opt_info_go(clone_stream!(socket), option);
            }
            proto::NBD_OPT_STRUCTURED_REPLY => {// 8
                let data = util::read_u32(clone_stream!(socket));
                if data > 0 {
                    println!("{}", data);
                    self.reply(
                        clone_stream!(socket),
                        proto::NBD_OPT_STRUCTURED_REPLY,
                        proto::NBD_REP_ERR_INVALID,
                        0
                    );
                } else {
                    self.reply(
                        clone_stream!(socket),
                        proto::NBD_OPT_STRUCTURED_REPLY,
                        proto::NBD_REP_ACK,
                        0
                    );
                    let session = self.session.take();
                    self.session = Some(session.unwrap().set_structured_reply());
                }
            }
            proto::NBD_OPT_SET_META_CONTEXT => {// 10
                self.handle_opt_set_meta_context(clone_stream!(socket));
            }
            _ => {
                eprintln!("Invalid/Unimplemented OPT: {:?}", option);
                self.reply_opt(
                    clone_stream!(socket),
                    option,
                    proto::NBD_REP_ERR_UNSUP,
                    0,
                );
            }
        }
    }

    fn reply(&mut self, socket: TcpStream, opt: u32, reply_type: u32, len: u32) {
        util::write_u64(0x3e889045565a9, clone_stream!(socket)); // REPLY MAGIC
        util::write_u32(opt, clone_stream!(socket));
        util::write_u32(reply_type, clone_stream!(socket));
        util::write_u32(len, socket);
    }

    fn reply_info_export(&mut self, socket: TcpStream, opt: u32, name: String) {
        self.reply(
            clone_stream!(socket),
            opt,
            proto::NBD_REP_INFO,
            12
        );
        let session = self.session.as_ref().unwrap();
        let mut volume_size: u64 = 256 * 1024 * 1024;
        if session.export.is_none() {
            let session = self.session.take();
            self.session = session.unwrap().mmap_file(name.clone().to_lowercase());
            volume_size = self.session.as_ref().unwrap().export.as_ref().unwrap().volume_size;
        } else {
            let session = self.session.as_ref().unwrap();
            volume_size = self.session.as_ref().unwrap().export.as_ref().unwrap().volume_size;
        }
        let flags: u16 = proto::NBD_FLAG_HAS_FLAGS | proto::NBD_FLAG_SEND_FLUSH | proto::NBD_FLAG_SEND_RESIZE | proto::NBD_FLAG_SEND_WRITE_ZEROES | proto::NBD_FLAG_SEND_CACHE | proto::NBD_FLAG_SEND_TRIM;
        util::write_u16(proto::NBD_INFO_EXPORT, clone_stream!(socket));
        util::write_u64(volume_size, clone_stream!(socket));
        util::write_u16(flags, clone_stream!(socket));
        println!("\t-->Export Data Sent:");
        println!("\t-->\t-->NBD_INFO_EXPORT: \t{:?}", proto::NBD_INFO_EXPORT as u16);
        println!("\t-->\t-->Volume Size: \t{:?}", volume_size as u32);
        println!("\t-->\t-->Transmission Flags: \t{:?}", flags as u16);
    }

    fn reply_opt(
        &mut self,
        mut socket: TcpStream,
        opt: u32,
        reply_type: u32,
        len: u32,
    ) {
        let data_permitted = vec![1, 6, 7, 9, 10].contains(&opt); // OPTION CODES THAT IS ALLOWED TO CARRY DATA
        self.reply(clone_stream!(socket), opt, reply_type, 4 + len);
        util::write_u32(len, clone_stream!(socket));
        println!(" -> Option: {:?}, Option length: {:?}, Data permitted: {:?}", opt, len, data_permitted);

        if len > 0 {
            let mut data = vec![0; len as usize];
            clone_stream!(socket) // Docs says server should not reject the data even if not needed
                .read_exact(&mut data)
                .expect("Error on reading Option Data!");
            println!("\t\\-> Data: {:?}", data);
            if data_permitted {
                write!(&data, socket);
                println!("Data sent for option {:?}", opt);
            }
        }
    }

    /*
    fn handle_opt_export(&mut self, socket: TcpStream) {
        //let name = u32::to_string(&datalen);
        println!("Working on it...");
    }
    */

    fn handle_opt_abort(&mut self, _socket: TcpStream) {
        /*
        let len = util::read_u32(clone_stream!(socket));
        self.reply_opt(
            clone_stream!(socket),
            proto::NBD_OPT_ABORT,
            proto::NBD_REP_ERR_UNSUP,
            len,
        );
        */
    }

    fn handle_opt_info_go(&mut self, mut socket: TcpStream, opt: u32) {
        let _len = util::read_u32(clone_stream!(socket));
        let namelen = util::read_u32(clone_stream!(socket));
        // if namelen > len - 6 { return NBD_EINVAL }
        let name = match namelen {
            0 => "default".to_string(),
            _ => util::read_string(namelen as usize, clone_stream!(socket)),
        };
        let info_req_count = util::read_u16(clone_stream!(socket));
        let mut info_reqs = Vec::new();
        for _ in 0..info_req_count {
            info_reqs.push(util::read_u16(clone_stream!(socket)));
        }
        println!("len:{},namelen:{},name:{},info_req_count:{}", _len, namelen, name, info_req_count);
        println!("\t-->Info Requests: {:?}", info_reqs);

        if info_reqs.is_empty() { //The client MAY list one or more items of specific information it is seeking in the list of information requests, or it MAY specify an empty list.
            info_reqs.push(3_u16);
        }

        for req in &info_reqs {
            match *req {
                proto::NBD_INFO_EXPORT => {// 0
                    self.reply_info_export(clone_stream!(socket), opt, name.clone());
                }
                proto::NBD_INFO_NAME => {// 1
                    self.reply(
                        clone_stream!(socket),
                        opt,
                        proto::NBD_REP_INFO,
                        0
                    );
                    util::write_u16(proto::NBD_INFO_NAME, clone_stream!(socket));
                    write!(name.as_bytes(), socket);
                }
                proto::NBD_INFO_DESCRIPTION => {// 2
                    let name_as_bytes = name.as_bytes();
                    let length_of_name = name_as_bytes.len();
                    self.reply(
                        clone_stream!(socket),
                        opt,
                        proto::NBD_REP_INFO,
                        2 + length_of_name as u32
                    );
                    util::write_u16(proto::NBD_INFO_DESCRIPTION, clone_stream!(socket));
                    write!(name_as_bytes, socket);
                }
                proto::NBD_INFO_BLOCK_SIZE => {// 3
                    self.reply(
                        clone_stream!(socket),
                        opt,
                        proto::NBD_REP_INFO,
                        14
                    );
                    util::write_u16(proto::NBD_INFO_BLOCK_SIZE, clone_stream!(socket));
                    util::write_u32(512 as u32, clone_stream!(socket));
                    util::write_u32(4*1024 as u32, clone_stream!(socket));
                    util::write_u32(32*1024*1024 as u32, clone_stream!(socket));
                    println!("\t-->Sent block size info");
                    self.reply_info_export(clone_stream!(socket), opt, name.clone());
                }
                r @ _ => {
                    eprintln!("Invalid Info Request: {:?}", r);
                    self.reply_opt(
                        clone_stream!(socket),
                        proto::NBD_REP_INFO,
                        proto::NBD_EINVAL as u32,
                        0
                    )
                }
            }
        }

        self.reply(
            clone_stream!(socket),
            opt,
            proto::NBD_REP_ACK,
            0
        );
        if opt == proto::NBD_OPT_GO {
            let session = self.session.as_ref().unwrap();
            if session.export.is_none() {
                let session = self.session.take();
                self.session = session.unwrap().mmap_file(name.clone().to_lowercase());
            }
        }
    }


    fn handle_opt_set_meta_context(&mut self, socket: TcpStream) {
        let total_length = util::read_u32(clone_stream!(socket));
        let export_name_length = util::read_u32(clone_stream!(socket));
        let export_name = match export_name_length {
            0 => "default".to_string(),
            _ => util::read_string(export_name_length as usize, clone_stream!(socket)),
        };
        let number_of_queries = util::read_u32(clone_stream!(socket));
        println!("\t-->total_length: {}, export_name_length: {}, export_name: {}, number_of_queries: {}", total_length, export_name_length, export_name, number_of_queries);
        if number_of_queries > 0 {
            for i in 0..number_of_queries {
                let query_length = util::read_u32(clone_stream!(socket));
                let query = util::read_string(query_length as usize, clone_stream!(socket));
                println!("\t-->\t-->iter: {}, query: {}", i + 1, query);
                self.reply(
                    clone_stream!(socket),
                    proto::NBD_OPT_SET_META_CONTEXT,
                    proto::NBD_REP_META_CONTEXT,
                    4 + query.len() as u32
                );
                let nbd_metadata_context_id = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos();
                let session = self.session.take();
                self.session = Some(session.unwrap().set_metadata_context_id(nbd_metadata_context_id));
                util::write_u32(nbd_metadata_context_id, clone_stream!(socket));
                clone_stream!(socket).write(query.to_lowercase().as_bytes()).expect("Couldn't send query data");
            }
        }
        self.reply(
            clone_stream!(socket),
            proto::NBD_OPT_SET_META_CONTEXT,
            proto::NBD_REP_ACK,
            0
        );
    }
}

fn main() {
    let mut server = NBDServer::new("0.0.0.0".to_string(), 10809);
    server.listen();
}
