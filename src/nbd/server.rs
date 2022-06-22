#![allow(unused_variables)]

use std::{
    io::{Write, BufWriter},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{Arc, RwLock},
    collections::{HashMap},
    rc::Rc,
    cell::RefCell,
};

use crate::{
    block::{BlockStorage, block_storage_with_config},
    nbd::{proto, NBDSession},
    util,
};

use log;

pub struct NBDServer {
    addr: SocketAddr,
    socket: TcpListener,
    //sessions: Vec<NBDSession>,
    host: String,
    port: u16,
    exports: Arc<RwLock<Vec<Arc<RwLock<NBDExport>>>>>
}

pub struct NBDExport {
    pub name: String,
    driver_type: String,
    config: String,
    pub driver: Arc<RwLock<Box<dyn BlockStorage>>>,
    pub in_use: bool
}

impl NBDExport {
    pub fn new(name: String, driver_type: String, conn_str: String) -> NBDExport {
        // TODO: unhardcode below from here (it is okay to hardcode in block/mod.rs though)
        if !["raw", "sharded", "distributed"].contains(&driver_type.as_str()) {
            panic!("Driver must be one of the values `raw` or `sharded`. Found '{}'", driver_type);
        }
        let driver = block_storage_with_config(
            name.clone(),
            driver_type.clone(),
            conn_str.clone()
        ).unwrap();
        log::info!("export {:?} -> {}({:?})", &name, &driver_type, &conn_str);
        NBDExport {
            name: name,
            driver_type: driver_type,
            config: conn_str,
            driver: Arc::new(RwLock::new(driver)),
            in_use: false
        }
    }
}


impl NBDServer {
    pub fn new(host: String, port: u16, exports: Vec<Arc<RwLock<NBDExport>>>) -> NBDServer {
        let addr: SocketAddr = format!("{}:{}", host, port).parse().unwrap();
        let socket_addr = addr.clone();

        NBDServer {
            addr,
            socket: TcpListener::bind(socket_addr).unwrap(),
            //sessions: Vec::new(),
            host,
            port,
            exports: Arc::new(RwLock::new(exports))
        }
    }

    pub fn listen(&mut self) {
        let hostport = format!("{}:{}", self.host, self.port);
        log::info!("Listening on {}", hostport);
        // let listener = self.socket;

        loop {
            // This part can be simplified with returning a result type and using `?` at the
            // end of .accept()?
            let (stream, _) = match self.socket.accept() {
                Ok(conn) => conn,
                Err(e) => {
                    log::error!("failed to accept: {:?}", e);
                    break;
                }
            };
            let session = self.handle_connection(Rc::new(RefCell::new(stream)));
            session.handle();
        }

        log::info!("Done");
    }

    fn handle_connection(&mut self, socket: Rc<RefCell<TcpStream>> /*, addr: SocketAddr*/) -> NBDSession {
        // TODO: Process socket
        let flags = self.handshake(Rc::clone(&socket));
        // TODO: implement Default for NBDSession
        let session = NBDSession::new(
            Rc::clone(&socket),
            flags,
            false,
            String::from(""),
            String::from(""),
            0,
            String::from(""),
            Arc::clone(&self.exports)
        );
        //self.sessions.push(session);
        log::info!("Connection established!");
        session
    }

    fn handshake(&mut self, socket: Rc<RefCell<TcpStream>>) -> [bool; 2] {
        log::debug!("Handshake started...");
        let newstyle = proto::NBD_FLAG_FIXED_NEWSTYLE;
        let no_zeroes = proto::NBD_FLAG_NO_ZEROES;
        let handshake_flags = (newstyle | no_zeroes) as u16;

        {
            let socket = Rc::clone(&socket);
            let m_socket = &*socket.borrow();
            let mut buf = BufWriter::new(m_socket);
            buf.write(b"NBDMAGIC").unwrap();
            buf.write(b"IHAVEOPT").unwrap();
            buf.write(&handshake_flags.to_be_bytes()).unwrap();
            buf.flush().unwrap();
        }
        log::trace!("Initial message sent");

        let socket = Rc::clone(&socket);
        let client_flags = util::read_u32(&socket.borrow());
        drop(socket);
        let flags_list = [
            client_flags & (proto::NBD_FLAG_C_FIXED_NEWSTYLE as u32) != 0,
            client_flags & (proto::NBD_FLAG_C_NO_ZEROES as u32) != 0,
        ];

        log::debug!(" -> fixedNewStyle: {}", flags_list[0]);
        log::debug!(" -> noZeroes: {}", flags_list[1]);
        log::debug!("Handshake done successfully!");
        flags_list
    }
}
