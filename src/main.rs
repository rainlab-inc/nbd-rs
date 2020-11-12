// use std::io;
// use std::io::prelude::*;
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
//use byteorder::{BigEndian as BE};
//use std::collections::HashMap;

mod proto;
mod util;

#[derive(Debug)]
struct NBDSession {
    socket: TcpStream,
    flags: [bool; 2],
    // addr: SocketAddr,
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
            self.handle_connection(stream);
        }

        println!("Done");
    }

    fn handle_connection(&mut self, socket: TcpStream /*, addr: SocketAddr*/) {
        // TODO: Process socket
        let socket_clone = socket.try_clone().expect("error");
        let flags = self.handshake(socket);
        let session = NBDSession {
            socket: socket_clone,
            flags,
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
        let socket_clone = socket.try_clone().expect("err on cloning.");

        socket
            .write(b"NBDMAGICIHAVEOPT")
            .expect("Couldn't send initial message!");
        util::write_u16(handshake_flags, socket);
        println!("Initial message sent");

        let client_flags = util::read_u32(socket_clone);
        let flags_list = [
            client_flags & (proto::NBD_FLAG_C_FIXED_NEWSTYLE as u32) != 0,
            client_flags & (proto::NBD_FLAG_C_NO_ZEROES as u32) != 0,
        ];

        println!(" -> fixedNewStyle: {}", flags_list[0]);
        println!(" -> noZeroes: {}", flags_list[1]);
        println!("Handshake done successfully!");
        flags_list
    }
}

fn main() {
    let mut server = NBDServer::new("0.0.0.0".to_string(), 10891);
    server.listen();
}
