mod chat_lib;

use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::{io, str};
use std::collections::HashMap;
use std::time::Duration;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use crate::chat_lib::{accept_client, create_tcp_server, socket_set_nonblock_nodelay,
                      interrupted, would_block};

/* ============================ Data structures ================================= */
const MAX_CLIENTS: usize = 1000;
const SERVER_PORT: u16 = 7711;

struct Client {
    stream: TcpStream,
    nick: String,
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
                    if event.is_readable() {
                        let msg = match clients.get_mut(&token) {
                            None => { None }
                            Some(client) => {
                                let mut read_buf = [0; 256];
                                let nread = client.stream.read(&mut read_buf);
                                match nread {
                                    Ok(size) if size > 0 => {
                                        let recv_msg = str::from_utf8(&read_buf[..size]).unwrap();
                                        let recv_msg = recv_msg.trim();
                                        if recv_msg.starts_with("/") {
                                            let parts: Vec<_> = recv_msg.splitn(2, ' ').collect();
                                            match parts[0] {
                                                "/nick" if parts.len() > 1 => client.nick = parts[1].to_string(),
                                                _ => {}
                                            }
                                            None
                                        } else {
                                            let msg = format!("{}> {}\n", client.nick, recv_msg);
                                            Some(msg)
                                        }
                                    }
                                    Err(ref e) if would_block(e) => {
                                        None
                                    }
                                    _ => {
                                        println!("Disconnected client(e) fd={}, nick={}", client.stream.as_raw_fd(), client.nick);
                                        if let Some(mut client) = clients.remove(&token) {
                                            poll.registry().deregister(&mut client.stream)?;
                                        }
                                        None
                                    }
                                }
                            }
                        };

                        match msg {
                            Some(msg) => {
                                if let Some(client) = clients.get(&token) {
                                    send_msg_to_all_clients_but(&clients, &client.stream, msg.as_bytes());
                                }
                            }
                            None => {}
                        }
                    }
                }
            }
        }
    }
}