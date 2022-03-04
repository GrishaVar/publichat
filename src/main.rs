use std::{net::TcpListener, io::Read, path::Path};
use rusqlite::{Connection, OpenFlags};

mod db;
mod constants;
use constants::*;

pub type Hash = [u8; HASH_SIZE];
pub type RSA = [u8; RSA_SIZE];
pub type Contents = [u8; CONTENT_SIZE];

const IP_PORT: &str = "localhost:7878";

/* Single "send message" packet structure

"start\n": 6, [115, 116, 97, 114, 116, 10];
chat_id: 32,
user_id: 32,
signature: 32,
rsa_pub: 32,
contents: 512,
"endend": 6, [101, 110, 100, 101, 110, 100]

Total: 652
*/


/* Fetch 100 latest messages in chat

"fetch\n": 6, [102, 101, 116, 99, 104, 10];
chat_id: 32,
"endend": 6, [101, 110, 100, 101, 110, 100]

Total: 44
*/


/* Advanced fetching of messages

"query\n": 6,
chat_id: 32,
user_id: 32,
time: 8,  messages starting from this time
time_reverse: 8,  messages ending from this time
num_messages: 4,
"endend": 6, [101, 110, 100, 101, 110, 100]

Total: 96
*/


const BAD_WORDS: [Hash; 4] = [
    [0; HASH_SIZE],
    [1; HASH_SIZE],
    [2; HASH_SIZE],
    [3; HASH_SIZE],
];  // todo: add bad words hashes

pub struct Message {
    chat_id: Hash,
    user_id: Hash,
    signature: Hash,
    rsa_pub: RSA,  // todo: what type is this?
    contents: Contents,
    time: Option<[u8; 16]>,
}  // 624 bytes big (without padding)

impl Message {
    fn is_bad_word(&self) -> bool {
        BAD_WORDS.contains(&self.chat_id)
    }

    fn from_bytes(bytes: &[u8; PACKET_SIZE]) -> Option<Message> {
        // check start and end paddings
        if bytes[..PADDING_SIZE]      != MSG_PADDING {return None}
        if bytes[END_PADDING_START..] != END_PADDING {return None}

        Some(Message {
            chat_id:   bytes[CHAT_ID_START..][..HASH_SIZE].try_into().ok()?,
            user_id:   bytes[USER_ID_START..][..HASH_SIZE].try_into().ok()?,
            signature: bytes[SIGNATURE_START..][..HASH_SIZE].try_into().ok()?,
            rsa_pub:   bytes[RSA_PUB_START..][..RSA_SIZE].try_into().ok()?,
            contents:  bytes[CONTENTS_START..][..CONTENT_SIZE].try_into().ok()?,
            time:      None,
        })
    }

    fn to_bytes(&self) -> [u8; PACKET_SIZE] {
        let mut res: [u8; PACKET_SIZE] = [0; PACKET_SIZE];

        res.copy_from_slice(&MSG_PADDING);
        res[CHAT_ID_START..].copy_from_slice(&self.chat_id);
        res[USER_ID_START..].copy_from_slice(&self.user_id);
        res[SIGNATURE_START..].copy_from_slice(&self.signature);
        res[RSA_PUB_START..].copy_from_slice(&self.rsa_pub);
        res[CONTENTS_START..].copy_from_slice(&self.contents);
        res[END_PADDING_START..].copy_from_slice(&END_PADDING);    
        res
    }
}

pub enum Query {
    Fetch {chat_id: Hash},
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db_conn = if let Some(path) = args.last() {
        if Path::new(path).is_file() {
            println!("Opening file {}...", path);
            Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)
                .unwrap_or_else(|_| {
                    println!("Failed to open file {}.", path);
                    std::process::exit(1);
                })  // todo: assert the db is valid
        } else {
            println!("File {} not found; creating...", path);
            db::create(Some(path))
        }
    } else {
        println!("No file path given; creating new DB in RAM");
        db::create( None)
    };

    {  // testing db
        db::add_msg(&db_conn, Message {
            chat_id:   [48; HASH_SIZE],
            user_id:   [2; HASH_SIZE],
            signature: [49; HASH_SIZE],
            rsa_pub:   [48; RSA_SIZE],
            contents:  [48; CONTENT_SIZE],
            time: None,
        });

        for m in db::fetch(&db_conn, [48; HASH_SIZE]) {
            println!("line: {:?}", m.user_id);
        }
    }

    let mut messages: Vec<Message> = Vec::with_capacity(100);

    let listener = TcpListener::bind(IP_PORT).unwrap();

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        // make new thread
        

        let mut buffer = [0; PACKET_SIZE];  // instead of buffer, iterate over data
        stream.read(&mut buffer).unwrap();

        match buffer[..PADDING_SIZE].try_into().unwrap() {
            // match incoming data against one of these IN A LOOP:
            /* Possible incoming requests:
                1) HTTP
                    1.1) HTTP GET (serve webpage). Close socket after this is done. (404, robots.txt, etc.)
                    1.2) HTTP upgrade (Websocket). TODO: what if you do HTTP after upgrade?
                2) "fetch\n" (get 50 latest from DB with chat_id)
                3) "query\n" (arbitrary SQL query from DB) (must include chat)
                4) "start\n" Send message (add to DB with time)
            */
            // "HTTP GET" => {
            //     socket.send(http_parser::parse(buffer));
            // },
            // "HTTP UPGRADE" => {
            //     socket.send(http_parser::parse(buffer));
            // },
            MSG_PADDING => {  // recieved a new message
                if let Some(msg) = Message::from_bytes(&buffer) {
                    messages.push(msg)
                }
            },
            FETCH_PADDING => {  // recieved request for messages
                
            },
            QUERY_PADDING => {
                
            }
            _ => continue
        }
    }
}

