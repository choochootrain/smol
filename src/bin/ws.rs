extern crate exitcode;
extern crate websocket;

use std::env;
use std::io::stdin;
use std::sync::mpsc::channel;
use std::thread;

use websocket::client::ClientBuilder;
use websocket::{Message, OwnedMessage};

use smol::errors::SmolError;

fn help(args: Vec<String>) -> SmolError {
    println!(
        "usage: {} CONNECTION
    Interact with CONNECTION using websocket protocol,",
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
            // Send loop
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
                    // If it's a close message, just send it and then return.
                    return;
                }
                _ => (),
            }
            // Send the message
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
        // Receive loop
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
                    // Got a close message, so send a close message and return
                    let _ = tx_1.send(OwnedMessage::Close(None));
                    return;
                }
                OwnedMessage::Ping(data) => {
                    match tx_1.send(OwnedMessage::Pong(data)) {
                        // Send a pong in response
                        Ok(()) => (),
                        Err(e) => {
                            println!("Receive Loop: {:?}", e);
                            return;
                        }
                    }
                }
                // Say what we received
                _ => println!("Receive Loop: {:?}", message),
            }
        }
    });

    loop {
        let mut input = String::new();

        stdin().read_line(&mut input)?;

        let trimmed = input.trim();

        let message = match trimmed {
            "/close" => {
                // Close the connection
                let _ = tx.send(OwnedMessage::Close(None));
                break;
            }
            // Send a ping
            "/ping" => OwnedMessage::Ping(b"PING".to_vec()),
            // Otherwise, just send text
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

    // We're exiting

    println!("Waiting for child threads to exit");

    let _ = send_loop.join();
    let _ = receive_loop.join();

    println!("Exited");
    Ok(())
}

fn run(args: Vec<String>) -> Result<(), SmolError> {
    let connection: Option<&str> = match args.len() {
        1 => Some("ws://echo.websocket.org"),
        2 => Some(&args[1]),
        _ => None
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
