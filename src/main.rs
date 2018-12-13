use std::io;
use std::io::prelude::*;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::net::TcpListener;
// use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
    sessions: Arc<Mutex<Vec<NBDSession>>>,
    // sessions: Vec<NBDSession>,
    token_counter: usize,
    port: u16
}


impl NBDServer {
    fn new(host: String, port: u16) -> NBDServer {
        let addr: SocketAddr = format!("{}:{}", host, port).parse().unwrap();
        let server = NBDServer {
            addr: addr,
            socket: TcpListener::bind(&addr).unwrap(),
            // sessions: vec![],
            sessions: Arc::new(Mutex::new(Vec::new())),
            token_counter: 0,
            port: port,
        };
        server
    }

    fn handle_connection(&self, socket: TcpStream /*, addr: SocketAddr*/) -> Result<(), io::Error> {
        // TODO: Process socket
        let session = NBDSession { socket: socket /*, addr: addr*/ };

        //self.sessions.push(session);
        match self.sessions.lock() {
            Ok(mut sessions) => sessions.push(session),
            Err(_) => panic!(),
        }

        println!("Connection established!");
        Ok(())
    }

    fn listen(self: NBDServer) {
        let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

        for stream in listener.incoming() {
            self.handle_connection(stream.unwrap());
        }

        println!("Done");
    }
}

fn main() {
    let server = NBDServer::new("0.0.0.0".to_string(), 10891);
    server.listen();
}
