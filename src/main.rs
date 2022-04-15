use std::{net::{TcpListener, TcpStream}, path::Path, sync::Arc, thread::{self, Builder}, fs};

mod db;
mod msg;
mod constants;
mod http;
mod smrt;
mod ws;
mod helpers;

use constants::*;
use helpers::*;

const IP_PORT: &str = "localhost:7878";


fn handle_incoming(mut stream: TcpStream, globals: &Arc<Globals>) -> Res {
    let mut pad_buf = [0; 4];
    read_exact(&mut stream, &mut pad_buf, "Failed to read protocol header")?;

    match &pad_buf {
        b"GET " => http::handle(stream, globals),
        b"SMRT" => smrt::handle(stream, globals),
        _ => Err("Failed to match protocol header"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let data_dir = if let Some(path) = args.last() {
        let path = Path::new(path);
        if !path.is_dir() {
            println!("Not a directory: {:?}", path);
            std::process::exit(1);
        }
        path.to_path_buf()  // put on heap
    } else {
        println!("No path given");
        std::process::exit(2);
    };
    println!("Using directory {:?}", data_dir.canonicalize().unwrap());

    {  // testing db
        const COUNT: usize = 25;  // no more than 94

        let msgs: Vec<MessageSt> = (0..COUNT as u8).map(|i| {[i+b'!'; MSG_ST_SIZE]}).collect();
        let path = data_dir.join("test.msgs");

        let t1 = std::time::SystemTime::now();
        for msg in msgs.iter() {db::push(&path, msg).unwrap()}

        let t2 = std::time::SystemTime::now();
        for i in 0..COUNT {
            db::query(&path, i as u32, 20, true).unwrap();
        }

        let t3 = std::time::SystemTime::now();
        for i in 0..COUNT {
            db::query(&path, i as u32, 20, false).unwrap();
        }

        let t4 = std::time::SystemTime::now();
        println!("Pushing {} messages: {}μs",       COUNT, t2.duration_since(t1).unwrap().as_micros());
        println!("Fetching forward {} times: {}μs", COUNT, t3.duration_since(t2).unwrap().as_micros());
        println!("Fetching bckward {} times: {}μs", COUNT, t4.duration_since(t3).unwrap().as_micros());
    }

    let listener = TcpListener::bind(IP_PORT).unwrap_or_else(|_| {
        println!("Failed to bind TCP port. Exiting...");
        std::process::exit(3);
    });

    let globals = {
        fn file_getter(path: &str) -> Vec<u8> {
            fs::read(path).unwrap_or_else(|_| {
                println!("Failed to open file: {}", path);
                std::process::exit(4);
            })
        }

        Arc::new(Globals {
            data_dir,
            index_html:  file_getter("page/index.html"),
            mobile_html: file_getter("page/mobile.html"),
            client_js:   file_getter("page/client.js"),
            jspack_js:   file_getter("page/jspack.js"),
            favicon_ico: file_getter("page/favicon.ico"),
        })
    };

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let globals = globals.clone();

            let name = match stream.peer_addr() {
                Ok(addr) => addr.to_string(),
                Err(_) => "unknown".to_string(),
            };

            let builder = Builder::new().name(name);  // todo: stack size?
            let handle = builder.spawn(move || {
                println!("Started  {}", thread::current().name().unwrap());
                if let Err(e) = handle_incoming(stream, &globals) {
                    println!(
                        "Thread {} finished with error:\n\t{e}",
                        thread::current().name().unwrap(),
                    );
                } else {
                    println!(
                        "Finished {} (no errors)",
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
