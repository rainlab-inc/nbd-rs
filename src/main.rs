// use std::io;
// use std::io::prelude::*;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
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
            let stream_clone = stream.try_clone().unwrap();

            self.handle_connection(stream);
            self.handshake(stream_clone);
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

    fn read_client(&mut self, mut socket: TcpStream) -> [u8; 32] {
        let mut data = [0 as u8; 32];
        while match socket.read(&mut data) {
            Ok(size) => {
                socket.write(&data[0..size]).unwrap();
                true
            }
            Err(_) => {
                println!("Error on reading");
                false
            }
        } {}
        data
    }

    fn handshake(&mut self, socket: TcpStream) {
        println!("Handshake started...");
        let newstyle = proto::NBD_FLAG_FIXED_NEWSTYLE;
        let no_zeroes = proto::NBD_FLAG_NO_ZEROES;
        let handshake_flags: Vec<u8> = vec![newstyle | no_zeroes; 2];
        let buf: &[u8; 16] = b"NBDMAGICIHAVEOPT";
        let _initial_message: Vec<&u8> = buf.iter().chain(handshake_flags.iter()).collect();
        println!("Created Initial Message: {:?}", _initial_message);
        self.read_client(socket);
    }
}

fn main() {
    let mut server = NBDServer::new("0.0.0.0".to_string(), 10891);
    server.listen();
}
