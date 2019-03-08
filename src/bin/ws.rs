extern crate exitcode;
extern crate termcolor;
extern crate websocket;

use std::env;
use std::io::{stdin, Write};
use std::sync::mpsc::channel;
use std::thread;

use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use websocket::client::ClientBuilder;
use websocket::{Message, OwnedMessage};

use smol::errors::SmolError;

fn help(args: Vec<String>) -> SmolError {
    println!(
        "usage: {} CONNECTION
    Interact with CONNECTION using websocket protocol.",
        args[0]
    );

    SmolError(exitcode::USAGE, None)
}

fn ws(connection: &str) -> Result<(), SmolError> {
    println!("Connecting to {}", connection);

    let client = ClientBuilder::new(connection)
        .map_err(|e| SmolError::from_err(exitcode::NOHOST, &e, "Could not connect"))?
        .add_protocol("rust-websocket")
        .connect_insecure()
        .map_err(|e| SmolError::from_err(exitcode::NOHOST, &e, "Could not connect"))?;

    println!("Successfully connected to {}", connection);

    let (mut receiver, mut sender) = client.split()?;

    let (tx, rx) = channel();

    let tx_1 = tx.clone();

    let send_loop = thread::spawn(move || {
        loop {
            let message = match rx.recv() {
                Ok(m) => m,
                Err(e) => {
                    println!("Send Loop: {:?}", e);
                    return;
                }
            };
            match message {
                OwnedMessage::Close(_) => {
                    let _ = sender.send_message(&message);
                    return;
                }
                _ => (),
            }
            match sender.send_message(&message) {
                Ok(()) => (),
                Err(e) => {
                    println!("Send Loop: {:?}", e);
                    let _ = sender.send_message(&Message::close());
                    return;
                }
            }
        }
    });

    let receive_loop = thread::spawn(move || {
        let mut stdout = StandardStream::stdout(ColorChoice::Always);

        for message in receiver.incoming_messages() {
            let message = match message {
                Ok(m) => m,
                Err(e) => {
                    println!("Receive Loop: {:?}", e);
                    let _ = tx_1.send(OwnedMessage::Close(None));
                    return;
                }
            };
            match message {
                OwnedMessage::Close(_) => {
                    let _ = tx_1.send(OwnedMessage::Close(None));
                    return;
                }
                OwnedMessage::Ping(data) => match tx_1.send(OwnedMessage::Pong(data)) {
                    Ok(()) => (),
                    Err(e) => {
                        println!("Receive Loop: {:?}", e);
                        return;
                    }
                },
                _ => {
                    match stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow))) {
                        Err(_) => return,
                        _ => (),
                    };

                    match writeln!(&mut stdout, "Receive Loop: {:?}", message) {
                        Ok(_) => (),
                        Err(_) => return,
                    }
                }
            }
        }
    });

    loop {
        let mut input = String::new();

        stdin().read_line(&mut input)?;

        let trimmed = input.trim();

        let message = match trimmed {
            "/close" => {
                let _ = tx.send(OwnedMessage::Close(None));
                break;
            }
            "/ping" => OwnedMessage::Ping(b"PING".to_vec()),
            _ => OwnedMessage::Text(trimmed.to_string()),
        };

        match tx.send(message) {
            Ok(()) => (),
            Err(e) => {
                println!("Main Loop: {:?}", e);
                break;
            }
        }
    }

    println!("Exiting");
    let _ = send_loop.join();
    let _ = receive_loop.join();
    println!("Exited");

    Ok(())
}

fn run(args: Vec<String>) -> Result<(), SmolError> {
    let connection: Option<&str> = match args.len() {
        1 => Some("ws://echo.websocket.org"),
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
