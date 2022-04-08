use std::{sync::Arc, io::{Read, Write}, path::Path};

use crate::{constants::*, db, msg};

fn query_bytes_to_args(data: &[u8; 4]) -> (u32, u8, bool) {
    let forward = data[0] & 0x80 != 0;  // check first bit
    let count = data[0] & 0x7f; // take the last 7 bits
    let id = u32::from_be_bytes(*data) & 0x00_ff_ff_ff;  // take the three last bytes
    (id, count, forward)
}

fn get_chat_file(chat_id: &Hash, data_dir: &Path) -> std::path::PathBuf {
    // encode hash into b64 and append to data_dir
    data_dir.join(base64::encode_config(
        chat_id,
        base64::Config::new(base64::CharacterSet::UrlSafe, false),
    ))
}

fn send_messages(stream: &mut (impl Read + Write), msgs: &Vec<MessageSt>) {
    // converts MessageSt to MessageOut and sends each into stream
    // msg::storage_to_packet
    // TcpStream::write
    let mut buffer = [0; 128*MSG_OUT_SIZE];

    for msg_id in 0..msgs.len() {
        let index= MSG_OUT_SIZE*msg_id;
        buffer[index..index+MSG_ID_SIZE].clone_from_slice(&msg_id.to_be_bytes());
        buffer[index+MSG_ID_SIZE..index+MSG_ST_SIZE].clone_from_slice(&msgs[msg_id]);
    }
    stream.write(&buffer[..msgs.len()*MSG_OUT_SIZE]).expect("failed to write buffer to steam.");
}

pub fn handle(mut stream: (impl Read + Write), data_dir: &Arc<Path>) {
    let mut pad_buf = [0; PADDING_SIZE];
    let mut snd_buf = [0; MSG_IN_SIZE];  // size of msg packet
    let mut chat_id_buf = [0; CHAT_ID_SIZE];
    let mut qry_arg_buf = [0; QUERY_ARG_SIZE];

    let mut st_buf = [0; MSG_ST_SIZE];
    loop {
        stream.read_exact(&mut pad_buf).expect("failed to read smrt padding!");
        
        match pad_buf {
            SEND_PADDING => {
                stream.read_exact(&mut snd_buf).expect("failed to read msg");
                stream.read_exact(&mut pad_buf).expect("failed to read end pad");  // todo: don't crash!
                if pad_buf != END_PADDING { todo!() }  // verify end padding
 
                chat_id_buf = msg::packet_to_storage(&snd_buf, &mut st_buf);
                db::push(&get_chat_file(&chat_id_buf, data_dir), &st_buf).expect("Failed to push to DB.");
            },
            FETCH_PADDING => {
                // fill fetch buffer
                stream.read_exact(&mut chat_id_buf).expect("failed to read fch chat id");
                stream.read_exact(&mut pad_buf).expect("failed to read end pad");  // todo: don't crash!
                // check "end"
                if pad_buf != END_PADDING { todo!() }  // verify end padding

                // get arguments for the db fetch
                let path = get_chat_file(&chat_id_buf, data_dir);

                let messages = db::fetch(&path, DEFAULT_FETCH_AMOUNT);

                // TODO send messages back to the client with a function
            },
            QUERY_PADDING => {
                // fill chat_id and arg buffer
                stream.read_exact(&mut chat_id_buf).expect("failed to read fch chat id");
                stream.read_exact(&mut qry_arg_buf).expect("failed to read fch args");

                // check "end"
                stream.read_exact(&mut pad_buf).expect("failed to read end pad");
                if pad_buf != END_PADDING { todo!() }  // verify end padding
                
                // get arguments for the db fetch
                let (id, count, forward) = query_bytes_to_args(&qry_arg_buf);
                let path = get_chat_file(&chat_id_buf, data_dir);
                
                // return query
                let messages = db::query(&path, id, count, forward);

                // TODO send messages back to the client with a function
                send_messages(&mut stream, &messages.unwrap())
            },
            _ => {
                //println!("{:?}", pad_buf.map(char::from));  // invalid padding  todo: respond with error
                println!("{:?}", pad_buf);  // invalid padding  todo: respond with error
                break;
            }
        }
    }
}
