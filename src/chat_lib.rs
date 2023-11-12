#![allow(dead_code)]

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use mio::net::{TcpListener, TcpStream};
use std::str::FromStr;

/* ======================== Low level networking stuff ========================== */
/* Create a TCP socket listening to 'port' ready to accept connections. */
pub fn create_tcp_server(port: u16) -> TcpListener {
    let ip_addr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let socket_addr = SocketAddr::new(ip_addr, port);

    // SO_REUSEPORT is set by bind
    let listener = TcpListener::bind(socket_addr).expect("Failed to bind to address");
    println!("Smallchat server listening on tcp://{}", &socket_addr);
    // TcpListener in mio is non-blocking by default
    // listener.set_nonblocking(true).expect("Failed to set non-blocking mode");

    listener
}

/* Set the specified socket in non-blocking mode, with no delay flag. */
pub fn socket_set_nonblock_nodelay(stream: &TcpStream) {
    // TcpStream in mio is non-blocking by default
    // stream.set_nonblocking(true).expect("Cannot set non-blocking");
    stream.set_nodelay(true).expect("Cannot set non-delay");
}

/* If there is a new connection ready to be accepted, we accept it
 * and return new client socket on success. */
pub fn accept_client(tcp_listener: &TcpListener) -> Option<TcpStream> {
    let accept_result = tcp_listener.accept();
    match accept_result {
        Ok((stream, _addr)) => { Some(stream) }
        Err(err) if would_block(&err) => {
            None
        }
        Err(err) => {
            println!("Error while accept client: {err:?}");
            None
        }
    }
}

/* Create a TCP socket and connect it to the specified address. */
pub fn tcp_connect(host: &String, port: u16) -> Option<TcpStream> {
    let ip_addr = IpAddr::V4(Ipv4Addr::from_str(host.as_str()).unwrap());
    let server_addr = SocketAddr::new(ip_addr, port);

    let server_stream = TcpStream::connect(server_addr);
    match server_stream {
        Ok(server_stream) => {
            socket_set_nonblock_nodelay(&server_stream);

            Some(server_stream)
        }
        Err(_) => {
            None
        }
    }
}

/* ======================== Utility functions ========================== */
pub fn would_block(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::WouldBlock
}

pub fn interrupted(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::Interrupted
}