// use std::io;
// use std::io::prelude::*;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::net::TcpStream;
// use std::collections::HashMap;

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
}

fn main() {
    let mut server = NBDServer::new("0.0.0.0".to_string(), 10891);
    server.listen();
}
