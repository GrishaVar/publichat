use std::collections::VecDeque;
use std::io::Write;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use sha3::{Sha3_256, Digest};

use publichat::helpers::*;
use publichat::constants::*;

mod msg;
use msg::Message;

fn send_msg(
    stream: &mut TcpStream,
    chat: &Hash,
    user: &Hash,
    cypher: &Contents,
) -> Res {
    // TODO: rename constants to something direction-agnostic
    let mut buf = [0; PADDING_SIZE + MSG_IN_SIZE + PADDING_SIZE];

    buf[..PADDING_SIZE].copy_from_slice(&SEND_PADDING);  // TODO: make padding const?
    buf[PADDING_SIZE+MSG_IN_CHAT_ID..][..CHAT_ID_SIZE].copy_from_slice(chat);
    buf[PADDING_SIZE+MSG_IN_RSA..][..HASH_SIZE].copy_from_slice(user);
    buf[PADDING_SIZE+MSG_IN_CYPHER..][..CYPHER_SIZE].copy_from_slice(cypher);
    buf[PADDING_SIZE+MSG_IN_SIZE..][..PADDING_SIZE].copy_from_slice(&END_PADDING);

    full_write(stream, &buf, "Failed to send message")
}

fn fetch_msgs(
    stream: &mut TcpStream,
    chat: &Hash,
) -> Res {
    // Send fetch request
    let mut send_buf = [0; PADDING_SIZE + FCH_SIZE + PADDING_SIZE];

    send_buf[..PADDING_SIZE].copy_from_slice(&FETCH_PADDING);
    send_buf[PADDING_SIZE+FCH_CHAT_ID..][..CHAT_ID_SIZE].copy_from_slice(chat);

    full_write(stream, &send_buf, "Failed to send fetch")?;

    // receive response  TODO: receive everything in different thread, like in js?
    let mut recv_header_buf = [0; HED_OUT_SIZE];
    read_exact(stream, &mut recv_header_buf, "Failed to read fetch response header")?;

    if recv_header_buf[..PADDING_SIZE] != *b"msg" {
        return Err("Fetch failed: incorrect padding received from server");
    }

    if recv_header_buf[HED_OUT_CHAT_ID_BYTE] != chat[0] {
        return Err("Fetch failed: got data from wrong chat");  // TODO: thread!!
    }

    // let mut msg = 
    // println!("got some data\n{:?}", )

    todo!()
}

fn query_msgs() {}

fn hash_twice(title: &[u8]) -> (Hash, Hash) {
    let mut once = [0; HASH_SIZE];
    let mut twice = [0; HASH_SIZE];

    // hash once
    let mut hasher = Sha3_256::new();
    hasher.update(title);
    once.copy_from_slice(&hasher.finalize());

    // hash twice
    let mut hasher = Sha3_256::new();
    hasher.update(once);
    twice.copy_from_slice(&hasher.finalize());

    // TODO: do hashes in a loop? zip source and dest? Might be overkill...
    (once, twice)
}

fn parse_header(header: &[u8; HED_OUT_SIZE]) -> Result<(u8, u32, u8, bool), &'static str> {
    // returns (chat id byte, message id, message count, forward)
    if header[..PADDING_SIZE] == MSG_PADDING {
        Ok((
            header[HED_OUT_CHAT_ID_BYTE],
            u32::from_be_bytes(header[HED_OUT_MSG_ID..][..QUERY_ARG_SIZE].try_into().unwrap()),
            header[HED_OUT_MSG_COUNT] & 0b0111_1111,  // can't fail unless consts wrong ^
            header[HED_OUT_MSG_COUNT] & 0b1000_0000 > 0,
        ))
    } else {
        Err("Received invalid header padding")
    }
}

struct GlobalState {
    queue: VecDeque<Message>,
    chat_key: Hash,
    chat_id: Hash,
    min_id: u32,
    max_id: u32,
}

fn clear_stream(stream: &mut TcpStream, n: usize) -> Res {
    // clear n messages from stream
    // todo: how to clear without allocating?
    let mut buf = vec![0; n * MSG_OUT_SIZE];
    read_exact(stream, &mut buf, "Failed to clear stream")
}

