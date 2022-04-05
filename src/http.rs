use std::{net::TcpStream, sync::Arc, path::Path, io::{Read, Write}, fs};
use sha1_smol::Sha1;
use crate::smrt;

fn handle_file(file: &str, stream: &mut TcpStream) {
    // BE CAREFUL WITH THIS ONE!
    println!("handling file");
    let file = match fs::read_to_string(file) {
        Ok(f) => f,
        _ => {handle_code(stream, 404); return;},
    };
    stream.write(format!(
        "HTTP/1.1 200\r\nContent-Length: {}\r\n\r\n{}",
        file.len(),
        file,
    ).as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn handle_ws(req: &String, stream: &mut TcpStream, data_dir: &Arc<Path>) {
    println!("handling ws");
    // handshake
    let key_in = match req.split("Sec-WebSocket-Key: ").nth(1) {
        Some(x) => &x[..24],
        _ => {handle_code(stream, 400); return},
    };
    let mut hasher = Sha1::new();
    hasher.update(key_in.as_bytes());
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let key_out = base64::encode(hasher.digest().bytes());

    stream.write(
        format!(
            "HTTP/1.1 101 Switching Protocols\r\n\
            Upgrade: websocket\r\n\
            Connection: Upgrade\r\n\
            Sec-WebSocket-Accept: {}\r\n\r\n",
            key_out,
        ).as_bytes()
    ).unwrap();
    stream.flush().unwrap();

    // launch SMRT
    // todo: does NOT handle ws en/decoding
    smrt::handle(stream, data_dir);
}

fn handle_code(stream: &mut TcpStream, code: u16) {
    println!("handling {}", code);
    stream.write(format!("HTTP/1.1 {}\r\n\r\n", code).as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn handle_robots(stream: &mut TcpStream) {
    println!("handling robots");
    stream.write(
        b"HTTP/1.1 200\r\nContent-Length: 25\r\n\r\n\
        User-agent: *\nDisallow: /"
    ).unwrap();
    stream.flush().unwrap();
}

pub fn handle(stream: &mut TcpStream, data_dir: &Arc<Path>) {
    // Handles GET requests (where first four bytes "GET " already consumed)
    let mut buf = [0; 512];
    stream.read(&mut buf).unwrap();
    let req = String::from_utf8_lossy(&buf).to_string();

    let path = match req.split(' ').nth(0) {
        Some(p) => p,
        None => return,  // faulty HTTP
    };
    println!("Recieved path: {}", path);

    match path {
        "/" | ""        => handle_file("page/index.html", stream),
        "/favicon.ico"  => handle_file("page/favicon.ico", stream),
        "/jspack.js"    => handle_file("page/jspack.js", stream),  // todo: remove
        "/ws/"          => handle_ws(&req, stream, data_dir),  // start WS
        "/robots.txt"   => handle_robots(stream),
        _               => handle_code(stream, 404),  // reject everything else
    };
}
