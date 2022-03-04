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
    pub time: Option<[u8; 16]>,
}  // 624 bytes big (without padding)

impl Message {
    fn is_bad_word(&self) -> bool {
        BAD_WORDS.contains(&self.chat_id)
    }

    pub fn from_bytes(bytes: &[u8; PACKET_SIZE]) -> Option<Message> {
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