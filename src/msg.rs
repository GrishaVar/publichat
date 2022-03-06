use crate::constants::*;
use crate::{Hash, RSA, Contents};

const BAD_WORDS: [Hash; 4] = [
    [0; HASH_SIZE],
    [1; HASH_SIZE],
    [2; HASH_SIZE],
    [3; HASH_SIZE],
];  // todo: add bad words hashes

pub struct Message {
    pub chat_id: Hash,
    pub user_id: Hash,
    pub signature: Hash,
    pub rsa_pub: RSA,  // todo: what type is this?
    pub contents: Contents,
    pub time: Option<u128>,
}  // 624 bytes big (without padding)

impl Message {
    fn is_bad_word(&self) -> bool {
        BAD_WORDS.contains(&self.chat_id)
    }

    pub fn from_packet(bytes: &[u8; PACKET_SIZE]) -> Option<Message> {
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

    pub fn to_packet(&self) -> [u8; PACKET_SIZE] {
        let mut res: [u8; PACKET_SIZE] = [0; PACKET_SIZE];

        res[PADDING_START..].copy_from_slice(&MSG_PADDING);
        res[CHAT_ID_START..].copy_from_slice(&self.chat_id);
        res[USER_ID_START..].copy_from_slice(&self.user_id);
        res[SIGNATURE_START..].copy_from_slice(&self.signature);
        res[RSA_PUB_START..].copy_from_slice(&self.rsa_pub);
        res[CONTENTS_START..].copy_from_slice(&self.contents);
        res[END_PADDING_START..].copy_from_slice(&END_PADDING);    
        res
    }

    pub fn from_storage(bytes: &[u8; ST_SIZE]) -> Option<Message> {
        Some(Message {
            time:      Some(u128::from_be_bytes(bytes[ST_TIME_START..][..TIME_SIZE].try_into().ok()?)),
            chat_id:   [0; HASH_SIZE],
            user_id:   bytes[ST_USER_START..][..HASH_SIZE].try_into().ok()?,
            contents:  bytes[ST_CONT_START..][..CONTENT_SIZE].try_into().ok()?,
            signature: [0; HASH_SIZE],
            rsa_pub:   [0; RSA_SIZE],
        })
    }

    pub fn to_storage(&self) -> [u8; ST_SIZE] {
        let mut res: [u8; ST_SIZE] = [0; ST_SIZE];
        res[ST_TIME_START..][..TIME_SIZE].copy_from_slice(&self.time.unwrap().to_be_bytes());  // todo: possible error?
        res[ST_USER_START..][..HASH_SIZE].copy_from_slice(&self.user_id);
        res[ST_CONT_START..][..CONTENT_SIZE].copy_from_slice(&self.contents);
        res
    }
}
