use std::{sync::Arc, io::{Read, Write}, path::Path};

use crate::{constants::*, db, msg, helpers::{Res, read_exact, full_write}};

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

fn send_messages(stream: &mut (impl Read + Write), msgs: &[MessageSt], first_id: u32) -> Res {
    // converts MessageSt to MessageOut and sends each into stream
    // msg::storage_to_packet
    // TcpStream::write
    if msgs.is_empty() { return Ok(()) }

    let mut buffer = [0; MAX_FETCH_AMOUNT as usize * MSG_OUT_SIZE];
    for (i, msg) in msgs.iter().enumerate() {
        let msg_pos: usize = MSG_OUT_SIZE * i as usize;
        msg::storage_to_packet(
            msg,
            &mut buffer[msg_pos..][..MSG_OUT_SIZE],
            first_id + i as u32,
        );  // todo: send one msg_id per packet to reduce redundant info
    }

    full_write(
        stream,
        &buffer[..msgs.len()*MSG_OUT_SIZE],
        "Failed to send messages in SMRT",
    )
}

pub fn handle(mut stream: (impl Read + Write), data_dir: &Arc<Path>) -> Res {
    let mut pad_buf = [0; PADDING_SIZE];
    let mut snd_buf = [0; MSG_IN_SIZE];  // size of msg packet
    let mut chat_id_buf = [0; CHAT_ID_SIZE];
    let mut qry_arg_buf = [0; QUERY_ARG_SIZE];

    let mut st_buf = [0; MSG_ST_SIZE];
    loop {
        read_exact(&mut stream, &mut pad_buf, "Failed to get first pad")?;
        match pad_buf {
            SEND_PADDING => {
                read_exact(&mut stream, &mut snd_buf, "Failed to read cypher")?;
                read_exact(&mut stream, &mut pad_buf, "Failed to read end pad (snd)")?;
                if pad_buf != END_PADDING { return Err("Incorrect end padding (snd)") }
 
                chat_id_buf = msg::packet_to_storage(&snd_buf, &mut st_buf);
                db::push(&get_chat_file(&chat_id_buf, data_dir), &st_buf)?;
            },
            FETCH_PADDING => {
                // fill fetch buffer
                read_exact(&mut stream, &mut chat_id_buf, "Failed to read fetch chat id")?;
                read_exact(&mut stream, &mut pad_buf, "Failed to read end pad (fch)")?;
                if pad_buf != END_PADDING { return Err("Incorrect end padding (fch)") }

                // get arguments for the db fetch
                let path = get_chat_file(&chat_id_buf, data_dir);
                // todo: add count to fetch message

                // fetch from db & send to client
                let (id, messages) = db::fetch(&path, DEFAULT_FETCH_AMOUNT)?;
                send_messages(&mut stream, &messages, id)?;
            },
            QUERY_PADDING => {
                // fill chat_id and arg buffer
                read_exact(&mut stream, &mut chat_id_buf, "Failed to read query chat id")?;
                read_exact(&mut stream, &mut qry_arg_buf, "Failed to read query args")?;
                read_exact(&mut stream, &mut pad_buf, "Failed to read end pad (qry)")?;
                if pad_buf != END_PADDING { return Err("Incorrect end padding (qry)") }
                
                // get arguments for the db fetch
                let (msg_id, count, forward) = query_bytes_to_args(&qry_arg_buf);
                let path = get_chat_file(&chat_id_buf, data_dir);

                // return query
                let (id, messages) = db::query(&path, msg_id, count, forward)?;
                send_messages(&mut stream, &messages, id)?;
            },
            _ => return Err("Recieved invalid SMRT header"),
        }
    }
}
