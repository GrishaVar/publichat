use std::{net::TcpListener, io::Read, path::Path, sync::mpsc, thread};
use rusqlite::Connection;

mod db;
mod msg;
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


pub enum Query {
    Fetch {chat_id: Hash},
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (db_reader, db_writer) = if let Some(path) = args.last() {
        if Path::new(path).is_file() {
            println!("Opening file {}...", path);
            (  // todo: better way of opening two connections?
                Connection::open(path)
                    .unwrap_or_else(|_| {
                        println!("Failed to open file {}.", path);
                        std::process::exit(1);
                    }),
                Connection::open(path).unwrap()
            )  // todo: assert the db is valid
        } else {
            println!("File {} not found; creating...", path);
            (db::create(Some(path)), Connection::open(path).unwrap())
        }
    } else {
        println!("No file path given; creating new DB in RAM");
        (db::create( None), db::create( None))  // two seperate DBs?
    };

    let (sender, reciever) = mpsc::channel::<msg::Message>();
    thread::spawn(move || while let Ok(msg) = reciever.recv() {
        db::add_msg(&db_writer, msg);
    });

    {  // testing
        sender.send(msg::Message {
            chat_id:   [48; HASH_SIZE],
            user_id:   [0; HASH_SIZE],
            signature: [49; HASH_SIZE],
            rsa_pub:   [48; RSA_SIZE],
            contents:  [48; CONTENT_SIZE],
            time: None,
        }).ok();
        sender.send(msg::Message {
            chat_id:   [48; HASH_SIZE],
            user_id:   [11; HASH_SIZE],
            signature: [49; HASH_SIZE],
            rsa_pub:   [48; RSA_SIZE],
            contents:  [48; CONTENT_SIZE],
            time: None,
        }).ok();
        sender.send(msg::Message {
            chat_id:   [48; HASH_SIZE],
            user_id:   [88; HASH_SIZE],
            signature: [49; HASH_SIZE],
            rsa_pub:   [48; RSA_SIZE],
            contents:  [48; CONTENT_SIZE],
            time: None,
        }).ok();
        thread::sleep_ms(2);
        for m in db::fetch(&db_reader, [48; HASH_SIZE]) {
            println!("line: {:?}", m.user_id);
        }
    }

    let mut messages: Vec<msg::Message> = Vec::with_capacity(100);

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
                if let Some(msg) = msg::Message::from_bytes(&buffer) {
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

