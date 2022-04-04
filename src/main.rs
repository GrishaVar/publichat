use std::{net::TcpListener, io::Read, path::Path, sync::Arc};

mod db;
mod msg;
mod constants;
mod http;
mod smrt;
use constants::*;


const IP_PORT: &str = "localhost:7878";


fn handle_incoming(mut stream: std::net::TcpStream, data_dir: Arc<Path>) {
    let mut pad_buf = [0; 4];
    stream.read(&mut pad_buf).expect("failed to read first 3 bytes!");

    match &pad_buf {
        b"GET " => http::handle(&mut stream, &data_dir),
        b"SMRT" => smrt::handle(&mut stream, &data_dir),
        _ => return,
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
        Arc::<Path>::from(path)
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
        for msg in msgs.iter() {db::push(&path, &msg).unwrap()}

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

    let listener = TcpListener::bind(IP_PORT).unwrap();

    for stream in listener.incoming() {
        let data_dir = data_dir.clone();
        if let Ok(stream) = stream {
            std::thread::spawn(move || handle_incoming(stream, data_dir));
        }
    }
}
