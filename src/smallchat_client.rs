mod chat_lib;

use std::{env, io, str};
use std::io::{Read, Write, stdout, Stdout};
use std::process::exit;
use std::time::Duration;
use mio::{Events, Interest, Poll, Token};
use termion::{async_stdin, clear};
use termion::event::Key;
use termion::event::Key::Ctrl;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use crate::chat_lib::{tcp_connect, would_block};

const CLIENT: Token = Token(0);

/* ============================================================================
 * Minimal line editing.
 * ========================================================================== */
fn terminal_clean_current_line(stdout: &mut Stdout) {
    write!(stdout, "{}", clear::CurrentLine).unwrap();
    stdout.flush().unwrap();
}

fn terminal_cursor_at_line_start(stdout: &mut Stdout) {
    write!(stdout, "\r").unwrap();
    stdout.flush().unwrap();
}

/* Hide the line the user is typing. */
fn input_buffer_hide(stdout: &mut Stdout) {
    terminal_clean_current_line(stdout);
    terminal_cursor_at_line_start(stdout);
}

fn input_buffer_show(stdout: &mut Stdout, input_buffer: &Vec<char>) {
    for c in input_buffer {
        write!(stdout, "{}", c).unwrap();
    }
    stdout.flush().unwrap();
}

fn input_buffer_clear(stdout: &mut Stdout, input_buffer: &mut Vec<char>) {
    input_buffer.clear();
    input_buffer_hide(stdout);
}

/* Main program. */
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

    let mut input_buffer = Vec::with_capacity(128);
    // Get the standard input stream.
    let mut stdin = async_stdin().keys();
    // let mut stdin = stdin();
    // Get the standard output stream and go to raw mode.
    let mut stdout = stdout().into_raw_mode().unwrap();

    loop {
        loop {
            // check the user input
            let key = stdin.next();
            match key {
                Some(Ok(key)) => {
                    match key {
                        Key::Backspace => {
                            // remove a char from the input buffer
                            input_buffer.pop();

                            // clear previous input and flush out current input
                            input_buffer_hide(&mut stdout);
                            input_buffer_show(&mut stdout, &input_buffer);
                        }
                        Key::Char(c) => {
                            input_buffer.push(c);

                            if c == '\n' || c == '\r' {
                                input_buffer_hide(&mut stdout);
                                write!(stdout, "{}", "you> ").unwrap();
                                input_buffer_show(&mut stdout, &input_buffer);

                                // send the data to the server
                                let input: Vec<u8> = input_buffer.iter().flat_map(
                                    |&c| c.to_string().into_bytes()).collect();
                                server_stream.write(input.as_slice()).unwrap();

                                // and clear the input buffer
                                input_buffer_clear(&mut stdout, &mut input_buffer);
                            } else {
                                // write every single char to the console
                                write!(stdout, "{}", c).unwrap();
                                stdout.flush().unwrap();
                            }
                        }
                        Ctrl(c) => {
                            if c.eq_ignore_ascii_case(&'c') {
                                // Ctrl+C, we need restore the console
                                stdout.suspend_raw_mode().unwrap();
                                exit(1);
                            }
                        }
                        _ => {}
                    }
                }
                _ => { break; }
            }
        }

        // check whether there is data from server
        poll.poll(&mut events, Some(Duration::from_millis(100)))?;
        for event in events.iter() {
            match event.token() {
                CLIENT => {
                    if event.is_readable() {
                        let mut read_buf = [0; 256];
                        let nread = server_stream.read(&mut read_buf);
                        match nread {
                            Ok(size) if size > 0 => {
                                let msg = str::from_utf8(&read_buf[..size]).unwrap();
                                input_buffer_hide(&mut stdout);
                                // the received only have a '\n', we need add '\r'
                                write!(stdout, "{}\r", msg).unwrap();
                                input_buffer_show(&mut stdout, &input_buffer);
                            }
                            Err(ref e) if would_block(e) => {
                                continue;
                            }
                            _ => {
                                println!("Disconnected from server");
                                exit(-1);
                            }
                        }
                    }
                }
                _ => { /* do nothing */ }
            }
        }
    }
}