use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, IpAddr};
use std::os::fd::AsRawFd;
use std::{io, str};
use std::collections::HashMap;
use std::time::Duration;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};

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

    // SO_REUSEPORT is set by bind
    let listener = TcpListener::bind(socket_addr).expect("Failed to bind to address");
    println!("Smallchat server listening on tcp://{}", &socket_addr);
    // TcpListener in mio is non-blocking as default
    //listener.set_nonblocking(true).expect("Failed to set non-blocking mode");

    listener
}

/* Set the specified socket in non-blocking mode, with no delay flag. */
fn socket_set_nonblock_nodelay(stream: &TcpStream) {
    // stream.set_nonblocking(true).expect("Cannot set non-blocking");
    stream.set_nodelay(true).expect("Cannot set non-delay");
}

/* If there is a new connection ready to be accepted, we accept it
 * and return new client socket on success. */
fn accept_client(tcp_listener: &TcpListener) -> Option<TcpStream> {
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

/* ====================== Small chat core implementation ======================== */
fn create_client(stream: TcpStream) -> Client {
    let stream_fd = stream.as_raw_fd();
    socket_set_nonblock_nodelay(&stream);
    let nick = format!("user:{}", stream_fd);

    Client { stream, nick }
}

/* Allocate and init the global stuff. */
fn init_chat() -> (TcpListener, HashMap<Token, Client>) {
    let listener = create_tcp_server(SERVER_PORT);
    (listener, HashMap::with_capacity(MAX_CLIENTS))
}

/* Send the specified string to all connected clients but the one
 * having as socket descriptor 'excluded'. */
fn send_msg_to_all_clients_but(clients_stream: &HashMap<Token, Client>, excluded: &TcpStream, msg: &[u8]) {
    for (_token, client) in clients_stream.iter() {
        let mut stream = &client.stream;
        if stream.as_raw_fd() != excluded.as_raw_fd() {
            stream.write(msg).expect("Failed write msg to client");
        }
    }
}

fn would_block(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::WouldBlock
}

fn interrupted(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::Interrupted
}

fn next(current: &mut Token) -> Token {
    let next = current.0;
    current.0 += 1;
    Token(next)
}

// Setup some tokens to allow us to identify which event is for which socket.
const SERVER: Token = Token(0);

fn main() -> io::Result<()> {
    let (mut tcp_listener, mut clients) = init_chat();

    // Create a poll instance.
    let mut poll = Poll::new()?;
    // Create storage for events.
    let mut events = Events::with_capacity(128);

    // Register the server with poll we can receive events for it.
    poll.registry()
        .register(&mut tcp_listener, SERVER, Interest::READABLE)?;

    // Unique token for each incoming connection.
    let mut unique_token = Token(SERVER.0 + 1);

    loop {
        if let Err(err) = poll.poll(&mut events, Some(Duration::from_millis(10))) {
            if interrupted(&err) {
                continue;
            }
            return Err(err);
        }
        for event in events.iter() {
            match event.token() {
                SERVER => loop {
                    // Received an event for the TCP server socket, which
                    // indicates we can accept an connection.
                    let stream_opt = accept_client(&tcp_listener);
                    match stream_opt {
                        None => { break; }
                        Some(mut stream) => {
                            let welcome_msg = "Welcome to Simple Chat! Use /nick <nick> to set your nick.\n";
                            stream.write(welcome_msg.as_bytes()).expect("Failed to send response to client");

                            let mut new_client = create_client(stream);

                            let token = next(&mut unique_token);
                            poll.registry().register(
                                &mut new_client.stream,
                                token,
                                Interest::READABLE.add(Interest::WRITABLE),
                            )?;

                            clients.insert(token, new_client);
                        }
                    }
                },
                token => {
                    let mut read_buf = [0; 256];
                    // Maybe received an event for a client
                    let msg = if let Some(client) = clients.get_mut(&token) {
                        if event.is_readable() {
                            let mut client_stream = &client.stream;
                            let nread = client_stream.read(&mut read_buf);
                            match nread {
                                Ok(0) => {
                                    println!("Disconnected client(0) fd={}, nick={}", client_stream.as_raw_fd(), client.nick);
                                    if let Some(mut client) = clients.remove(&token) {
                                        poll.registry().deregister(&mut client.stream)?;
                                    }
                                    None
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
                                        None
                                    } else {
                                        let mut msg_vec = format!("{}> ", client.nick).as_bytes().to_vec();
                                        msg_vec.extend_from_slice(&read_buf[..size]);

                                        Some(msg_vec)
                                    }
                                }
                                Err(ref e) if would_block(e) => {
                                    None
                                }
                                Err(_) => {
                                    println!("Disconnected client(e) fd={}, nick={}", client.stream.as_raw_fd(), client.nick);
                                    if let Some(mut client) = clients.remove(&token) {
                                        poll.registry().deregister(&mut client.stream)?;
                                    }
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    } else {
                        // Sporadic events happen, we can safely ignore them.
                        None
                    };
                    match msg {
                        None => {}
                        Some(msg) => {
                            if let Some(client) = clients.get(&token) {
                                send_msg_to_all_clients_but(&clients, &client.stream, msg.as_slice());
                            }
                        }
                    }
                }
            }
        }
    }
}