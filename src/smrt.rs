use std::{sync::Arc, io::{Read, Write}, path::Path};

use crate::{constants::*, db, helpers::*};

fn query_bytes_to_args(data: &[u8; 4]) -> (u32, u8, bool) {
    let forward = data[0] & 0x80 != 0;  // check first bit
    let count = data[0] & 0x7f; // take the last 7 bits
    let id = u32::from_be_bytes(*data) & 0x00_ff_ff_ff;  // take the three last bytes
    (id, count, forward)
}

fn get_chat_file(chat_id: &Hash, data_dir: &Path) -> std::path::PathBuf {
    // encode hash into b64 and append to data_dir
    use base64::{Config, CharacterSet::UrlSafe};
    data_dir.join(base64::encode_config(chat_id, Config::new(UrlSafe, false)))
}

pub fn packet_to_storage(src: &MessageIn, dest: &mut MessageSt) -> Hash {
    // Takes bytes from client, 
    // Extract RSA pub, cypher, signature. Generate time.
    // Write buffer intended for storage into dest.
    // Return chat_id if successful
    
    let msg_time: u64 = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap()
        .as_millis().try_into().expect("go play with your hoverboard");
    dest[..TIME_SIZE].clone_from_slice(&msg_time.to_be_bytes());
    dest[MSG_ST_CYPHER_START..].clone_from_slice(&src[MSG_IN_CYPHER..]);

    // return the chat ID
    let mut chat_id = [0; CHAT_ID_SIZE];
    chat_id.clone_from_slice(&src[MSG_IN_CHAT_ID..][..CHAT_ID_SIZE]);
    chat_id
}

fn send_messages(
    stream: &mut (impl Read + Write),
    chat_id: &Hash,
    msg_id: u32,  // id of first message in msgs
    forward: bool,
    count: u8,
    msgs: Vec<u8>
) -> Res {
    // converts MessageSt to MessageOut and sends each into stream
    // msg::storage_to_packet
    // TcpStream::write
    if count > 127 { return Err("Tried to send too many messages") }

    // Use max size buffer - size not known, but stack is big anyway
    let mut buffer = [0; HED_OUT_SIZE + MSG_OUT_SIZE * MAX_FETCH_AMOUNT as usize];

    // construct header for messages
    buffer[HED_OUT_PAD..HED_OUT_CHAT_ID_BYTE].copy_from_slice(&MSG_PADDING);
    buffer[HED_OUT_CHAT_ID_BYTE..HED_OUT_MSG_ID].copy_from_slice(&chat_id[..1]);  // only 1st
    buffer[HED_OUT_MSG_ID..HED_OUT_MSG_COUNT].copy_from_slice(&msg_id.to_be_bytes()[1..]);
    buffer[HED_OUT_MSG_COUNT] = (u8::from(forward) << 7) | count;

    // fill buffer with messages
    buffer[HED_OUT_SIZE..][..msgs.len()].copy_from_slice(&msgs);

    // send
    full_write(
        stream,
        &buffer[..HED_OUT_SIZE + count as usize * MSG_OUT_SIZE],
        "Failed to send messages in SMRT",
    )
}

pub fn handle(mut stream: (impl Read + Write), globals: &Arc<Globals>) -> Res {
    let mut pad_buf = [0; PADDING_SIZE];
    let mut snd_buf = [0; MSG_IN_SIZE];  // size of msg packet
    let mut chat_id_buf = [0; CHAT_ID_SIZE];
    let mut qry_arg_buf = [0; QUERY_ARG_SIZE];

    let mut st_buf = [0; MSG_ST_SIZE];
    loop {
        read_exact(
            &mut stream,
            &mut pad_buf,
            "Failed to read SMRT pad. Socket timed out?",
        )?;
        match pad_buf {
            SEND_PADDING => {
                read_exact(&mut stream, &mut snd_buf, "Failed to read cypher")?;
                read_exact(&mut stream, &mut pad_buf, "Failed to read end pad (snd)")?;
                if pad_buf != END_PADDING { return Err("Incorrect end padding (snd)") }
 
                chat_id_buf = packet_to_storage(&snd_buf, &mut st_buf);
                db::push(&get_chat_file(&chat_id_buf, &globals.data_dir), &st_buf)?;
            },
            FETCH_PADDING => {
                // fill fetch buffer
                read_exact(&mut stream, &mut chat_id_buf, "Failed to read fetch chat id")?;
                read_exact(&mut stream, &mut pad_buf, "Failed to read end pad (fch)")?;
                if pad_buf != END_PADDING { return Err("Incorrect end padding (fch)") }

                // get arguments for the db fetch
                let path = get_chat_file(&chat_id_buf, &globals.data_dir);
                // todo: add count to fetch message

                // fetch from db & send to client
                let (count, msg_id, messages) = db::fetch(&path, DEFAULT_FETCH_AMOUNT)?;
                send_messages(&mut stream, &chat_id_buf, msg_id, true, count, messages)?;
            },
            QUERY_PADDING => {
                // fill chat_id and arg buffer
                read_exact(&mut stream, &mut chat_id_buf, "Failed to read query chat id")?;
                read_exact(&mut stream, &mut qry_arg_buf, "Failed to read query args")?;
                read_exact(&mut stream, &mut pad_buf, "Failed to read end pad (qry)")?;
                if pad_buf != END_PADDING { return Err("Incorrect end padding (qry)") }
                
                // get arguments for the db fetch
                let (msg_id, count, forward) = query_bytes_to_args(&qry_arg_buf);
                let path = get_chat_file(&chat_id_buf, &globals.data_dir);

                // return query
                let (count, msg_id, messages) = db::query(&path, msg_id, count, forward)?;
                send_messages(&mut stream, &chat_id_buf, msg_id, forward, count, messages)?;
            },
            _ => return Err("Recieved invalid SMRT header"),
        }
    }
}
