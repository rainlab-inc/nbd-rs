#![allow(unused_variables)]

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    time::{SystemTime, UNIX_EPOCH},
    collections::{HashMap},
};

use clap::{App, Arg, crate_authors, crate_version};

use crate::storage::StorageBackend;

use log;
use env_logger;

// https://github.com/NetworkBlockDevice/nbd/blob/master/nbd-server.c#L2362-L2468
// NBD_OPT_GO | NBD_OPT_INFO: https://github.com/NetworkBlockDevice/nbd/blob/master/nbd-server.c#L2276-L2353

mod object;
mod storage;
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

#[derive(Hash)]
struct NBDExportConfig {
    name: String,
    driver_type: String,
    conn_str: String
}

impl NBDExportConfig {
    fn new(name: String, driver_type: String, conn_str: String) -> NBDExportConfig {
        if !["raw", "sharded"].contains(&driver_type.as_str()) {
            panic!("Driver must be one of the values `raw` or `sharded`. Found '{}'", driver_type);
        }
        println!("+ export {:?} -> {:?}({:?})", &name, &driver_type, &conn_str);
        NBDExportConfig {
            name: name,
            driver_type: driver_type,
            conn_str: conn_str
        }
    }
}

struct NBDSession {
    socket: TcpStream,
    flags: [bool; 2],
    structured_reply: bool,
    driver: Option<Box<dyn storage::StorageBackend>>,
    driver_name: String,
    // TODO: contexts: list of active contexts with attached metadata_context_ids
    metadata_context_id: u32,
    //request: Option<NBDRequest>,
    //option: Option<NBDOption>, // addr: SocketAddr,
                               // socket
                               // inputQueue
                               // remote
}

impl<'a> NBDSession {
    fn new(
        socket: TcpStream,
        flags: [bool; 2],
        structured_reply: bool,
        driver_name: String,
        image_name: String,
        metadata_context_id: u32,
        storage_config: String
    ) -> NBDSession {
        let mut session = NBDSession {
            socket: socket,
            flags: flags,
            structured_reply: structured_reply,
            driver: None,
            metadata_context_id: metadata_context_id,
            driver_name: driver_name.clone()
        };
        if driver_name.as_str() != "" {
            let driver = NBDSession::init_storage_driver(image_name, driver_name, storage_config);
            session.driver = driver;
        }
        session
    }

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

    fn init_storage_driver(image_name: String, driver_name: String, storage_config: String) -> Option<Box<dyn StorageBackend>> {
        match driver_name.as_str() {
            "raw" => {
                Some(Box::new(storage::RawBlock::new(image_name.clone(), storage_config)))
            },
            "sharded" => {
                Some(Box::new(storage::ShardedBlock::new(image_name.to_lowercase().clone(), storage_config)))
            },
            _ => {
                println!("Couldn't find storage driver: <{}>", driver_name);
                None
            }
        }
    }
}

struct NBDServer {
    addr: SocketAddr,
    socket: TcpListener,
    session: Option<NBDSession>,
    host: String,
    port: u16,
    exports: HashMap<String, NBDExportConfig>
}

