use std::collections::VecDeque;
use std::error::Error;
use std::io::Write;
use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::mem;

use publichat::helpers::*;
use publichat::buffers::{
    msg_head,
    msg_out_c as msg_out,
    cypher::Buf as CypherBuf,
};

mod msg;
use msg::Message;

mod common;
use common::*;

mod display;
use display::Display;

mod crypt;
use crypt::{sha, ed25519};

mod comm;

// mutex lock shortuct
macro_rules! lock { ($s:tt) => { $s.lock().map_err(|_| "Failed to lock state") } }

fn parse_header(header: &msg_head::Buf) -> Result<(u8, u32, u8, bool), &'static str> {
    // returns (chat id byte, message id, message count, forward)
    let (pad_buf, cid_buf, mid_buf, count_buf) = msg_head::split(header);
    let mut msg_id = [0; 4];  // TODO: this is ugly. Consider combining cid and mid
    msg_id[1..].copy_from_slice(mid_buf);  // can't fail
    if pad_buf == msg_head::PAD {
        Ok((
            cid_buf[0],  // can't fail
            u32::from_be_bytes(msg_id),
            count_buf[0] & 0b0111_1111,  // can't fail
            count_buf[0] & 0b1000_0000 > 0,
        ))
    } else {
        println!("{header:?}");
        Err("Received invalid header padding")
    }
}


