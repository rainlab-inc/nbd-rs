#![allow(unused_variables, unused_imports)]

use std::{
    io::{Read, Write},
    net::{TcpStream},
    time::{SystemTime, UNIX_EPOCH},
    sync::{Arc, RwLock},
    cell::RefCell,
};

use crate::{
    block::{BlockStorage, block_storage_with_config},
    util,
    nbd::{proto, server}
};


pub struct NBDSession {
    pub socket: TcpStream,
    pub flags: [bool; 2],
    pub structured_reply: bool,
    pub selected_export: Option<Arc<RwLock<server::NBDExport>>>,
    pub driver_name: String,
    pub export_refs: Arc<RwLock<Vec<Arc<RwLock<server::NBDExport>>>>>,
    // TODO: contexts: list of active contexts with attached metadata_context_ids
    pub metadata_context_id: u32,
    //request: Option<NBDRequest>,
    //option: Option<NBDOption>, // addr: SocketAddr,
                               // socket
                               // inputQueue
                               // remote
}

impl NBDSession {
    pub fn new(
        socket: TcpStream,
        flags: [bool; 2],
        structured_reply: bool,
        driver_name: String,
        image_name: String,
        metadata_context_id: u32,
        storage_config: String,
        export_refs: Arc<RwLock<Vec<Arc<RwLock<server::NBDExport>>>>>
    ) -> NBDSession {
        NBDSession {
            socket: socket,
            flags: flags,
            structured_reply: structured_reply,
            selected_export: None,
            metadata_context_id: metadata_context_id,
            driver_name: driver_name.clone(),
            export_refs: export_refs
        }
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

    pub fn handle(&mut self) {
        log::debug!("Transmission");
        loop {
            let req = match util::read_u32(clone_stream!(self.socket)) {
                0x25609513 => 0x25609513, // NBD_REQUEST_MAGIC
                0x49484156 => match util::read_u32(clone_stream!(self.socket)) {
                    // "IHAV"
                    0x454F5054 => 0x49484156454F5054 as u64, // "IHAV + EOPT"
                    e => {
                        log::error!(
                            "Error at NBD_IHAVEOPT. Expected: 0x49484156454F5054 Got: {:?}",
                            format!("{:#X}", e)
                        );
                        break;
                    }
                },
                0x0 => {
                    log::info!("Terminating connection.");
                    break;
                },
                e => {
                    log::error!(
                        "Error at NBD_REQUEST_MAGIC. Expected: 0x25609513 Got: {:?}",
                        format!("{:#X}", e)
                    );
                    break;
                }
            };
            match req {
                0x25609513 => {
                    // NBD_REQUEST_MAGIC
                    self.handle_request();
                }
                0x49484156454F5054 => {
                    // IHAVEOPT
                    self.handle_option();
                }
                _ => (),
            }
        }
        log::info!("Transmission ended");
        let selected_export = self.selected_export.take();
        if selected_export.is_some() {
            let selected_export_un = selected_export.unwrap();
            let mut write_lock = selected_export_un.try_write().unwrap();
            {
                let mut driver = Arc::get_mut(&mut write_lock.driver).unwrap().try_write().unwrap();
                driver.close();
            }
            drop(write_lock);
        }
    }
    fn handle_request(&mut self) {
        let flags = util::read_u16(clone_stream!(self.socket));
        let req_type = util::read_u16(clone_stream!(self.socket));
        let handle = util::read_u64(clone_stream!(self.socket));
        let offset = util::read_u64(clone_stream!(self.socket));
        let datalen = util::read_u32(clone_stream!(self.socket));
        match req_type {
            proto::NBD_CMD_READ => { // 0
                log::debug!("NBD_CMD_READ");
                log::trace!("\t-->flags:{}, handle: {}, offset: {}, datalen: {}", flags, handle, offset, datalen);
                log::trace!("STRUCTURED REPLY: {}", self.structured_reply);
                let selected_export = self.selected_export.as_ref().unwrap();
                let mut write_lock = selected_export.try_write().unwrap();
                let driver = Arc::get_mut(&mut write_lock.driver).unwrap().try_write().unwrap();
                let buffer_res = driver.read(offset, datalen as usize);
                if buffer_res.is_err() {
                    // handle error
                    let err = buffer_res.err().unwrap().to_string();
                    log::warn!("NBD_CMD_READ failed: {}", err.clone());
                    let err_msg = err.as_bytes();
                    if self.structured_reply == true {
                        self.structured_reply(
                            proto::NBD_REPLY_FLAG_DONE,
                            proto::NBD_REPLY_TYPE_ERROR,
                            handle,
                            6 + err_msg.len() as u32
                        );
                        util::write_u32(proto::NBD_REP_ERR_UNKNOWN, clone_stream!(self.socket));
                        util::write_u16(err_msg.len() as u16, clone_stream!(self.socket));
                        write!(err_msg, self.socket);
                    } else {
                        self.simple_reply(
                            proto::NBD_REP_ERR_UNKNOWN,
                            handle
                        );
                    }
                } else {
                    log::trace!("NBD_CMD_READ ok!");
                    if self.structured_reply == true {
                        self.structured_reply(
                            proto::NBD_REPLY_FLAG_DONE,
                            proto::NBD_REPLY_TYPE_OFFSET_DATA,
                            handle,
                            8 + datalen
                        );
                        util::write_u64(offset, clone_stream!(self.socket));
                    } else {
                        self.simple_reply(0_u32, handle);
                    }
                    self.socket.write(&buffer_res.unwrap()).expect("Couldn't send data.");
                }
            }
            proto::NBD_CMD_WRITE => { // 1
                log::debug!("NBD_CMD_WRITE");
                log::trace!("\t-->flags:{}, handle: {}, offset: {}, datalen: {}", flags, handle, offset, datalen);
                let mut data = vec![0; datalen as usize];
                match clone_stream!(self.socket).read_exact(&mut data) {
                    Ok(_) => {
                        let selected_export = self.selected_export.as_ref().unwrap();
                        let mut write_lock = selected_export.try_write().unwrap();
                        let mut driver = Arc::get_mut(&mut write_lock.driver).unwrap().try_write().unwrap();
                        let driver_name = driver.get_name();

                        let write_res = driver.write(offset, datalen as usize, &data);
                        if write_res.is_err() {
                            // handle error
                            let err = write_res.err().unwrap().to_string();
                            log::warn!("NBD_CMD_WRITE failed: {}", err.clone());
                            let err_msg = err.as_bytes();
                            if self.structured_reply == true {
                                self.structured_reply(
                                    proto::NBD_REPLY_FLAG_DONE,
                                    proto::NBD_REPLY_TYPE_ERROR,
                                    handle,
                                    6 + err_msg.len() as u32
                                );
                                util::write_u32(proto::NBD_REP_ERR_UNKNOWN, clone_stream!(self.socket));
                                util::write_u16(err_msg.len() as u16, clone_stream!(self.socket));
                                write!(err_msg, self.socket);
                            } else {
                                self.simple_reply(
                                    proto::NBD_REP_ERR_UNKNOWN,
                                    handle
                                );
                            }
                        } else {
                            log::trace!("NBD_CMD_WRITE ok!");

                            if self.structured_reply == true {
                                self.structured_reply(
                                    proto::NBD_REPLY_FLAG_DONE,
                                    proto::NBD_REPLY_TYPE_NONE,
                                    handle,
                                    0
                                );
                            } else {
                                self.simple_reply(
                                    0_u32,
                                    handle
                                );
                            }
                        }
                    },
                    Err(e) => {
                        log::error!("{}", e);
                        if self.structured_reply == true {
                            let err_msg = b"Could not receive the data. Please try again later";
                            self.structured_reply(
                                proto::NBD_REPLY_FLAG_DONE,
                                proto::NBD_REPLY_TYPE_ERROR,
                                handle,
                                6 + err_msg.len() as u32
                            );
                            util::write_u32(proto::NBD_REP_ERR_UNKNOWN, clone_stream!(self.socket));
                            util::write_u16(err_msg.len() as u16, clone_stream!(self.socket));
                            write!(err_msg, self.socket);
                        } else {
                            self.simple_reply(
                                proto::NBD_REP_ERR_UNKNOWN,
                                handle
                            );
                        }
                    }
                }
            }
            proto::NBD_CMD_DISC => { // 2
                // Terminate TLS
                log::debug!("NBD_CMD_DISC");
                let selected_export = self.selected_export.take();
                if selected_export.is_some() {
                    let selected_export_un = selected_export.unwrap();
                    let mut write_lock = selected_export_un.try_write().unwrap();
                    {
                        let mut driver = Arc::get_mut(&mut write_lock.driver).unwrap().try_write().unwrap();
                        driver.close();
                    }
                    drop(write_lock);
                }
            }
            proto::NBD_CMD_FLUSH => { // 3
                log::debug!("NBD_CMD_FLUSH");
                let selected_export = self.selected_export.as_ref().unwrap();
                let mut write_lock = selected_export.try_write().unwrap();
                {
                    let mut driver = Arc::get_mut(&mut write_lock.driver).unwrap().try_write().unwrap();
                    let driver_name = driver.get_name();
                    let volume_size = driver.get_volume_size() as usize;
                    match driver.flush(0, volume_size) {
                        Ok(_) => log::trace!("flushed"),
                        Err(e) => log::error!("{}", e) // TODO: Reflect error to client
                    }
                }
                if self.structured_reply == true {
                    self.structured_reply(
                        proto::NBD_REPLY_FLAG_DONE,
                        proto::NBD_REPLY_TYPE_NONE,
                        handle,
                        0
                    );
                } else {
                    self.simple_reply(0_u32, handle);
                }
            }
            proto::NBD_CMD_TRIM => { // 4
                log::debug!("NBD_CMD_TRIM");
                let selected_export = self.selected_export.as_ref().unwrap();
                let mut write_lock = selected_export.try_write().unwrap();
                {
                    let mut driver = Arc::get_mut(&mut write_lock.driver).unwrap().try_write().unwrap();
                    let driver_name = driver.get_name();
                    let volume_size = driver.get_volume_size() as usize;
                    log::trace!("offset: {}, length: {}", offset, datalen);
                    match driver.trim(offset, datalen as usize) {
                        Ok(_) => log::trace!("trimmed"),
                        Err(e) => log::error!("{}", e) // TODO: Reflect error to client
                    }
                }
                if self.structured_reply == true {
                    self.structured_reply(
                        proto::NBD_REPLY_FLAG_DONE,
                        proto::NBD_REPLY_TYPE_NONE,
                        handle,
                        0
                    );
                } else {
                    self.simple_reply(0_u32, handle);
                }
            }
            proto::NBD_CMD_BLOCK_STATUS => { // 7
                // fsync
                log::debug!("NBD_CMD_BLOCK_STATUS");
                let single_extent_only = (flags & proto::NBD_CMD_FLAG_REQ_ONE) != 0;
                log::trace!("\t-->flags:{}, handle: {}, offset: {}, datalen: {}", flags, handle, offset, datalen);
                self.structured_reply(
                    proto::NBD_REPLY_FLAG_DONE,
                    proto::NBD_REPLY_TYPE_BLOCK_STATUS,
                    handle,
                    12 //datalen
                );
                util::write_u32(self.metadata_context_id, clone_stream!(self.socket));
                util::write_u32(datalen, clone_stream!(self.socket));
                util::write_u32(0, clone_stream!(self.socket));
            }
            _ => {
                log::warn!("Invalid/Unimplemented CMD: {:?}", req_type);
                self.simple_reply(proto::NBD_REP_ERR_UNSUP, handle);
            }
        }
    }

    fn simple_reply(&self, err_code: u32, handle: u64) {
        util::write_u32(0x67446698_u32, clone_stream!(self.socket)); // SIMPLE REPLY MAGIC
        util::write_u32(err_code, clone_stream!(self.socket));
        util::write_u64(handle, clone_stream!(self.socket));
    }

    fn structured_reply(&self, flags: u16, reply_type: u16, handle: u64, length_of_payload: u32) {
        util::write_u32(0x668e33ef_u32, clone_stream!(self.socket)); // STRUCTURED REPLY MAGIC
        util::write_u16(flags, clone_stream!(self.socket));
        util::write_u16(reply_type, clone_stream!(self.socket));
        util::write_u64(handle, clone_stream!(self.socket));
        util::write_u32(length_of_payload, clone_stream!(self.socket));
    }

    fn handle_option(&mut self) {
        let option = util::read_u32(clone_stream!(self.socket));
        log::debug!("Option: {}", option);
        match option {
            proto::NBD_OPT_ABORT => {// 2
                self.handle_opt_abort();
                //println!("Shutting down connection...")'
                //self.socket.shutdown(Shutdown::Both).expect("Shutdown failed");
                //break;
            }
            proto::NBD_OPT_INFO | proto::NBD_OPT_GO => {// 6, 7
                self.handle_opt_info_go(option);
            }
            proto::NBD_OPT_STRUCTURED_REPLY => {// 8
                let data = util::read_u32(clone_stream!(self.socket));
                if data > 0 {
                    log::trace!("{}", data);
                    self.reply(
                        proto::NBD_OPT_STRUCTURED_REPLY,
                        proto::NBD_REP_ERR_INVALID,
                        0
                    );
                } else {
                    self.reply(
                        proto::NBD_OPT_STRUCTURED_REPLY,
                        proto::NBD_REP_ACK,
                        0
                    );
                    self.structured_reply = true;
                }
            }
            proto::NBD_OPT_SET_META_CONTEXT => {// 10
                self.handle_opt_set_meta_context();
            }
            _ => {
                log::warn!("Invalid/Unimplemented OPT: {:?}", option);
                self.reply_opt(
                    option,
                    proto::NBD_REP_ERR_UNSUP,
                    0,
                );
            }
        }
    }

    fn reply(&mut self, opt: u32, reply_type: u32, len: u32) {
        util::write_u64(0x3e889045565a9, clone_stream!(self.socket)); // REPLY MAGIC
        util::write_u32(opt, clone_stream!(self.socket));
        util::write_u32(reply_type, clone_stream!(self.socket));
        util::write_u32(len, clone_stream!(self.socket));
    }

    fn reply_info_export(&mut self, opt: u32, export_name: String) {
        self.reply(
            opt,
            proto::NBD_REP_INFO,
            12
        );
        let volume_size: u64;
        if self.selected_export.is_none() {
            self.select_export(export_name);
        }
        let selected_export = self.selected_export.as_ref().unwrap();
        let mut write_lock = selected_export.try_write().unwrap();
        {
            let driver = Arc::get_mut(&mut write_lock.driver).unwrap().try_write().unwrap();
            let driver_name = driver.get_name();
            volume_size = driver.get_volume_size();
        }
        let flags: u16 = proto::NBD_FLAG_HAS_FLAGS | proto::NBD_FLAG_SEND_FLUSH | proto::NBD_FLAG_SEND_RESIZE | proto::NBD_FLAG_SEND_CACHE | proto::NBD_FLAG_SEND_TRIM;
        util::write_u16(proto::NBD_INFO_EXPORT, clone_stream!(self.socket));
        util::write_u64(volume_size, clone_stream!(self.socket));
        util::write_u16(flags, clone_stream!(self.socket));
        log::debug!("\t-->Export Data Sent:");
        log::debug!("\t-->\t-->NBD_INFO_EXPORT: \t{:?}", proto::NBD_INFO_EXPORT as u16);
        log::debug!("\t-->\t-->Volume Size: \t{:?}", volume_size as u32);
        log::debug!("\t-->\t-->Transmission Flags: \t{:?}", flags as u16);
    }

    fn reply_opt(&mut self, opt: u32, reply_type: u32, len: u32) {
        let data_permitted = vec![1, 6, 7, 9, 10].contains(&opt); // OPTION CODES THAT IS ALLOWED TO CARRY DATA
        self.reply(opt, reply_type, 4 + len);
        util::write_u32(len, clone_stream!(self.socket));
        log::debug!(" -> Option: {:?}, Option length: {:?}, Data permitted: {:?}", opt, len, data_permitted);

        if len > 0 {
            let mut data = vec![0; len as usize];
            clone_stream!(self.socket) // Docs says server should not reject the data even if not needed
                .read_exact(&mut data)
                .expect("Error on reading Option Data!");
            log::trace!("\t\\-> Data: {:?}", data);
            if data_permitted {
                write!(&data, self.socket);
                log::trace!("Data sent for option {:?}", opt);
            }
        }
    }

    fn handle_opt_abort(&mut self) {
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

    fn handle_opt_info_go(&mut self, opt: u32) {
        log::debug!("handle_opt_info_go");
        let _len = util::read_u32(clone_stream!(self.socket));
        let namelen = util::read_u32(clone_stream!(self.socket));
        // if namelen > len - 6 { return NBD_EINVAL }
        let name = match namelen {
            0 => "default".to_string(),
            _ => util::read_string(namelen as usize, clone_stream!(self.socket)),
        };
        let info_req_count = util::read_u16(clone_stream!(self.socket));
        let mut info_reqs = Vec::new();
        for _ in 0..info_req_count {
            info_reqs.push(util::read_u16(clone_stream!(self.socket)));
        }
        log::trace!("len:{},namelen:{},name:{},info_req_count:{}", _len, namelen, name, info_req_count);
        log::trace!("\t-->Info Requests: {:?}", info_reqs);

        self.select_export(name.clone().to_lowercase());
        if self.selected_export.is_none() {
            log::warn!("Unknown export: {}", &name.to_lowercase());
            let err_msg = String::from(format!("Unknown export: {}", &name.to_lowercase()));
            let err_as_bytes = err_msg.as_bytes();
            self.reply(
                opt,
                proto::NBD_REP_ERR_UNKNOWN,
                err_as_bytes.len() as u32
            );
            write!(err_as_bytes, self.socket);
            return
        }

        if info_reqs.is_empty() { //The client MAY list one or more items of specific information it is seeking in the list of information requests, or it MAY specify an empty list.
            info_reqs.push(3_u16);
        }

        for req in &info_reqs {
            match *req {
                proto::NBD_INFO_EXPORT => {// 0
                    self.reply_info_export(opt, name.clone().to_lowercase());
                }
                proto::NBD_INFO_NAME => {// 1
                    self.reply(opt, proto::NBD_REP_INFO, 0);
                    util::write_u16(proto::NBD_INFO_NAME, clone_stream!(self.socket));
                    write!(name.as_bytes(), self.socket);
                }
                proto::NBD_INFO_DESCRIPTION => {// 2
                    let name_as_bytes = name.as_bytes();
                    let length_of_name = name_as_bytes.len();
                    self.reply(
                        opt,
                        proto::NBD_REP_INFO,
                        2 + length_of_name as u32
                    );
                    util::write_u16(proto::NBD_INFO_DESCRIPTION, clone_stream!(self.socket));
                    write!(name_as_bytes, self.socket);
                }
                proto::NBD_INFO_BLOCK_SIZE => {// 3
                    self.reply(opt, proto::NBD_REP_INFO, 14);
                    util::write_u16(proto::NBD_INFO_BLOCK_SIZE, clone_stream!(self.socket));
                    util::write_u32(512 as u32, clone_stream!(self.socket));
                    util::write_u32(4*1024 as u32, clone_stream!(self.socket));
                    util::write_u32(32*1024*1024 as u32, clone_stream!(self.socket));
                    log::debug!("\t-->Sent block size info");
                    self.reply_info_export(opt, name.clone().to_lowercase());
                }
                r @ _ => {
                    log::warn!("Invalid Info Request: {:?}", r);
                    self.reply_opt(
                        proto::NBD_REP_INFO,
                        proto::NBD_EINVAL as u32,
                        0
                    )
                }
            }
        }

        self.reply(opt, proto::NBD_REP_ACK, 0);
        /*
        if (opt == proto::NBD_OPT_GO) & (self.selected_export.is_none()) {
            let session = self.session.take().unwrap();
            self.session = Some(NBDSession::new(
                clone_stream!(self.socket),
                session.flags,
                session.structured_reply,
                selected_export.1.driver_type.clone(),
                String::from(selected_export.0),
                session.metadata_context_id,
                selected_export.1.conn_str.clone()
            ));
        }
        */
    }


    fn handle_opt_set_meta_context(&mut self) {
        log::debug!("handle_opt_set_meta_context");
        let total_length = util::read_u32(clone_stream!(self.socket));
        let export_name_length = util::read_u32(clone_stream!(self.socket));
        let export_name = match export_name_length {
            0 => "default".to_string(),
            _ => util::read_string(export_name_length as usize, clone_stream!(self.socket)),
        };
        let number_of_queries = util::read_u32(clone_stream!(self.socket));
        log::trace!("\t-->total_length: {}, export_name_length: {}, export_name: {}, number_of_queries: {}", total_length, export_name_length, export_name, number_of_queries);
        if number_of_queries > 0 {
            for i in 0..number_of_queries {
                let query_length = util::read_u32(clone_stream!(self.socket));
                let query = util::read_string(query_length as usize, clone_stream!(self.socket));
                log::trace!("\t-->\t-->iter: {}, query: {}", i + 1, query);
                self.reply(
                    proto::NBD_OPT_SET_META_CONTEXT,
                    proto::NBD_REP_META_CONTEXT,
                    4 + query.len() as u32
                );
                let nbd_metadata_context_id = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos();
                self.metadata_context_id = nbd_metadata_context_id;
                util::write_u32(nbd_metadata_context_id, clone_stream!(self.socket));
                clone_stream!(self.socket).write(query.to_lowercase().as_bytes()).expect("Couldn't send query data");
            }
        }
        self.reply(proto::NBD_OPT_SET_META_CONTEXT, proto::NBD_REP_ACK, 0);
    }

    fn select_export(&mut self, export_name: String) {
        for export in &*self.export_refs.read().unwrap() {
            if *export.read().unwrap().name == export_name {
                self.selected_export = Some(Arc::clone(&export));
                break;
            }
        }
    }
}

//TODO
//impl Default for NBDSession {}
