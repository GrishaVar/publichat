use std::{sync::Arc, net::TcpStream, io::Read, path::Path};

use crate::{constants::*, db, msg};

fn query_bytes_to_args(data: &[u8; 4]) -> (u32, u8, bool) {
    let forward = data[0] & 0x80 != 0;  // check first bit
    let count = data[0] & 0x7f; // take the last 7 bits
    let id = u32::from_be_bytes(*data) & 0x00_ff_ff_ff;  // take the three last bytes
    (id, count, forward)
}

fn get_chat_file(chat_id: &Hash, data_dir: &Path) -> std::path::PathBuf {
    // encode hash into b64 and append to data_dir
    data_dir.join(base64::encode(chat_id))
}


pub fn handle(mut stream: TcpStream, data_dir: &Arc<Path>) {
    let mut pad_buf = [0; PADDING_SIZE];
    let mut snd_buf = [0; MSG_IN_SIZE];  // size of msg packet
    let mut chat_id_buf = [0; CHAT_ID_SIZE];
    let mut qry_arg_buf = [0; QUERY_ARG_SIZE];

    let mut st_buf = [0; MSG_ST_SIZE];
    loop {
        stream.read_exact(&mut pad_buf).expect("failed to smrt padding!");
        
        match pad_buf {
            SEND_PADDING => {
                stream.read_exact(&mut snd_buf).expect("failed to read msg");
                stream.read_exact(&mut pad_buf).expect("failed to read end pad");  // todo: don't crash!
                if pad_buf != END_PADDING { todo!() }  // verify end padding
 
                chat_id_buf = msg::packet_to_storage(&snd_buf, &mut st_buf);
                db::push(&get_chat_file(&chat_id_buf, data_dir), &st_buf);
            },
            FETCH_PADDING => {
                stream.read_exact(&mut chat_id_buf).expect("failed to read fch");
                stream.read_exact(&mut pad_buf).expect("failed to read end pad");


                
                // fill fetch buffer
                // check "end"
                // return fetch
                //fetch_latest(path: &PathBuf, count: u8)

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
            },
            _ => return,  // invalid padding  todo: respond with error
        }
    }
}