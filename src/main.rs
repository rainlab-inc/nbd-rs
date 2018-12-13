// use std::io;
// use std::io::prelude::*;
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
    host: String,
    port: u16,
}


impl NBDServer {
    fn new(host: String, port: u16) -> NBDServer {
        let addr: SocketAddr = format!("{}:{}", host, port).parse().unwrap();

        let server = NBDServer {
            host,
            addr,
            socket: TcpListener::bind(&addr).unwrap(),
            sessions: Arc::new(Mutex::new(Vec::new())),
            port,
        };
        server
    }

    fn handle_connection(&self, socket: TcpStream /*, addr: SocketAddr*/) {
        // TODO: Process socket
        let mut session = NBDSession {
            socket,
            // addr,
        };

        //self.sessions.push(session);
        match self.sessions.lock() {
            Ok(mut sessions) => {
                sessions.push(session);
            },
            Err(_) => panic!(),
        }

        println!("Connection established!");
    }

    fn listen(self: NBDServer) {
        let hostport = format!("{}:{}", self.host, self.port);
        println!("Listening on {}", hostport);
        // let listener = self.socket;

        for stream in self.socket.incoming() {
            self.handle_connection(stream.unwrap());
        }

        println!("Done");
    }
}

fn main() {
    let server = NBDServer::new("0.0.0.0".to_string(), 10891);
    server.listen();
}
