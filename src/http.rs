use std::{net::TcpStream, sync::Arc, io::Read};
use sha1_smol::Sha1;
use crate::smrt;
use crate::ws::WsStream;
use crate::helpers::{Res, full_write, Globals};

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

fn handle_ws(req: &str, mut stream: TcpStream, globals: &Arc<Globals>) -> Res {
    // handshake
    let key_in = match req.split("Sec-WebSocket-Key: ").nth(1) {
        Some(val) => &val[..24],
        _ => {
            send_code(&mut stream, 400)?;
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
    drop(hasher);
    drop(key_out);

    // launch SMRT
    let mut stream = WsStream::new(stream);
    smrt::handle(&mut stream, globals)
}

fn send_code(stream: &mut TcpStream, code: u16) -> Res {
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

fn handle_version(stream: &mut TcpStream) -> Res {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()  // TODO: consider using reader to avoid Vec allocation?
        .map_err(|_| "Failed to exec git command")?;

    let hash = output.stdout;  // has a \n at the end!
    if hash.len() != 41 { return Err("Retrieved hash not 40 charachters") }

    // this next part is pretty sinful, but avoids allocations (hash len = 40)
    const HEADER: &[u8; 76] = b"\
        HTTP/1.1 200\r\n\
        Content-Length: 40\r\n\r\n\
        1234567890123456789012345678901234567890";  // these are 40 charachters
    let mut data = HEADER.clone();
    data[HEADER.len()-40..].copy_from_slice(&hash[..40]);

    full_write(stream, &data, "Failed to send commit hash")
}

pub fn handle(mut stream: TcpStream, globals: &Arc<Globals>) -> Res {
    // Handles GET requests (where first four bytes "GET " already consumed)
    let mut buf = [0; 1024];  // todo: think more about sizes
    stream.read(&mut buf).map_err(|_| "Failed to read HTTP packet")?;
    let req = std::str::from_utf8(&buf).map_err(|_| "Recieved non-utf8 HTTP")?;

    if !req.ends_with("\0\0\0\0\0\0\0\0") {
        // Received HTTP packet was (probably) bigger than 1 KiB
        send_code(&mut stream, 413)?;
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
        "/version"       => handle_version(&mut stream),
        _                => send_code(&mut stream, 404),  // reject everything else
    }
}
