#![allow(dead_code)]  // unused constants

pub const PADDING_SIZE: usize = 3;
pub const HASH_SIZE: usize = 32;
pub const SIG_SIZE: usize = 0;  // HASH_SIZE;
pub const CHAT_ID_SIZE: usize = HASH_SIZE;  // HASH_SIZE;
pub const RSA_SIZE: usize = HASH_SIZE;  // 5;  // todo: what is the correct length?
pub const CYPHER_SIZE: usize = 128;
pub const QUERY_ARG_SIZE: usize = std::mem::size_of::<u32>();
pub const TIME_SIZE: usize = std::mem::size_of::<u64>();
pub const MSG_ID_SIZE: usize = std::mem::size_of::<u32>();

pub const MAX_FETCH_AMOUNT: u8 = 50;
pub const DEFAULT_FETCH_AMOUNT: u8 = 50;

pub type Hash       = [u8; HASH_SIZE];
pub type Rsa        = [u8; RSA_SIZE];
pub type Contents   = [u8; CYPHER_SIZE];

pub type MessageIn  = [u8; MSG_IN_SIZE];
pub type MessageSt  = [u8; MSG_ST_SIZE];
pub type MessageOut = [u8; MSG_OUT_SIZE];

// Sizes of incoming message
pub const MSG_IN_CHAT_ID: usize         = 0;
pub const MSG_IN_RSA: usize             = MSG_IN_CHAT_ID + CHAT_ID_SIZE;
pub const MSG_IN_CYPHER: usize          = MSG_IN_RSA + HASH_SIZE;
pub const MSG_IN_SIGNATURE: usize       = MSG_IN_CYPHER + CYPHER_SIZE;
pub const MSG_IN_SIZE: usize            = MSG_IN_SIGNATURE + SIG_SIZE;

// Sizes of storage blocks
pub const MSG_ST_TIME_START: usize      = 0;
pub const MSG_ST_RSA_START: usize       = MSG_ST_TIME_START + TIME_SIZE;
pub const MSG_ST_CYPHER_START: usize    = MSG_ST_RSA_START + RSA_SIZE;
pub const MSG_ST_SIGNATURE: usize       = MSG_ST_CYPHER_START + CYPHER_SIZE;
pub const MSG_ST_SIZE: usize            = MSG_ST_SIGNATURE + SIG_SIZE;

// Sizes of outgoing network packets
pub const MSG_OUT_ID: usize             = 0;
pub const MSG_OUT_TIME: usize           = MSG_OUT_ID + MSG_ID_SIZE;
pub const MSG_OUT_RSA: usize            = MSG_OUT_TIME + TIME_SIZE;
pub const MSG_OUT_CYPHER: usize         = MSG_OUT_RSA + RSA_SIZE;
pub const MSG_OUT_SIG: usize            = MSG_OUT_CYPHER + CYPHER_SIZE;
pub const MSG_OUT_SIZE: usize           = MSG_OUT_SIG + SIG_SIZE;

/*
Get message from client:
    > Chat ID
    - RSA pub
    - Message cypher
    - Signature

Storage block
    - Time
    - RSA pub
    - Message cypher
    - Signature

Send message to client:
    > Message ID
    - Time
    - RSA pub
    - Message cypher
    - Signature
*/


// NETWORK PACKETS:

// Network paddings
pub const SEND_PADDING:  [u8; PADDING_SIZE] = *b"snd";
pub const FETCH_PADDING: [u8; PADDING_SIZE] = *b"fch";
pub const QUERY_PADDING: [u8; PADDING_SIZE] = *b"qry";
pub const END_PADDING:   [u8; PADDING_SIZE] = *b"end";

// Sizes of incomming fetch packets
pub const FCH_CHAT_ID: usize        = 0;
pub const FCH_SIZE: usize           = FCH_CHAT_ID + CHAT_ID_SIZE;

// Sizes of incomming query packets
pub const QRY_CHAT_ID: usize        = 0;
pub const QRY_ARGS: usize           = QRY_CHAT_ID + CHAT_ID_SIZE;  // direction and amount
pub const QRY_MSG_ID: usize         = QRY_ARGS + 1;  // 3 bytes msg ID (ARGS is 1 byte)
pub const QRY_SIZE: usize           = QRY_MSG_ID;

/*
// Sizes of incomming fetch packets
pub const FCH_PADDING: usize        = 0;
pub const FCH_CHAT_ID: usize        = FCH_PADDING + PADDING_SIZE;
pub const FCH_END_PADDING: usize    = FCH_CHAT_ID + HASH_SIZE;
pub const FCH_SIZE: usize           = FCH_END_PADDING + PADDING_SIZE;

// Sizes of incomming query packets
pub const QRY_PADDING: usize        = 0;
pub const QRY_CHAT_ID: usize        = QRY_PADDING + PADDING_SIZE;
pub const QRY_QUERY_ARG: usize      = QRY_CHAT_ID + HASH_SIZE;
pub const QRY_END_PADDING: usize    = QRY_QUERY_ARG + QUERY_SIZE;
pub const QRY_SIZE: usize           = QRY_END_PADDING + PADDING_SIZE;

// Size of message packet
pub const SND_PADDING: usize        = 0;
pub const SND_MESSAGE: usize        = SND_PADDING + PADDING_SIZE;
pub const SND_END_PADDING: usize    = SND_MESSAGE + MSG_IN_SIZE;
pub const SND_SIZE: usize           = SND_END_PADDING + PADDING_SIZE;
*/
