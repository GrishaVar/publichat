use std::{net::{TcpListener, TcpStream}, path::Path, sync::Arc, thread::{self, Builder}};

mod db;
mod constants;
mod http;
mod smrt;
mod ws;
mod helpers;

use helpers::*;

const IP_PORT: &str = "localhost:7878";


fn handle_incoming(mut stream: TcpStream, globals: &Arc<Globals>) -> Res {
    let mut pad_buf = [0; 4];

    let mut http_handled: u8 = 0;
    while {  // Handle repeated HTTP requests
        stream.peek(&mut pad_buf)
            .map_err(|_| "Failed to read protocol header (HTTP timeout?)")?;
        &pad_buf == b"GET "
    } {
        http::handle(
            stream.try_clone().map_err(|_| "Failed to clone")?,
            globals
        )?;

        http_handled += 1;  // TODO: better system for dropping connections
        if http_handled >= 3 {
            stream.shutdown(std::net::Shutdown::Both)
                .map_err(|_| "HTTP shutdown failed")?;
            return Ok(());
        }
        if http_handled == 1 {
            stream.set_read_timeout(Some(std::time::Duration::from_secs(1)))
                .map_err(|_| "Failed to set short timeout")?;
        }
    }

    // HTTP finished. Read either SMRT or fail.
    if &pad_buf == b"SMRT" {
        read_exact(&mut stream, &mut pad_buf, "Failed to remove SMRT buffer")?;
        smrt::handle(stream, globals)
    } else {
        Err("Failed to match protocol header")
    }
}

fn main() {
    let globals = {

        // Get chat directory path
        let data_dir = {
            let args: Vec<String> = std::env::args().skip(1).collect();
            if let Some(path) = args.last() {
                let path = Path::new(path);
                if !path.is_dir() {
                    println!("Not a directory: {:?}", path);
                    std::process::exit(1);
                }
                path.to_path_buf()  // put on heap
            } else {
                println!("No path given");
                std::process::exit(1);
            }
        };
        println!("Using directory {:?}", data_dir.canonicalize().unwrap());

        // Get git hash
        let git_hash = {
            let output = std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap_or_else(|_| {
                    println!("Failed to exec git command");
                    std::process::exit(1);
                });

            let git_output = output.stdout;  // has a \n at the end!
            if git_output.len() != 41
                || *git_output.last().unwrap() != b'\n'
                || std::str::from_utf8(&git_output).is_err() {
                    println!("Received strange data from git");
                    std::process::exit(1);
                }

            let mut git_hash = [0; 40];
            git_hash.copy_from_slice(&git_output[..40]);
            git_hash
        };
        println!("Using git hash {}", std::str::from_utf8(&git_hash).unwrap());

        Arc::new(Globals {
            data_dir,
            git_hash,
        })
    };

    let listener = TcpListener::bind(IP_PORT).unwrap_or_else(|_| {
        println!("Failed to bind TCP port. Exiting...");
        std::process::exit(1);
    });
    println!("Using IP & port {}", IP_PORT);

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let globals = globals.clone();

            let name = match stream.peer_addr() {
                Ok(addr) => addr.to_string(),
                Err(_) => "unknown".to_string(),
            };

            let builder = Builder::new().name(name);  // todo: stack size?
            let handle = builder.spawn(move || {
                println!("Handling {}", thread::current().name().unwrap());
                if let Err(e) = handle_incoming(stream, &globals) {
                    println!(
                        "Finished {} with:\n\t{e}",
                        thread::current().name().unwrap(),
                    );
                } else {
                    println!(
                        "Finished {} (no message)",
                        thread::current().name().unwrap(),
                    );
                }
            });

            if let Err(e) = handle {
                println!("Failed to create thread: {e}");
            }
        } else {
            println!("failed to bind stream: {}", stream.err().unwrap());
        }
    }
}
