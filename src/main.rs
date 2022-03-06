use std::{net::TcpListener, io::Read, path::Path};

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
    let data_dir = if let Some(path) = args.last() {
        let path = Path::new(path);
        if !path.is_dir() {
            println!("Not a directory: {:?}", path);
            std::process::exit(1);
        }
        path
    } else {
        println!("No path given");
        std::process::exit(1);
    };
    println!("Using directory {:?}", data_dir.canonicalize().unwrap());

    {  // testing db
        const COUNT: usize = 100;
        let msgs: Vec<msg::Message> = (0..COUNT as u8).map(|i| {
            msg::Message {
                user_id:   [b'!' + i; HASH_SIZE],
                chat_id:   [i + 1; HASH_SIZE],
                signature: [i + 2; HASH_SIZE],
                rsa_pub:   [i + 3; RSA_SIZE],  // todo: what type is this?
                contents:  [i + 4; CONTENT_SIZE],
                time: Some(i as u128),
            }
        }).collect();
        let path = data_dir.join("test.msgs");

        let t1 = std::time::SystemTime::now();
        for msg in msgs.iter() {db::push(&path, &msg).unwrap()}

        let t2 = std::time::SystemTime::now();
        for _ in 0..COUNT {db::fetch(&path, None).unwrap();}

        let t3 = std::time::SystemTime::now();
        for i in 1..COUNT+1 {
            print!("{:3}:   ", i);
            for m in db::fetch(&path, Some(i)).unwrap() {
                print!("{}", m.user_id[0] as char);
            }
            println!();
        }

        let t4 = std::time::SystemTime::now();
        println!("Pushing 100 messages: {}μs", t2.duration_since(t1).unwrap().as_micros());
        println!("Fetching last 100 times: {}μs", t3.duration_since(t2).unwrap().as_micros());
        println!("Fetching mid 100 times: {}μs", t4.duration_since(t3).unwrap().as_micros());
    }

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

            },
            FETCH_PADDING => {  // recieved request for messages
                
            },
            QUERY_PADDING => {
                
            }
            _ => continue
        }
    }
}