// Listener thread handles parsing data received from server
// - Receive message packets; parse; break up into messages
// - Insert into queue in correct place
fn listener(mut stream: TcpStream, state: Arc<Mutex<GlobalState>>) -> Res {
    let mut hed_buf = msg_head::DEFAULT;
    loop {
        read_exact(&mut stream, &mut hed_buf, "Failed to read head buffer")?;
        // TODO: what should happen when this fails?
        // I guess thread closes and require reconnect

        let (chat, first_id, count, forward) = parse_header(&hed_buf)?;
        if count == 0 { continue }  // skip no messages

        // read messages expected from header
        let mut buf = vec![0; count as usize * msg_out::SIZE];  // TODO: consider array
        read_exact(&mut stream, &mut buf, "Failed to bulk read fetch")?;

        let mut s = lock!(state)?;
        if chat != s.chat_id[0] { continue }  // skip wrong chat

        let last_id = first_id + count as u32 - 1;  // inclusive. Can't undeflow

        if s.min_id > s.max_id {  // initial fetch
            // handle initial fetch separately; skip all checks
            for msg in buf.chunks_exact(msg_out::SIZE) {
                let msg = Message::new(msg.try_into().unwrap(), &s.chat_key)?;
                s.queue.push_back(msg);
            }
            s.min_id = first_id;
            s.max_id = last_id;
            continue;  // initial fetch finished, move to next packet
        }

        if s.max_id + 1 < first_id  // disconnected ahead
           || s.min_id > last_id + 1  // disconnected behind
           || (s.min_id <= first_id && last_id <= s.max_id)  // already have this
           || (first_id < s.min_id && s.max_id < last_id)  // overflow on both sides
        { continue }  // skip all these

        if forward {
            if last_id > s.max_id {  // good proper data here
                let i = if first_id <= s.max_id {s.max_id-first_id+1} else {0};
                assert_eq!(s.max_id + 1, first_id + i);
                for msg in buf.chunks_exact(msg_out::SIZE).skip(i as usize) {
                    let msg = Message::new(msg.try_into().unwrap(), &s.chat_key)?;
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


// Requester thread handles sending requests (fetch & query) to server
fn requester(mut stream: TcpStream, state: Arc<Mutex<GlobalState>>) -> Res {
    let chat_id = lock!(state)?.chat_id;

    // Fetch until we get first message packet
    while lock!(state)?.queue.is_empty() {
        comm::send_fetch(&mut stream, &chat_id)?;
        thread::sleep(FQ_DELAY);
    }

    // Query for scroll or fetch for more
    loop {
        comm::send_query(
            &mut stream,
            &chat_id,
            true,
            50,
            state.lock().unwrap().max_id,  // TODO: edit ID
        )?;
        thread::sleep(FQ_DELAY);
    }
}


// Sender threads sends messages to server as they come in from snd_rx
fn sender(
    mut stream: TcpStream,
    state: Arc<Mutex<GlobalState>>,
    snd_rx: mpsc::Receiver<String>,
    keypair: ed25519::Keypair,
) -> Res {
    let chat_id = lock!(state)?.chat_id;
    let chat_key = lock!(state)?.chat_key;

    let mut cypher_buf: CypherBuf;
    let mut signature_buf: ed25519::SigBuf;

    loop {
        let msg = snd_rx.recv().map_err(|_| "Message sender hung up")?;  // blocks
        if msg.split_whitespace().next().is_none() { continue; }  // empty msg

        cypher_buf = Message::make_cypher(&msg, &chat_key, keypair.public.as_bytes())?;
        signature_buf = ed25519::sign(&cypher_buf, &keypair);
        comm::send_msg(&mut stream, &chat_id, &cypher_buf, &signature_buf)?;
    }
}


fn main() -> Result<(), Box<dyn Error>> {  // TODO: return Res instead?
    eprintln!("Starting client...");
    // arguments: addr:port title user

    let mut args = std::env::args().skip(1).collect::<Vec<_>>();

    let server_addr = args.get(0).ok_or("No addr given")?
        .to_socket_addrs()?
        .next().ok_or("Zero addrs received?")?;

    let chat = mem::take(args.get_mut(1).ok_or("No title given")?);
    let chat_key = sha::hash(chat.as_bytes());
    let chat_id = sha::hash(&chat_key);

    let user = mem::take(args.get_mut(2).ok_or("No username given")?);
    let keypair = ed25519::make_keypair(user.as_bytes())?;

    eprintln!("Connecting to server {:?}...", server_addr);
    let mut stream = TcpStream::connect(server_addr)?;
    eprintln!("Connected!");

    stream.write_all(b"SMRT")?;

    let queue = VecDeque::with_capacity(500);
    let state = GlobalState {
        queue,
        chat_key,  // TODO: this doesn't change; store somewhere else?
        chat_id,
        min_id: 1,
        max_id: 0,
    };
    let state = Arc::new(Mutex::new(state));

    // mpsc for sending messages
    let (msg_tx, msg_rx) = mpsc::channel::<String>();

    // start listener thread
    let stream_c = stream.try_clone()?;
    let state_c = state.clone();
    eprintln!("Starting listener thread...");
    thread::spawn(|| {
        match listener(stream_c, state_c) {
            Ok(_) => eprintln!("Listener thread finished"),
            Err(e) => eprintln!("Listener thread crashed: {e}"),
        }
    });

    // start requester thread
    let stream_c = stream.try_clone()?;
    let state_c = state.clone();
    eprintln!("Starting requester thread...");
    thread::spawn(|| {
        match requester(stream_c, state_c) {
            Ok(_) => eprintln!("Requester loop finished"),
            Err(e) => eprintln!("Requester loop crashed: {e}"),
        };
    });

    // start sender thread
    let stream_c = stream.try_clone()?;
    let state_c = state.clone();
    eprintln!("Starting requester thread...");
    thread::spawn(|| {
        match sender(stream_c, state_c, msg_rx, keypair) {
            Ok(_) => eprintln!("Sender loop finished"),
            Err(e) => eprintln!("Sender loop crashed: {e}"),
        };
    });

    // start drawer thread
    eprintln!("Starting drawer...");
    match Display::start(state, msg_tx, chat.as_str()) {
        Ok(_) => eprintln!("Drawer finished"),
        Err(e) => eprintln!("Drawer crashed: {e}"),
    }

    Ok(())
}
