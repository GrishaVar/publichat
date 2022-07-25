use std::collections::VecDeque;
use std::error::Error;
use std::io::Write;
use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::mem;

use publichat::helpers::*;
use publichat::constants::*;

mod msg;
use msg::Message;

mod common;
use common::*;

mod display;
use display::Display;

mod crypt;
mod comm;

fn parse_header(header: &[u8; HED_OUT_SIZE]) -> Result<(u8, u32, u8, bool), &'static str> {
    // returns (chat id byte, message id, message count, forward)
    if header[..PADDING_SIZE] == MSG_PADDING {
        Ok((
            header[HED_OUT_CHAT_ID_BYTE],  // TODO: poorly named consts here...
            u32::from_be_bytes(header[HED_OUT_CHAT_ID_BYTE..][..QUERY_ARG_SIZE].try_into().unwrap()) & 0x00_ff_ff_ff,
            header[HED_OUT_MSG_COUNT] & 0b0111_1111,  // can't fail unless consts wrong ^
            header[HED_OUT_MSG_COUNT] & 0b1000_0000 > 0,
        ))
    } else {
        println!("{header:?}");
        Err("Received invalid header padding")
    }
}


fn listener(mut stream: TcpStream, state: Arc<Mutex<GlobalState>>) -> Res {
    let mut hed_buf = [0; HED_OUT_SIZE];
    loop {
        read_exact(&mut stream, &mut hed_buf, "Failed to read head buffer")?;
        // TODO: what should happen when this fails?
        // I guess thread closes and require reconnect
        
        let (chat, first_id, count, forward) = parse_header(&hed_buf)?;
        if count == 0 { continue }  // skip no messages

        // read messages expected from header
        let mut buf = vec![0; count as usize * MSG_OUT_SIZE];  // TODO: consider array
        read_exact(&mut stream, &mut buf, "Failed to bulk read fetch")?;

        let mut s = state.lock().map_err(|_| "Failed to lock state")?;
        if chat != s.chat_id[0] { continue }  // skip wrong chat

        let last_id = first_id + count as u32 - 1;  // inclusive. Can't undeflow

        if s.min_id > s.max_id {  // initial fetch
            // handle initial fetch separately; skip all checks
            for msg in buf.chunks_exact(MSG_OUT_SIZE) {
                let msg = Message::new(msg.try_into().unwrap(), &s.chat_key)?;
                s.queue.push_back(msg);
            }
            s.min_id = first_id;
            s.max_id = last_id;
            continue;  // initial fetch finished, move to next packet
        }

        if s.max_id + 1 < first_id ||  // disconnected ahead
           s.min_id > last_id + 1 ||  // disconnected behind
           (s.min_id <= first_id && last_id <= s.max_id) ||  // already have this
           (first_id < s.min_id && s.max_id < last_id)  // overflow on both sides
        { continue }  // skip all these

        if forward {
            if last_id > s.max_id {  // good proper data here
                let i = if first_id <= s.max_id {s.max_id-first_id+1} else {0};
                assert_eq!(s.max_id + 1, first_id + i);
                for msg in buf.chunks_exact(MSG_OUT_SIZE).skip(i as usize) {
                    let msg = Message::new(msg.try_into().unwrap(), &s.chat_key)?;
                    // println!("{}", msg);
                    s.queue.push_back(msg);
                }
                // buf.chunks_exact(MSG_OUT_SIZE)
                //     .skip(i as usize)
                //     .map(|msg| Message::new(msg.try_into().unwrap(), &s.chat_key)?)
                //     .for_each(|msg| s.queue.push_back(msg));
                s.max_id = last_id;
            } else {  // points forwards but behind our data
                continue;
            }
        } else {  // not forwards (for scrolling up)
            todo!()
        }
    }
}


fn requester(
    mut stream: TcpStream,
    state: Arc<Mutex<GlobalState>>,
    snd_rx: mpsc::Receiver<String>,
) -> Res {
    let user_id = state.lock().map_err(|_| "Failed to lock state")?.user_id;
    let chat_id = state.lock().map_err(|_| "Failed to lock state")?.chat_id;
    let chat_key = state.lock().map_err(|_| "Failed to lock state")?.chat_key;
    let mut cypher_buf = [0; CYPHER_SIZE];
    let mut signature_buf = [0; SIGNATURE_SIZE];

    // Fetch until we get first message packet
    while state.lock().map_err(|_| "Failed to lock state")?.queue.is_empty() {
        comm::send_fetch(&mut stream, &chat_id)?;
        if let Ok(msg) = snd_rx.try_recv() {
            cypher_buf = Message::make_cypher(&msg, &chat_key).unwrap();  // TODO: unwrap
            signature_buf = [0; SIGNATURE_SIZE];
            comm::send_msg(&mut stream, &chat_id, &cypher_buf, &signature_buf)?;
        }
        thread::sleep(FQ_DELAY);
    }

    // Query for scroll or fetch for more
    loop {
        comm::send_query(
            &mut stream,
            &chat_id,
            true,
            50,
            state.lock().unwrap().max_id,
        )?;
        if let Ok(msg) = snd_rx.try_recv() {
            cypher_buf = Message::make_cypher(&msg, &chat_key).unwrap();  // TODO: unwrap
            signature_buf = [0; SIGNATURE_SIZE];
            comm::send_msg(&mut stream, &chat_id, &cypher_buf, &signature_buf)?;
        }
        thread::sleep(FQ_DELAY);
    }
}

fn main() -> Result<(), Box<dyn Error>> {  // TODO: return Res instead?
    println!("Starting client...");
    // arguments: addr:port title user

    let mut args = std::env::args().skip(1).collect::<Vec<_>>();

    let server_addr = args.get(0).ok_or("No addr given")?
        .to_socket_addrs()?
        .next().ok_or("Zero addrs received?")?;

    let chat = mem::take(args.get_mut(1).ok_or("No title given")?);
    let (chat_key, chat_id) = crypt::hash_twice(chat.as_bytes());

    let user = mem::take(args.get_mut(2).ok_or("No username given")?);
    let user_id = crypt::hash(user.as_bytes());

    println!("Connecting to server {:?}...", server_addr);
    let mut stream = TcpStream::connect(server_addr)?;
    println!("Connected!");

    stream.write_all(b"SMRT")?;

    let queue = VecDeque::with_capacity(500);
    let state = GlobalState {
        queue,
        chat_key,
        chat_id,
        user_id,
        min_id: 1,
        max_id: 0,
    };
    let state = Arc::new(Mutex::new(state));

    // mpsc for sending messages
    let (msg_tx, msg_rx) = mpsc::channel::<String>();

    // start listener thread
    let stream2 = stream.try_clone()?;
    let state2 = state.clone();
    println!("Starting listener thread...");
    thread::spawn(|| {
        match listener(stream2, state2) {
            Ok(_) => println!("Listener thread finished"),
            Err(e) => println!("Listener thread crashed: {e}"),
        }
    });

    // start requester thread
    let state3 = state.clone();
    println!("Starting requester thread...");
    thread::spawn(|| {
        match requester(stream, state3, msg_rx) {  // requester sends messages from tx to server
            Ok(_) => println!("Request loop finished"),
            Err(e) => println!("Request loop crashed: {e}"),
        };
    });

    // start drawer thread
    println!("Starting drawer...");
    match Display::start(state, msg_tx) {  // drawer recieves text input and send to requester
        Ok(_) => println!("Drawer finished"),
        Err(e) => println!("Drawer crashed: {e}"),
    }

    Ok(())
}