impl<'a> NBDServer {
    fn new(host: String, port: u16, exports: HashMap<String, NBDExportConfig>) -> NBDServer {
        let addr: SocketAddr = format!("{}:{}", host, port).parse().unwrap();
        let socket_addr = addr.clone();

        NBDServer {
            addr,
            socket: TcpListener::bind(socket_addr).unwrap(),
            session: None,
            host,
            port,
            exports
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
        let session = NBDSession::new(
            socket,
            flags,
            false,
            String::from(""),
            String::from(""),
            0,
            String::from("")
        );
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
        let structured_reply: bool = {
            let session = self.session.as_ref().unwrap();
            session.structured_reply
        };
        match req_type {
            proto::NBD_CMD_READ => { // 0
                println!("NBD_CMD_READ");
                println!("\t-->flags:{}, handle: {}, offset: {}, datalen: {}", flags, handle, offset, datalen);
                let session = self.session.as_ref().unwrap();
                println!("STRUCTURED REPLY: {}", structured_reply);
                let driver = session.driver.as_ref().unwrap();
                let buffer = driver.read(offset, datalen as usize);
                if structured_reply == true {
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
                socket.write(&buffer.unwrap()).expect("Couldn't send data.");
            }
            proto::NBD_CMD_WRITE => { // 1
                println!("NBD_CMD_WRITE");
                println!("\t-->flags:{}, handle: {}, offset: {}, datalen: {}", flags, handle, offset, datalen);
                let mut data = vec![0; datalen as usize];
                match clone_stream!(socket).read_exact(&mut data) {
                    Ok(_) => {
                        let mut session = self.session.take().unwrap();
                        let mut driver = session.driver.take().unwrap();
                        let driver_name = driver.get_name();

                        // TODO: Handle errors
                        driver.write(offset, datalen as usize, &data)
                            .unwrap(); // panics on error

                        if structured_reply == true {
                            NBDServer::structured_reply(
                                clone_stream!(socket),
                                proto::NBD_REPLY_FLAG_DONE,
                                proto::NBD_REPLY_TYPE_NONE,
                                handle,
                                0
                            );
                        } else {
                            NBDServer::simple_reply(
                                clone_stream!(socket),
                                0_u32,
                                handle
                            );
                        }
                        self.session = Some(NBDSession {
                            socket: clone_stream!(socket),
                            flags: session.flags,
                            structured_reply: session.structured_reply,
                            driver: Some(driver),
                            driver_name: driver_name,
                            metadata_context_id: session.metadata_context_id
                        });
                    },
                    Err(e) => {
                        eprintln!("{}", e);
                        if structured_reply == true {
                            let err_msg = b"Could not receive the data. Please try again later";
                            NBDServer::structured_reply(
                                clone_stream!(socket),
                                proto::NBD_REPLY_FLAG_DONE,
                                proto::NBD_REPLY_TYPE_ERROR,
                                handle,
                                6 + err_msg.len() as u32
                            );
                            util::write_u32(proto::NBD_REP_ERR_UNKNOWN, clone_stream!(socket));
                            util::write_u16(err_msg.len() as u16, clone_stream!(socket));
                            write!(err_msg, socket);
                        } else {
                            NBDServer::simple_reply(
                                clone_stream!(socket),
                                proto::NBD_REP_ERR_UNKNOWN,
                                handle
                            );
                        }
                    }
                }
            }
            proto::NBD_CMD_DISC => { // 2
                // Terminate TLS
                println!("NBD_CMD_DISC");
                let mut session = self.session.take().unwrap();
                let driver = session.driver.take();
                if driver.is_some() {
                    driver.unwrap().close();
                }
                self.session = Some(NBDSession::new(
                    clone_stream!(socket),
                    session.flags,
                    session.structured_reply,
                    String::from(""),
                    String::from(""),
                    session.metadata_context_id,
                    String::from("")
                ));
            }
            proto::NBD_CMD_FLUSH => { // 3
                println!("NBD_CMD_FLUSH");
                let mut session = self.session.take().unwrap();
                let mut driver = session.driver.take().unwrap();
                let driver_name = driver.get_name();
                match driver.flush(0, driver.get_volume_size() as usize) {
                    Ok(_) => println!("flushed"),
                    Err(e) => eprintln!("{}", e)
                }
                if structured_reply == true {
                    NBDServer::structured_reply(
                        clone_stream!(socket),
                        proto::NBD_REPLY_FLAG_DONE,
                        proto::NBD_REPLY_TYPE_NONE,
                        handle,
                        0
                    );
                } else {
                    NBDServer::simple_reply(clone_stream!(socket), 0_u32, handle);
                }
                self.session = Some(NBDSession {
                    socket: clone_stream!(socket),
                    flags: session.flags,
                    structured_reply: session.structured_reply,
                    driver: Some(driver),
                    driver_name: driver_name,
                    metadata_context_id: session.metadata_context_id
                });
            }
            proto::NBD_CMD_BLOCK_STATUS => { // 7
                // fsync
                let session = self.session.as_ref().unwrap();
                let single_extent_only = (flags & proto::NBD_CMD_FLAG_REQ_ONE) != 0;
                println!("NBD_CMD_BLOCK_STATUS");
                println!("\t-->flags:{}, handle: {}, offset: {}, datalen: {}", flags, handle, offset, datalen);
                NBDServer::structured_reply(
                    clone_stream!(socket),
                    proto::NBD_REPLY_FLAG_DONE,
                    proto::NBD_REPLY_TYPE_BLOCK_STATUS,
                    handle,
                    12 //datalen
                );
                util::write_u32(session.metadata_context_id, clone_stream!(socket));
                util::write_u32(datalen, clone_stream!(socket));
                util::write_u32(0, clone_stream!(socket));
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
        // FIXME! selected_export could be None. a proper error must be returned
        let selected_export = self.exports.get_key_value(&name.to_lowercase()).unwrap();
        let session = self.session.as_ref().unwrap();
        let mut volume_size: u64 = 256 * 1024 * 1024;
        if session.driver.is_none() {
            let session = self.session.as_ref().unwrap();
            self.session = Some(NBDSession::new(
                clone_stream!(socket),
                session.flags,
                session.structured_reply,
                selected_export.1.driver_type.clone(),
                String::from(selected_export.0),
                session.metadata_context_id,
                selected_export.1.conn_str.clone()
            ));
            volume_size = self.session.as_ref().unwrap().driver.as_ref().unwrap().get_volume_size();
        } else {
            volume_size = self.session.as_ref().unwrap().driver.as_ref().unwrap().get_volume_size();
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
        if (opt == proto::NBD_OPT_GO) & (self.session.as_ref().unwrap().driver.is_none()) {
            let selected_export = self.exports.get_key_value(&name.to_lowercase()).unwrap();
            let session = self.session.take().unwrap();
            self.session = Some(NBDSession::new(
                clone_stream!(socket),
                session.flags,
                session.structured_reply,
                selected_export.1.driver_type.clone(),
                String::from(selected_export.0),
                session.metadata_context_id,
                selected_export.1.conn_str.clone()
            ));
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

    /*
    fn get_driver_type_from_export_name(&self, export_name: String) -> String {
        let exports: Vec<&NBDExportConfig> = self.exports
            .iter()
            .filter(|export| export.export_name.to_lowercase() == export_name.to_lowercase())
            .collect();
        let export = exports.first();
        if export.is_none() {
            return String::from("");
        }
        export.unwrap().driver_type.clone()
    }
    */
}

fn main() {
    env_logger::init();
    log::trace!("Parsing arguments");
    let matches = App::new("nbd-rs")
        .about("NBD Server written in Rust.")
        .author(crate_authors!())
        .version(crate_version!())
        .arg(
            Arg::with_name("export")
                .short("e")
                .long("export")
                .takes_value(true)
                .value_names(&["export_name", "driver", "conn_str"])
                .required(true)
                .multiple(true)
                .number_of_values(3)
                .long_help(
"USAGE:
[-e | --export EXPORT_NAME; DRIVER (raw, sharded); CONN_STR]...
Sets the export(s) via `export` argument. Must be used at least once."),
        )
        .get_matches();
    let vals: Vec<String> = matches.values_of("export")
        .unwrap()
        .map(|val| val.to_string())
        .collect();
    let mut exports = HashMap::<String, NBDExportConfig>::new();
    for i in 0..(matches.occurrences_of("export") as usize) {
        exports.insert(vals[i * 3].clone(),
            NBDExportConfig::new(
                vals[i * 3].clone(),
                vals[i * 3 + 1].clone(),
                vals[i * 3 + 2].clone()
            )
        );
    }
    let mut server = NBDServer::new("0.0.0.0".to_string(), 10809, exports);
    server.listen();
}
