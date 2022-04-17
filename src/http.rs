use std::{net::TcpStream, sync::Arc, io::Read};
use crate::smrt;
use crate::ws::WsStream;
use crate::helpers::{Res, full_write, Globals};

fn send_code(code: u16, stream: &mut TcpStream) -> Res {
    full_write(
        stream,
        format!("HTTP/1.1 {}\r\n\r\n", code).as_bytes(),
        "Failed to send HTTP status code"
    )
}

fn send_data(code: u16, data: &[u8], stream: &mut TcpStream) -> Res {
    let header_string = format!(
        "HTTP/1.1 {}\r\nContent-Length: {}\r\n\r\n",
        code,
        data.len(),
    );

    full_write(
        stream,
        &[header_string.as_bytes(), data].concat(),
        "Failed to send file",
    )
}

fn handle_robots(stream: &mut TcpStream) -> Res {
    const RESP_ROBOTS: &[u8] = b"\
        HTTP/1.1 200\r\n\
        Content-Length: 25\r\n\r\n\
        User-agent: *\nDisallow: /
    ";
    full_write(stream, RESP_ROBOTS, "Failed to send robots")
}

fn handle_version(stream: &mut TcpStream, globals: &Arc<Globals>) -> Res {
    // TODO: avoid allocations to pre-building packet?
    // TODO: should functions like this map_err to show where the issue is?
    send_data(200, &globals.git_hash, stream)
        .map_err(|_| "Failed to send version")
}

fn handle_ws(req: &str, mut stream: TcpStream, globals: &Arc<Globals>) -> Res {
    // handshake
    let key_in = match req.split("Sec-WebSocket-Key: ").nth(1) {
        Some(val) => &val[..24],
        _ => {
            send_code(400, &mut stream)?;
            return Err("Couldn't find WS key");
        },
    };
    WsStream::handshake(&mut stream, key_in)?;

    // launch SMRT
    let mut stream = WsStream::new(stream);
    smrt::handle(&mut stream, globals)
}

pub fn handle(mut stream: TcpStream, globals: &Arc<Globals>) -> Res {
    // Handles GET requests (where first four bytes "GET " already consumed)
    let mut buf = [0; 1024];  // todo: think more about sizes
    stream.read(&mut buf).map_err(|_| "Failed to read HTTP packet")?;
    let req = std::str::from_utf8(&buf).map_err(|_| "Recieved non-utf8 HTTP")?;

    if !req.ends_with("\0\0\0\0\0\0\0\0") {
        // Received HTTP packet was (probably) bigger than 1 KiB
        send_code(413, &mut stream)?;
        return Err("Received very large HTTP packet; aborted.")
    }

    let path = match req.split(' ').next() {
        Some(p) => p,
        None => return Err("Failed to find HTTP path"),  // faulty HTTP
    };

    match path {
        "/" | ""         => send_data(200, &globals.index_html, &mut stream),
        "/favicon.ico"   => send_data(200, &globals.favicon_ico, &mut stream),
        "/client.js"     => send_data(200, &globals.client_js, &mut stream),
        "/mobile" | "/m" => send_data(200, &globals.mobile_html, &mut stream),
        "/ws"            => handle_ws(req, stream, globals),  // start WS
        "/robots.txt"    => handle_robots(&mut stream),
        "/version"       => handle_version(&mut stream, globals),
        _                => send_data(404, &globals.four0four, &mut stream),
    }
}
