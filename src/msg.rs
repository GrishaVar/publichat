use crate::constants::*;
use crate::Hash;

pub fn packet_to_storage(src: &[u8; NET_IN_SIZE], dest: &mut [u8; ST_SIZE]) -> Option<Hash> {
    // Takes bytes from client, 
    // Extract RSA pub, cypher, signature. Generate time.
    // Write buffer intended for storage into dest.
    // Return chat_id if successful

    if src[..PADDING_SIZE]       != MSG_PADDING {return None}
    if src[NET_IN_END_PADDING..] != END_PADDING {return None}
    
    let msg_time = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    dest[..TIME_SIZE].clone_from_slice(&u128::to_be_bytes(msg_time));
    dest[ST_RSA_START..].clone_from_slice(&src[NET_IN_RSA..NET_IN_END_PADDING]);

    // return the chat ID
    let mut chat_id = [0; HASH_SIZE];
    chat_id.clone_from_slice(&src[NET_IN_CHAT_ID..][..HASH_SIZE]);
    Some(chat_id)
}

pub fn storage_to_packet(src: &[u8; ST_SIZE], dest: &mut [u8; NET_OUT_SIZE], msg_id: u32) {
    // Takes bytes from chat file
    // adds the chat id and message id
    // Returns nothing but has side affects
    dest[..NET_OUT_TIME].clone_from_slice(&msg_id.to_be_bytes());
    dest[NET_OUT_TIME..].clone_from_slice(&src[..]);
}
