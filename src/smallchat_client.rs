mod chat_lib;

use std::{env, io};
use std::io::Read;
use std::process::exit;
use std::time::Duration;
use mio::{Events, Interest, Poll, Token};
use crate::chat_lib::{tcp_connect, would_block};

const CLIENT: Token = Token(0);

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: smallchat_client <host> <port>\n");
        exit(-1);
    }

    let host = args.get(1).unwrap();
    let port: u16 = args.get(2).unwrap().parse().unwrap();

    let mut server_stream = tcp_connect(host, port).unwrap();

    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);

    // Register the socket with `Poll`
    poll.registry().register(&mut server_stream, CLIENT, Interest::READABLE)?;

    loop {
        poll.poll(&mut events, Some(Duration::from_millis(100)))?;

        for event in events.iter() {
            match event.token() {
                CLIENT => {
                    if event.is_readable() {
                        let mut read_buf = [0; 256];
                        let nread = server_stream.read(&mut read_buf);
                        match nread {
                            Ok(0) => {
                                println!("Disconnected from server");
                                exit(-1);
                            }
                            Ok(size) => {
                                let msg = String::from_utf8(read_buf[..size].to_vec()).unwrap();
                                print!("{}", msg);
                            }
                            Err(ref e) if would_block(e) => {
                                continue;
                            }
                            Err(_) => {
                                println!("Disconnected from server");
                                exit(-1);
                            }
                        }
                    }
                }
                Token(_) => { // do nothing
                }
            }
        }
    }
}