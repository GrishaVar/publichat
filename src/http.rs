use std::{net::TcpStream, sync::Arc, path::Path, fs, io::{Read, Write}};
use sha1_smol::Sha1;
use crate::smrt;
use crate::ws::WsStream;

fn handle_file(file: &str, stream: &mut TcpStream) {
    // BE CAREFUL WITH THIS ONE!
    println!("handling file: {}", file);
    let body = match fs::read(file) {
        Ok(f) => f,
        _ => {handle_code(stream, 404); return;},
    };
    
    let header_string = format!(
        "HTTP/1.1 200\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );

    stream.write(&[header_string.as_bytes(), &body].concat()).unwrap();
}

fn handle_ws(req: &String, mut stream: TcpStream, data_dir: &Arc<Path>) {
    println!("handling ws");
    // handshake
    let key_in = match req.split("Sec-WebSocket-Key: ").nth(1) {
        Some(val) => &val[..24],
        _ => {handle_code(&mut stream, 400); return},
    };
    let mut hasher = Sha1::new();
    hasher.update(key_in.as_bytes());
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let key_out = base64::encode(hasher.digest().bytes());

    // Note: handle_ws assumes there is no more HTTP data left in the socket.
    // All data in the socket from now on will be parsed by WS.

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
    println!("Finished handshake; moving on to smrt");

    // drop heap stuff not needed for smrt::handle
    drop(req); 
    drop(hasher);
    drop(key_in);  // this should be on stack but whatever
    drop(key_out);

    // launch SMRT
    let mut stream = WsStream::new(stream);
    smrt::handle(&mut stream, data_dir);
}

fn handle_code(stream: &mut TcpStream, code: u16) {
    println!("handling {}", code);
    stream.write(format!("HTTP/1.1 {}\r\n\r\n", code).as_bytes()).unwrap();
}

fn handle_robots(stream: &mut TcpStream) {
    println!("handling robots");
    stream.write(
        b"HTTP/1.1 200\r\nContent-Length: 25\r\n\r\n\
        User-agent: *\nDisallow: /"
    ).unwrap();
}

pub fn handle(mut stream: TcpStream, data_dir: &Arc<Path>) {
    // Handles GET requests (where first four bytes "GET " already consumed)
    let mut buf = [0; 1024];  // todo: think more about sizes
    stream.read(&mut buf).unwrap();
    let req = String::from_utf8_lossy(&buf).to_string();
    // assert!(req.ends_with("\r\n\r\n"));  // todo: fill more if this crashes

    let path = match req.split(' ').nth(0) {
        Some(p) => p,
        None => return,  // faulty HTTP
    };
    println!("Recieved path: {}", path);

    match path {
        "/" | ""        => handle_file("page/index.html", &mut stream),
        "/favicon.ico"  => handle_file("page/favicon.ico", &mut stream),
        "/jspack.js"    => handle_file("page/jspack.js", &mut stream),  // todo: remove
        "/ws"           => {handle_ws(&req, stream, data_dir); return},  // start WS
        "/robots.txt"   => handle_robots(&mut stream),
        _               => handle_code(&mut stream, 404),  // reject everything else
    };
    stream.flush().unwrap();
}
