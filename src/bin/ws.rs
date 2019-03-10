extern crate exitcode;
extern crate termcolor;
extern crate websocket;
extern crate futures;
extern crate tokio;

use std::env;
use std::io::stdin;
use std::thread;

use futures::future::Future;
use futures::sink::Sink;
use futures::stream::Stream;
use futures::sync::mpsc;

use websocket::result::WebSocketError;
use websocket::{ClientBuilder, OwnedMessage};

use smol::result::{SmolResult, SmolError};

fn help(args: Vec<String>) -> SmolResult<()> {
    println!(
        "usage: {} CONNECTION
    Interact with CONNECTION using websocket protocol.",
        args[0]
    );

    SmolError(exitcode::USAGE, None).into()
}

fn ws(connection: &str) -> SmolResult<()> {
    println!("Connecting to {}", connection);

    let mut runtime = tokio::runtime::current_thread::Builder::new()
        .build()?;

    let (usr_msg, stdin_ch) = mpsc::channel(0);
    thread::spawn(move || {
        let mut input = String::new();
        let mut stdin_sink = usr_msg.wait();
        loop {
            input.clear();
            stdin().read_line(&mut input).unwrap();
            let trimmed = input.trim();

            let (close, msg) = match trimmed {
                "/close" => (true, OwnedMessage::Close(None)),
                "/ping" => (false, OwnedMessage::Ping(b"PING".to_vec())),
                _ => (false, OwnedMessage::Text(trimmed.to_string())),
            };

            stdin_sink
                .send(msg).expect("Uh oh");

            if close {
                break;
            }
        }
    });

    let runner = ClientBuilder::new(connection)
        .map_err(|e| SmolError::from_err(exitcode::NOHOST, &e, "Could not connect"))?
        .async_connect(None)
        .and_then(|(duplex, _)| {
            println!("Connected");

            let (sink, stream) = duplex.split();
            stream
                .filter_map(|message| {
                    println!("Received Message: {:?}", message);
                    match message {
                        OwnedMessage::Close(e) => Some(OwnedMessage::Close(e)),
                        OwnedMessage::Ping(d) => Some(OwnedMessage::Pong(d)),
                        _ => None,
                    }
                })
                .select(stdin_ch.map_err(|_| WebSocketError::NoDataAvailable))
                .forward(sink)
        });

    runtime.block_on(runner)
        .map_err(|e| SmolError::from_err(exitcode::NOHOST, &e, "Runtime error"))?;

    Ok(())
}

fn run(args: Vec<String>) -> SmolResult<()> {
    let connection: Option<&str> = match args.len() {
        1 => Some("wss://echo.websocket.org"),
        2 => Some(&args[1]),
        _ => None,
    };

    match connection {
        Some(conn) => ws(conn).into(),
        None => help(args).into(),
    }
}

fn main() {
    match run(env::args().collect()) {
        Ok(_) => ::std::process::exit(exitcode::OK),
        Err(SmolError(code, Some(message))) => {
            println!("{}", message);
            ::std::process::exit(code);
        }
        Err(SmolError(code, _)) => ::std::process::exit(code),
    }
}
