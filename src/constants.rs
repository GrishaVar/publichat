#![allow(dead_code)]  // unused constants

/*
Get message from client:
    - PADDING                                       3
    - Chat ID                                       32
    - Cypher  // content unknown to server          440
        - Chat key (first 4 bytes)                  4
        - Client time                               8
        - Public key                                32
        - Message                                   <396
        - Padding to cypher size                    <=396
    - Signature                                     64
    - PADDING                                       3

Storage block                                       512
    - Server time                                   8
    - Cypher                                        440
    - Signature                                     64

Send message to client:
    - Server time                                   8
    - Cypher                                        440
    - Signature                                     64
*/

pub const PADDING_SIZE: usize = 3;
pub const SIGNATURE_SIZE: usize = 64;
pub const CHAT_ID_SIZE: usize = 32;
pub const CYPHER_SIZE: usize = 440;  // see very pretty diagram below (line 7)
pub const QUERY_ARG_SIZE: usize = std::mem::size_of::<u32>();
pub const TIME_SIZE: usize = std::mem::size_of::<u64>();
pub const MSG_ID_SIZE: usize = QUERY_ARG_SIZE - 1;
//pub const QUERY_DIRECTION_COUNT: usize = QUERY_ARG_SIZE - MSG_ID_SIZE;

pub const MAX_FETCH_AMOUNT: u8 = 50;
pub const DEFAULT_FETCH_AMOUNT: u8 = 50;

pub type Hash       = [u8; CHAT_ID_SIZE];
pub type Cypher     = [u8; CYPHER_SIZE];
pub type Signature  = [u8; SIGNATURE_SIZE];
pub type MsgData    = [u8; CYPHER_SIZE + SIGNATURE_SIZE];

pub type MessageIn  = [u8; MSG_IN_SIZE];
pub type MessageSt  = [u8; MSG_ST_SIZE];
pub type MessageOut = [u8; MSG_OUT_SIZE];

// Sizes of incoming message
pub const MSG_IN_CHAT_ID: usize         = 0;
pub const MSG_IN_CYPHER: usize          = MSG_IN_CHAT_ID + CHAT_ID_SIZE;
pub const MSG_IN_SIGNATURE: usize       = MSG_IN_CYPHER + CYPHER_SIZE;
pub const MSG_IN_SIZE: usize            = MSG_IN_SIGNATURE + SIGNATURE_SIZE;

// Sizes of storage blocks
pub const MSG_ST_TIME_START: usize      = 0;
pub const MSG_ST_CYPHER_START: usize    = MSG_ST_TIME_START + TIME_SIZE;
pub const MSG_ST_SIGNATURE: usize       = MSG_ST_CYPHER_START + CYPHER_SIZE;
pub const MSG_ST_SIZE: usize            = MSG_ST_SIGNATURE + SIGNATURE_SIZE;

// Sizes of outgoing header of multi-msg packet
pub const HED_OUT_PAD: usize            = 0;
pub const HED_OUT_CHAT_ID_BYTE: usize   = HED_OUT_PAD + PADDING_SIZE;
pub const HED_OUT_MSG_ID: usize         = HED_OUT_CHAT_ID_BYTE + 1;  // only 1st
pub const HED_OUT_MSG_COUNT: usize      = HED_OUT_MSG_ID + MSG_ID_SIZE;
pub const HED_OUT_SIZE: usize           = HED_OUT_MSG_COUNT + 1;  // max 127

// Sizes of an outgoing message
pub const MSG_OUT_TIME: usize           = 0;
pub const MSG_OUT_CYPHER: usize         = MSG_OUT_TIME + TIME_SIZE;
pub const MSG_OUT_SIG: usize            = MSG_OUT_CYPHER + CYPHER_SIZE;
pub const MSG_OUT_SIZE: usize           = MSG_OUT_SIG + SIGNATURE_SIZE;


// NETWORK PACKETS:

// Network paddings (to server)
pub const SEND_PADDING:  [u8; PADDING_SIZE] = *b"snd";
pub const FETCH_PADDING: [u8; PADDING_SIZE] = *b"fch";
pub const QUERY_PADDING: [u8; PADDING_SIZE] = *b"qry";
pub const END_PADDING:   [u8; PADDING_SIZE] = *b"end";

// Network paddings (to client)
pub const MSG_PADDING:   [u8; PADDING_SIZE] = *b"msg";

// Sizes of incoming fetch packets
pub const FCH_CHAT_ID: usize        = 0;
pub const FCH_SIZE: usize           = FCH_CHAT_ID + CHAT_ID_SIZE;

// Sizes of incoming query packets
pub const QRY_CHAT_ID: usize        = 0;
pub const QRY_ARGS: usize           = QRY_CHAT_ID + CHAT_ID_SIZE;  // direction and amount
pub const QRY_MSG_ID: usize         = QRY_ARGS + 1;  // 3 bytes msg ID (ARGS is 1 byte)
pub const QRY_SIZE: usize           = QRY_MSG_ID;