fn listener(
    mut stream: TcpStream,
    state: Arc<Mutex<GlobalState>>,
) -> Res {
    let mut hed_buf = [0; HED_OUT_SIZE];
    loop {
        read_exact(&mut stream, &mut hed_buf, "Failed to read head buffer")?;
        // TODO: what should happen when this fails?
        // I guess thread closes and require reconnect
        
        let (chat, id, count, forward) = parse_header(&hed_buf)?;
        let last_id = id + count as u32;
        let mut state = state.lock().map_err(|_| "Failed to lock state")?;

        if chat != state.chat_id[0] || count == 0{
            // skip old data and empty packets
            clear_stream(&mut stream, count as usize)?;
            continue;
        }

        if state.min_id > state.max_id {  // fetch
            // handle fetch separately; skip all checks
            let mut buf = vec![0; count as usize * MSG_OUT_SIZE];
            read_exact(&mut stream, &mut buf, "Failed to bulk read fetch")?;
            for msg in buf.chunks_exact(MSG_OUT_SIZE) {
                let msg = Message::new(msg.try_into().unwrap(), &state.chat_key)?;
                println!("{}", msg);
                state.queue.push_back(msg);
            }
            state.min_id = id;
            state.max_id = id + count as u32 - 1;
        } else {
            clear_stream(&mut stream, count as usize)?;
            continue;
        }

        // if state.min_id <= state.max_id {
        //     // skip all checks on initial fetch. min/max initialised swapped
        //     if 
        // }

        // if id > state.max_id + 1 || last_id < state.min_id {continue}  // data not connected


        // if state.queue.is_empty() != (id == 0) {continue}  // skip non-zero for empty chat
        // if forward && id > 0 && state.max_id > id - 1 {continue}


        // if forward && id != 0 && queue.back().unwrap().msg_id != id - 1 {continue}
        // if !forward && queue.front().unwrap().msg_id != id + count as u32 {continue}
        // // TODO: use Option::contains when it's stable ^

        // let mut msgs = vec![0; count as usize * MSG_OUT_SIZE];
        // read_exact(&mut stream, &mut msgs, "Failed to bulk read messages")?;
        // if forward {
        //     for i in 0..count as usize {
        //         msg_buf.copy_from_slice(&msgs[i*MSG_OUT_SIZE..][..MSG_OUT_SIZE]);
        //         Message::new(msg_buf, &cur_chat)?;
        //     }
        // }
    }
}

fn main() {
    println!("Starting client...");

    let server_addr = {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if let Some(addr) = args.last() {
            addr.to_owned()
        } else {
            println!("No address given");
            std::process::exit(1);
        }
    };
    println!("Connecting to server {}...", server_addr);
    let mut stream = TcpStream::connect(&server_addr).unwrap_or_else(|e| {
        println!("Failed to connect to to server: {}", e);
        std::process::exit(2);
    });
    println!("Connected!");

    stream.write_all(b"SMRT").unwrap_or_else(|e| {
        println!("Failed to write SMRT header: {}", e);
        std::process::exit(3);
    });


    let mut cur_user = b"tui guy                         ";
    let mut cur_chat = b"12";
    let (chat_key, chat_id) = hash_twice(cur_chat);
    let queue = VecDeque::with_capacity(500);
    let state = GlobalState {
        queue,
        chat_key,
        chat_id,
        min_id: 1,
        max_id: 0,
    };
    let state = Arc::new(Mutex::new(state));

    // start listener thread
    let stream2 = stream.try_clone().map_err(|_| "Failed to clone stream").unwrap();
    let state2 = state.clone();
    thread::spawn(|| {
        println!("Starting listener thread.");
        if let Err(e) = listener(stream2, state2) {
            println!("Listener thread crashed: {}", e);
        } else {
            println!("Listener thread finished");
        }
    });


    let msg = b"\
    12345678\
    12345678\
    12345678\
    12345678\
    12345678\
    12345678\
    12345678\
    12345678\
    12345678\
    12345678\
    12345678\
    12345678\
    12345678\
    12345678\
    12345678\
    12345678";

    // send_msg(&mut stream, &chat_hash_2, cur_user, msg);

    loop {
        full_write(
            &mut stream,
            &[&b"fch"[..], &chat_id[..], &b"end"[..]].concat(),
            "failed to write",
        );
        thread::sleep_ms(1000);
    }

}
