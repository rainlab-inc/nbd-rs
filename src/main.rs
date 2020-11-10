// use std::io;
// use std::io::prelude::*;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
//use byteorder::{BigEndian as BE};
//use std::collections::HashMap;

mod proto;

#[derive(Debug)]
struct NBDSession {
    socket: TcpStream,
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
            self.handshake();
        }

        println!("Done");
    }

    fn handle_connection(&mut self, socket: TcpStream /*, addr: SocketAddr*/) {
        // TODO: Process socket
        let session = NBDSession {
            socket,
            // addr,
        };
        self.session = Some(session);
        println!("Connection established!");
    }

    fn read_client(&mut self) -> [u8; 4] {
        let mut data = [0 as u8; 4];
        let mut socket = &self.session.as_ref().unwrap().socket;
        socket.read(&mut data).expect("Error on reading client.");
        data
    }

    fn handshake(&mut self) {
        println!("Handshake started...");
        let newstyle = proto::NBD_FLAG_FIXED_NEWSTYLE;
        let no_zeroes = proto::NBD_FLAG_NO_ZEROES;
        let handshake_flags = (newstyle | no_zeroes) as u16;//u8::from_ne_bytes((newstyle | no_zeroes).to_ne_bytes());
        let mut stream = &self.session.as_ref().unwrap().socket;
        stream.write(b"NBDMAGICIHAVEOPT").expect("Couldn't send initial message!");
        stream.write(&handshake_flags.to_be_bytes()).expect("Couldn't send handshake flags");
        println!("Initial message sent");
        let client_flags = u32::from_be_bytes(self.read_client());
        let c_newstyle = client_flags & (proto::NBD_FLAG_C_FIXED_NEWSTYLE as u32);
        let c_no_zeroes = client_flags & (proto::NBD_FLAG_C_NO_ZEROES as u32);
        println!(" -> fixedNewStyle: {}", c_newstyle != 0);
        println!(" -> noZeroes: {}", c_no_zeroes != 0);
    }
}

fn main() {
    let mut server = NBDServer::new("0.0.0.0".to_string(), 10891);
    server.listen();
}
