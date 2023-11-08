use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream, SocketAddr, IpAddr};
use std::os::fd::AsRawFd;
use std::{io, str};

/* ============================ Data structures ================================= */
const MAX_CLIENTS: usize = 1000;
const SERVER_PORT: u16 = 7711;

struct Client {
    stream: TcpStream,
    nick: String,
}

/* ======================== Low level networking stuff ========================== */
/* Create a TCP socket listening to 'port' ready to accept connections. */
fn create_tcp_server(port: u16) -> TcpListener {
    let ip_addr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let socket_addr = SocketAddr::new(ip_addr, port);

    // SO_REUSEPORT isn't cross-platform, can't set directly
    let listener = TcpListener::bind(socket_addr).expect("Failed to bind to address");
    println!("Smallchat server listening on tcp://{}", &socket_addr);
    listener.set_nonblocking(true).expect("Failed to set non-blocking mode");

    listener
}

/* Set the specified socket in non-blocking mode, with no delay flag. */
fn socket_set_nonblock_nodelay(stream: &TcpStream) {
    stream.set_nonblocking(true).expect("Cannot set non-blocking");
    stream.set_nodelay(true).expect("Cannot set non-delay");
}

/* If there is a new connection ready to be accepted, we accept it
 * and return new client socket on success. */
fn accept_client(tcp_listener: &TcpListener) -> Option<TcpStream> {
    let accept_result = tcp_listener.accept();
    match accept_result {
        Ok((stream, _addr)) => { Some(stream) }
        Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
            None
        }
        Err(err) => {
            println!("Error while accept client: {err:?}");
            None
        }
    }
}

/* ====================== Small chat core implementation ======================== */
fn create_client(stream: TcpStream, clients: &mut Vec<Option<Client>>) {
    let stream_fd = stream.as_raw_fd();
    socket_set_nonblock_nodelay(&stream);
    let nick = format!("user:{}", stream_fd);

    let client = Client { stream, nick };
    clients.push(Some(client));
}

/* Allocate and init the global stuff. */
fn init_chat() -> (TcpListener, Vec<Option<Client>>) {
    let listener = create_tcp_server(SERVER_PORT);
    (listener, Vec::with_capacity(MAX_CLIENTS))
}

/* Send the specified string to all connected clients but the one
 * having as socket descriptor 'excluded'. */
fn send_msg_to_all_clients_but(clients_stream: &mut Vec<TcpStream>, excluded: &TcpStream, msg: &[u8]) {
    for stream in clients_stream.iter_mut() {
        // TODO Doesn't work now, because the clients_stream are cloned from the initial stream, and the raw fd is different
        if stream.as_raw_fd() != excluded.as_raw_fd() {
            stream.write(msg).expect("Failed write msg to client");
        }
    }
}


fn main() {
    let (tcp_listener, mut clients) = init_chat();
    loop {
        let stream_opt = accept_client(&tcp_listener);
        match stream_opt {
            None => {}
            Some(mut stream) => {
                let welcome_msg = "Welcome to Simple Chat! Use /nick <nick> to set your nick.\n";
                stream.write(welcome_msg.as_bytes()).expect("Failed to send response to client");

                create_client(stream, &mut clients);
            }
        }

        let mut clients_stream_clone: Vec<TcpStream> = clients.iter_mut().filter_map(
            |client_opt|
                match client_opt {
                    None => { None }
                    Some(client) => {
                        Some(client.stream.try_clone().expect("Can't clone the stream"))
                    }
                }
        ).collect();

        let mut read_buf = [0; 256];
        for client_opt in clients.iter_mut() {
            match client_opt {
                None => {}
                Some(client) => {
                    let mut client_stream = &client.stream;
                    let nread = client_stream.read(&mut read_buf);
                    match nread {
                        Ok(0) => {
                            println!("Disconnected client(0) fd={}, nick={}", client_stream.as_raw_fd(), client.nick);
                            *client_opt = None;
                            continue;
                        }
                        Ok(mut size) => {
                            if read_buf[0] == '/' as u8 {
                                if let Some(index) = read_buf.iter().position(|&num| num == '\n' as u8) {
                                    read_buf[index] = 0;
                                    size -= 1;
                                }
                                if let Some(index) = read_buf.iter().position(|&num| num == '\r' as u8) {
                                    read_buf[index] = 0;
                                    size -= 1;
                                }

                                if read_buf.starts_with(b"/nick") {
                                    if let Some(arg) = read_buf.iter().position(|&num| num == ' ' as u8) {
                                        let nick = &read_buf[arg + 1..size];
                                        client.nick = str::from_utf8(nick).unwrap().to_string();
                                    }
                                }
                            } else {
                                let msg = &read_buf[..size];
                                println!("{} {:?}", client.nick, msg);

                                send_msg_to_all_clients_but(&mut clients_stream_clone, &client.stream, msg);
                            }
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            continue;
                        }
                        Err(_) => {
                            println!("Disconnected client(e) fd={}, nick={}", client.stream.as_raw_fd(), client.nick);
                            *client_opt = None;
                            continue;
                        }
                    }
                }
            }
        }
    }
}