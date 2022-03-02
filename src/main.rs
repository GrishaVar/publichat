use std::{net::TcpListener, io::Read};

type Hash = (u128, u128);


const BAD_WORDS: [Hash; 4] = [
    (0, 0),
    (1, 1),
    (2, 2),
    (3, 3),
];  // todo: add bad words hashes
const IP_PORT: &str = "localhost:7878";
const PADDING_SIZE: usize = 6;
const CONTENT_SIZE: usize = 512;

const MSG_PADDING: [u8; PADDING_SIZE] = *b"start\n";
const FETCH_PADDING: [u8; PADDING_SIZE] = *b"fetch\n";
const QUERY_PADDING: [u8; PADDING_SIZE] = *b"query\n";
const END_PADDING: [u8; PADDING_SIZE] = *b"endend";

const CHAT_ID_START: usize = PADDING_SIZE + 32;
const USER_ID_START: usize = CHAT_ID_START + 32;
const SIGNATURE_START: usize = USER_ID_START + 32;
const RSA_PUB_START: usize = SIGNATURE_START + 32;
const CONTENTS_START: usize = RSA_PUB_START + 32;
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



struct Message {
    chat_id: Hash,
    user_id: Hash,
    signature: (u128, u128),
    rsa_pub: (u128, u128),  // todo: what type is this?
    contents: [u8; PACKET_SIZE],
}  // 624 bytes big (without padding)

impl Message {
    fn is_bad_word(&self) -> bool {
        BAD_WORDS.contains(&self.chat_id)
    }

    fn from_bytes(bytes: &[u8; PACKET_SIZE]) -> Option<Message> {
        // check start and end paddings
        if bytes[..PADDING_SIZE] != MSG_PADDING {return None}
        if bytes[END_PADDING_START..] != END_PADDING {return None}

        let to_hash = |pos: usize| (
            u128::from_be_bytes(bytes[pos*32..][00..16].try_into().unwrap()),
            u128::from_be_bytes(bytes[pos*32..][16..32].try_into().unwrap()),
        );

        Some(Message {
            chat_id: to_hash(0),
            user_id: to_hash(1),
            signature: to_hash(2),
            rsa_pub: to_hash(3),
            contents: bytes[CONTENTS_START..END_PADDING_START].try_into().unwrap(),
        })
    }

    fn to_bytes(&self) -> [u8; PACKET_SIZE] {
        let mut res: [u8; PACKET_SIZE] = [0; PACKET_SIZE];
        res.copy_from_slice(&MSG_PADDING);    

        let mut insert_hash = |pos: usize, hash: Hash| {
            res[pos..].copy_from_slice(&hash.0.to_be_bytes());
            res[pos+16..].copy_from_slice(&hash.1.to_be_bytes());
        };
        
        insert_hash(CHAT_ID_START, self.chat_id);
        insert_hash(USER_ID_START, self.user_id);
        insert_hash(SIGNATURE_START, self.signature);
        insert_hash(RSA_PUB_START, self.rsa_pub);
        res[CONTENTS_START..].copy_from_slice(&self.contents);

        res[END_PADDING_START..].copy_from_slice(&END_PADDING);    
        res
    }
}

fn main() {
    let mut messages: Vec<Message> = Vec::with_capacity(100);

    let listener = TcpListener::bind(IP_PORT).unwrap();
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();

        let mut buffer = [0; PACKET_SIZE];
        stream.read(&mut buffer).unwrap();

        
        match buffer[..PADDING_SIZE].try_into().unwrap() {
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
