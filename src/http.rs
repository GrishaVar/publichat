use std::{net::TcpStream, sync::Arc, path::Path, fs, io::Read};
use sha1_smol::Sha1;
use crate::smrt;
use crate::ws::WsStream;
use crate::helpers::{Res, full_write};

fn handle_file(file: &str, stream: &mut TcpStream) -> Res {
    // BE CAREFUL WITH THIS ONE!
    let body = match fs::read(file) {
        Ok(f) => f,
        _ => return handle_http_code(stream, 404),
    };
    
    let header_string = format!(
        "HTTP/1.1 200\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );

    full_write(
        stream,
        &[header_string.as_bytes(), &body].concat(),
        "Failed to send/read file ",
    )
}

fn handle_ws(req: &String, mut stream: TcpStream, data_dir: &Arc<Path>) -> Res {
    // handshake
    let key_in = match req.split("Sec-WebSocket-Key: ").nth(1) {
        Some(val) => &val[..24],
        _ => {
            handle_http_code(&mut stream, 400)?;
            return Err("Couldn't find WS key");
        },
    };
    let mut hasher = Sha1::new();
    hasher.update(key_in.as_bytes());
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let key_out = base64::encode(hasher.digest().bytes());

    // Note: handle_ws assumes there is no more HTTP data left in the socket.
    // All data in the socket from now on will be parsed by WS.

    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
        Upgrade: websocket\r\n\
        Connection: Upgrade\r\n\
        Sec-WebSocket-Accept: {}\r\n\r\n",
        key_out,
    );

    full_write(
        &mut stream,
        response.as_bytes(),
        "Failed to send WS upgrade accept packet"
    )?;

    // drop heap stuff not needed for smrt::handle
    drop(req); 
    drop(hasher);
    drop(key_in);  // this should be on stack but whatever
    drop(key_out);

    // launch SMRT
    let mut stream = WsStream::new(stream);
    smrt::handle(&mut stream, data_dir)
}

fn handle_http_code(stream: &mut TcpStream, code: u16) -> Res {
    full_write(
        stream,
        format!("HTTP/1.1 {}\r\n\r\n", code).as_bytes(),
        "Failed to send HTTP status code"
    )
}

fn handle_robots(stream: &mut TcpStream) -> Res {
    full_write(
        stream,
        b"HTTP/1.1 200\r\nContent-Length: 25\r\n\r\nUser-agent: *\nDisallow: /",
        "Failed to send robots",
    )
}

pub fn handle(mut stream: TcpStream, data_dir: &Arc<Path>) -> Res {
    // Handles GET requests (where first four bytes "GET " already consumed)
    let mut buf = [0; 1024];  // todo: think more about sizes
    if let Err(_) = stream.read(&mut buf) { return Err("Failed to read HTTP packet") }
    let req = String::from_utf8_lossy(&buf).to_string();
    // assert!(req.ends_with("\r\n\r\n"));  // todo: fill more if this crashes

    let path = match req.split(' ').nth(0) {
        Some(p) => p,
        None => return Err("Failed to find HTTP path"),  // faulty HTTP
    };

    match path {
        "/" | ""        => handle_file("page/index.html", &mut stream),
        "/favicon.ico"  => handle_file("page/favicon.ico", &mut stream),
        "/jspack.js"    => handle_file("page/jspack.js", &mut stream),  // todo: remove
        "/ws"           => handle_ws(&req, stream, data_dir),  // start WS
        "/robots.txt"   => handle_robots(&mut stream),
        _               => handle_http_code(&mut stream, 404),  // reject everything else
    }
}
