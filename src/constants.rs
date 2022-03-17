#![allow(dead_code)]  // unused constants

pub const PADDING_SIZE: usize = 6;
pub const HASH_SIZE: usize = 32;
pub const SIG_SIZE: usize = HASH_SIZE;
pub const RSA_SIZE: usize = 5;  // todo: what is the correct length?
pub const CYPHER_SIZE: usize = 128;
pub const TIME_SIZE: usize = std::mem::size_of::<u128>();
pub const MSG_ID_SIZE: usize = std::mem::size_of::<u32>();
 
pub type Hash       = [u8; HASH_SIZE];
pub type RSA        = [u8; RSA_SIZE];
pub type Contents   = [u8; CYPHER_SIZE];

pub type MessageIn  = [u8; NET_IN_SIZE];
pub type MessageSt  = [u8; ST_SIZE];
pub type MessageOut = [u8; NET_OUT_SIZE];

pub const MSG_PADDING:   [u8; PADDING_SIZE] = *b"start\n";
pub const FETCH_PADDING: [u8; PADDING_SIZE] = *b"fetch\n";
pub const QUERY_PADDING: [u8; PADDING_SIZE] = *b"query\n";
pub const END_PADDING:   [u8; PADDING_SIZE] = *b"endend";

// Sizes of network packets
pub const NET_IN_PADDING: usize     = 0;
pub const NET_IN_CHAT_ID: usize     = NET_IN_PADDING + PADDING_SIZE;
pub const NET_IN_RSA: usize         = NET_IN_CHAT_ID + HASH_SIZE;
pub const NET_IN_CYPHER: usize      = NET_IN_RSA + HASH_SIZE;
pub const NET_IN_SIGNATURE: usize   = NET_IN_CYPHER + CYPHER_SIZE;
pub const NET_IN_END_PADDING: usize = NET_IN_SIGNATURE + SIG_SIZE;
pub const NET_IN_SIZE: usize        = NET_IN_END_PADDING + PADDING_SIZE;

// Sizes of storage blocks
pub const ST_TIME_START: usize      = 0;
pub const ST_RSA_START: usize       = ST_TIME_START + TIME_SIZE;
pub const ST_CYPHER_START: usize    = ST_RSA_START + RSA_SIZE;
pub const ST_SIGNATURE: usize       = ST_CYPHER_START + CYPHER_SIZE;
pub const ST_SIZE: usize            = ST_SIGNATURE + SIG_SIZE;

// Sizes of outgoing network packets
pub const NET_OUT_ID: usize         = 0;
pub const NET_OUT_TIME: usize       = NET_OUT_ID + MSG_ID_SIZE;
pub const NET_OUT_RSA: usize        = NET_OUT_TIME + TIME_SIZE;
pub const NET_OUT_CYPHER: usize     = NET_OUT_RSA + RSA_SIZE;
pub const NET_OUT_SIG: usize        = NET_OUT_CYPHER + CYPHER_SIZE;
pub const NET_OUT_SIZE: usize       = NET_OUT_SIG + HASH_SIZE;

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
