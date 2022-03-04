pub const PADDING_SIZE: usize = 6;
pub const HASH_SIZE: usize = 32;
pub const RSA_SIZE: usize = 5;  // todo: what is the correct length?
pub const CONTENT_SIZE: usize = 512;
 
pub const MSG_PADDING:   [u8; PADDING_SIZE] = *b"start\n";
pub const FETCH_PADDING: [u8; PADDING_SIZE] = *b"fetch\n";
pub const QUERY_PADDING: [u8; PADDING_SIZE] = *b"query\n";
pub const END_PADDING:   [u8; PADDING_SIZE] = *b"endend";

pub const PADDING_START: usize     = 0; 
pub const CHAT_ID_START: usize     = PADDING_SIZE + HASH_SIZE;
pub const USER_ID_START: usize     = CHAT_ID_START + HASH_SIZE;
pub const SIGNATURE_START: usize   = USER_ID_START + HASH_SIZE;
pub const RSA_PUB_START: usize     = SIGNATURE_START + HASH_SIZE;
pub const CONTENTS_START: usize    = RSA_PUB_START + HASH_SIZE;
pub const END_PADDING_START: usize = CONTENTS_START + CONTENT_SIZE;
pub const PACKET_SIZE: usize       = END_PADDING_START + PADDING_SIZE;