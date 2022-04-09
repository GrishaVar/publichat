use crate::constants::*;

pub fn packet_to_storage(src: &MessageIn, dest: &mut MessageSt) -> Hash {
    // Takes bytes from client, 
    // Extract RSA pub, cypher, signature. Generate time.
    // Write buffer intended for storage into dest.
    // Return chat_id if successful
    
    let msg_time = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap()
        .as_millis();
    dest[..TIME_SIZE].clone_from_slice(&u128::to_be_bytes(msg_time));
    dest[MSG_ST_RSA_START..].clone_from_slice(&src[MSG_IN_RSA..]);

    // return the chat ID
    let mut chat_id = [0; HASH_SIZE];
    chat_id.clone_from_slice(&src[MSG_IN_CHAT_ID..][..HASH_SIZE]);
    chat_id
}

pub fn storage_to_packet(src: &MessageSt, dest: &mut [u8], msg_id: u32) {
    // Takes bytes from chat file
    // adds the chat id and message id
    // Returns nothing but has side affects
    dest[..MSG_OUT_TIME].clone_from_slice(&msg_id.to_be_bytes());
    dest[MSG_OUT_TIME..].clone_from_slice(&src[..]);
}
