use std::collections::VecDeque;
use std::io::Write;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use publichat::helpers::*;
use publichat::constants::*;

mod msg;
use msg::Message;

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
        Err("Received invalid header padding")
    }
}

struct GlobalState {
    queue: VecDeque<Message>,
    chat_key: Hash,
    chat_id: Hash,
    min_id: u32,
    max_id: u32,  // inclusive
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
                println!("{}", msg);
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
                    println!("{}", msg);
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


    let user = b"tui guy                         ";
    let chat = b"12";
    let (chat_key, chat_id) = crypt::hash_twice(chat);
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

    while state.lock().unwrap().queue.is_empty() {
        comm::send_fetch(&mut stream, &chat_id);
        thread::sleep_ms(1000);
    }
    loop {
        comm::send_query(
            &mut stream,
            &chat_id,
            true,
            50,
            state.lock().unwrap().max_id,
        );
        thread::sleep_ms(200);
    }

}
