use std::{net::TcpListener, io::Read, path::Path};

type Hash = [u8; 32];
type RSA = [u8; 5];  // todo: what is the correct length?
use rusqlite::{Connection, OpenFlags};


const IP_PORT: &str = "localhost:7878";
const PADDING_SIZE: usize = 6;
const CONTENT_SIZE: usize = 512;
const HASH_SIZE: usize = std::mem::size_of::<Hash>();
const RSA_SIZE: usize = std::mem::size_of::<RSA>();

const MSG_PADDING: [u8; PADDING_SIZE] = *b"start\n";
const FETCH_PADDING: [u8; PADDING_SIZE] = *b"fetch\n";
const QUERY_PADDING: [u8; PADDING_SIZE] = *b"query\n";
const END_PADDING: [u8; PADDING_SIZE] = *b"endend";

const CHAT_ID_START: usize = PADDING_SIZE + HASH_SIZE;
const USER_ID_START: usize = CHAT_ID_START + HASH_SIZE;
const SIGNATURE_START: usize = USER_ID_START + HASH_SIZE;
const RSA_PUB_START: usize = SIGNATURE_START + HASH_SIZE;
const CONTENTS_START: usize = RSA_PUB_START + HASH_SIZE;
const END_PADDING_START: usize = CONTENTS_START + CONTENT_SIZE;
const PACKET_SIZE: usize = END_PADDING_START + PADDING_SIZE;

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

struct Message {
    chat_id: Hash,
    user_id: Hash,
    signature: Hash,
    rsa_pub: RSA,  // todo: what type is this?
    contents: [u8; CONTENT_SIZE],
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

enum Query {
    Fetch {chat_id: Hash},
}

fn db_create(path: Option<&str>) -> Connection {
    let conn = if let Some(path) = path {
        Connection::open(path).expect("Failed to open path")
    } else {
        Connection::open_in_memory().expect("Failed to make new db")
    };
    conn.execute_batch("\
        BEGIN; \
        CREATE TABLE Messages ( \
            chat      blob(64)  NOT NULL, \
            user      blob(64)  NOT NULL, \
            time      blob(16)  NOT NULL, \
            rsa_pub   blob(64)  NOT NULL, \
            signature blob(63)  NOT NULL, \
            message   blob(512) NOT NULL \
        ); \
        COMMIT; \
    ").expect("Failed to create table");
    conn
}

fn db_add_msg(conn: &Connection, msg: Message) {
    let mut stmt = conn.prepare_cached(
        "INSERT INTO Messages (chat, user, time, rsa_pub, signature, message) \
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    ).expect("Failed to make cached add_msg query");
    stmt.execute(&[
        &msg.chat_id,
        &msg.user_id,
        &std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).expect("Time travelers!")
            .as_nanos().to_be_bytes()[..16],
        &msg.rsa_pub,
        &msg.signature,
        &msg.contents,
    ]).expect("Failed to add message");
}

fn db_fetch(conn: &Connection, chat_id: Hash) -> Vec<Message> {  // todo: consider returning iterator somehow?
    let mut res = Vec::with_capacity(50);
    let mut stmt = conn.prepare_cached(
        "SELECT * FROM Messages WHERE chat = ? \
        ORDER BY rowid DESC LIMIT 50"
    ).expect("Failed to make cached fetch query");
    let msgs = stmt.query_map([&chat_id], |row| Ok(Message {
        chat_id: row.get(0).unwrap(),
        user_id: row.get(1).unwrap(),
        time: row.get(2).unwrap(),
        rsa_pub: row.get(3).unwrap(),
        signature: row.get(4).unwrap(),
        contents: row.get(5).unwrap(),
    })).expect("Failed to convert");
    res.extend(msgs.map(|msg| msg.unwrap()));
    println!("debug: {:?}", res.len());
    res  // returns newest message first!!!
}

fn db_query(conn: &Connection, query: Query) -> Vec<Message> {
    todo!();
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
            db_create(Some(path))
        }
    } else {
        println!("No file path given; creating new DB in RAM");
        db_create( None)
    };

    db_add_msg(&db_conn, Message {  // testing
        chat_id:   [48; HASH_SIZE],
        user_id:   [2; HASH_SIZE],
        signature: [49; HASH_SIZE],
        rsa_pub:   [48; RSA_SIZE],
        contents:  [48; CONTENT_SIZE],
        time: None,
    });

    for m in db_fetch(&db_conn, [48; HASH_SIZE]) {
        println!("line: {:?}", m.user_id);
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

