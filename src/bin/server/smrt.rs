use std::{sync::Arc, io::{Read, Write}, path::Path, convert::TryInto};

use crate::db::{
    self,
    MAX_FETCH_AMOUNT,
    DEFAULT_FETCH_AMOUNT,
};

use publichat::helpers::*;
use publichat::buffers::{
    pad,
    msg_head,
    hash::{self, Buf as HashBuf}, 
    qry_arg::{self, Buf as QryArgBuf},
    msg_in_s::{self as msg_in, Buf as MsgInBuf},
    msg_out_s::{self as msg_out, Buf as MsgStBuf},
};

fn query_bytes_to_args(data: &QryArgBuf) -> (u32, u8, bool) {
    let forward = data[0] & 0x80 != 0;  // check first bit
    let count = data[0] & 0x7f; // take the last 7 bits
    let id = u32::from_be_bytes(*data) & 0x00_ff_ff_ff;  // take the three last bytes
    (id, count, forward)
}

fn get_chat_file(chat_id: &HashBuf, data_dir: &Path) -> std::path::PathBuf {
    // encode hash into b64 and append to data_dir
    use base64::{Config, CharacterSet::UrlSafe};
    data_dir.join(base64::encode_config(chat_id, Config::new(UrlSafe, false)))
}

pub fn packet_to_storage(src: &MsgInBuf, dest: &mut MsgStBuf) -> HashBuf {
    // Takes bytes from client, copy data over into msg storage buffer
    // Return chat id

    let msg_time: u64 = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap()
        .as_millis().try_into().expect("go play with your hoverboard");

    let (src_id, src_data) = msg_in::split(src);
    let (dest_time, dest_data) = msg_out::split_mut(dest);

    dest_time.copy_from_slice(&msg_time.to_be_bytes());
    dest_data.copy_from_slice(src_data);

    // return the chat ID
    // TODO: return reference?
    let mut chat_id = HashBuf::default();
    chat_id.copy_from_slice(src_id);
    chat_id
}

fn send_messages(
    stream: &mut (impl Read + Write),
    chat_id: &HashBuf,
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
    let mut buffer = [0; msg_head::SIZE + msg_out::SIZE * MAX_FETCH_AMOUNT as usize];
    let (  // this is horrible but idk how I could format it better...
        buf_pad,
        buf_chat_id,
        buf_msg_id,
        buf_count,
    ) = msg_head::split_mut((&mut buffer[..msg_head::SIZE]).try_into().unwrap());
    // ^ can't fail, perfect size

    // construct header for messages
    buf_pad.copy_from_slice(&msg_head::PAD);
    buf_chat_id.copy_from_slice(&chat_id[..1]);
    buf_msg_id.copy_from_slice(&msg_id.to_be_bytes()[1..]);
    buf_count[0] = (u8::from(forward) << 7) | count;

    // fill buffer with messages
    buffer[msg_head::SIZE..][..msgs.len()].copy_from_slice(&msgs);

    // send
    full_write(
        stream,
        &buffer[..msg_head::SIZE + count as usize * msg_out::SIZE],
        "Failed to send messages in SMRT",
    )
}

pub fn handle(mut stream: (impl Read + Write), globals: &Arc<Globals>) -> Res {
    let mut pad_buf: [u8; 3] = pad::DEFAULT;
    let mut snd_buf = msg_in::DEFAULT;  // size of msg packet
    let mut chat_id_buf = hash::DEFAULT;
    let mut qry_arg_buf = qry_arg::DEFAULT;
    let mut st_buf = msg_out::DEFAULT;

    loop {
        read_exact(
            &mut stream,
            &mut pad_buf,
            "Failed to read SMRT pad. Socket timed out?",
        )?;
        match pad_buf {
            pad::SEND_PADDING => {
                read_exact(&mut stream, &mut snd_buf, "Failed to read cypher")?;
                read_exact(&mut stream, &mut pad_buf, "Failed to read end pad (snd)")?;
                if pad_buf != pad::END_PADDING { return Err("Incorrect end padding (snd)") }
 
                chat_id_buf = packet_to_storage(&snd_buf, &mut st_buf);
                db::push(&get_chat_file(&chat_id_buf, &globals.data_dir), &st_buf)?;
            },
            pad::FETCH_PADDING => {
                // fill fetch buffer
                read_exact(&mut stream, &mut chat_id_buf, "Failed to read fetch chat id")?;
                read_exact(&mut stream, &mut pad_buf, "Failed to read end pad (fch)")?;
                if pad_buf != pad::END_PADDING { return Err("Incorrect end padding (fch)") }

                // get arguments for the db fetch
                let path = get_chat_file(&chat_id_buf, &globals.data_dir);
                // todo: add count to fetch message

                // fetch from db & send to client
                let (count, msg_id, messages) = db::fetch(&path, DEFAULT_FETCH_AMOUNT)?;
                send_messages(&mut stream, &chat_id_buf, msg_id, true, count, messages)?;
            },
            pad::QUERY_PADDING => {
                // fill chat_id and arg buffer
                // TODO: read in one go, then split with buffers?
                read_exact(&mut stream, &mut chat_id_buf, "Failed to read query chat id")?;
                read_exact(&mut stream, &mut qry_arg_buf, "Failed to read query args")?;
                read_exact(&mut stream, &mut pad_buf, "Failed to read end pad (qry)")?;
                if pad_buf != pad::END_PADDING { return Err("Incorrect end padding (qry)") }
                
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
