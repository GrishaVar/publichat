use std::{net::TcpListener, io::Read, path::Path, process::id};

extern crate base64;

mod db;
mod msg;
mod constants;
use constants::*;



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

fn handle_incoming(mut stream: std::net::TcpStream, data_dir: Path) {
    let mut pad_buf = [0; 4];
    stream.read(&mut pad_buf).expect("failed to read first 3 bytes!");

    match &pad_buf {
        b"GET " => todo!(), // handle_http(stream),
        b"SMRT" => handle_smrt(stream, data_dir),
        _ => return
    }
}

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

fn handle_smrt(mut stream: std::net::TcpStream, data_dir: &Path) {
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
                // fill fetch buffer
                stream.read_exact(&mut chat_id_buf).expect("failed to read fch chat id");
                stream.read_exact(&mut pad_buf).expect("failed to read end pad");  // todo: don't crash!
                
                // check "end"
                if pad_buf != END_PADDING { todo!() }  // verify end padding

                // get arguments for the db fetch
                let path = get_chat_file(&chat_id_buf, data_dir);

                let messages = db::fetch_latest(&path, DEFAULT_FETCH_AMOUNT);

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
                let messages = db::fetch(&path, id, count, forward);

                // TODO send messages back to the client with a function
            },
            _ => return,  // invalid padding  todo: respond with error
        }
    }
}

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
        const COUNT: usize = 25;  // no more than 94

        let msgs: Vec<MessageSt> = (0..COUNT as u8).map(|i| {[i+b'!'; MSG_ST_SIZE]}).collect();
        let path = data_dir.join("test.msgs");

        let t1 = std::time::SystemTime::now();
        for msg in msgs.iter() {db::push(&path, &msg).unwrap()}

        let t2 = std::time::SystemTime::now();
        for i in 0..COUNT {
            print!("Forward {:3}:   ", i);
            for m in db::fetch(&path, i as u32, 20, true).unwrap() {
                print!("{}", m[0] as char);
            }
            println!();
        }

        let t3 = std::time::SystemTime::now();
        for i in 0..COUNT {
            print!("Bckward {:3}:   ", i);
            for m in db::fetch(&path, i as u32, 20, false).unwrap() {
                print!("{}", m[0] as char);
            }
            println!();
        }

        let t4 = std::time::SystemTime::now();
        println!("Pushing {} messages: {}μs",       COUNT, t2.duration_since(t1).unwrap().as_micros());
        println!("Fetching forward {} times: {}μs", COUNT, t3.duration_since(t2).unwrap().as_micros());
        println!("Fetching bckward {} times: {}μs", COUNT, t4.duration_since(t3).unwrap().as_micros());
    }

    let listener = TcpListener::bind(IP_PORT).unwrap();

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            std::thread::spawn(move || handle_incoming(stream, data_dir));
        }
    }


    
}

