use std::net::TcpStream;

use publichat::helpers::*;
use publichat::constants::*;

pub fn send_msg(  // TODO: create global "smrt" crate with en/decoding functions
    stream: &mut TcpStream,
    chat: &Hash,
    cypher: &Cypher,
    signature: &Signature,
) -> Res {
    // TODO: rename constants to something direction-agnostic
    let mut buf = [0; PADDING_SIZE + MSG_IN_SIZE + PADDING_SIZE];

    buf[..PADDING_SIZE].copy_from_slice(&SEND_PADDING);  // TODO: make padding const?
    buf[PADDING_SIZE+MSG_IN_CHAT_ID..][..CHAT_ID_SIZE].copy_from_slice(chat);
    buf[PADDING_SIZE+MSG_IN_CYPHER..][..CYPHER_SIZE].copy_from_slice(cypher);
    buf[PADDING_SIZE+MSG_IN_SIGNATURE..][..SIGNATURE_SIZE].copy_from_slice(signature);
    buf[PADDING_SIZE+MSG_IN_SIZE..][..PADDING_SIZE].copy_from_slice(&END_PADDING);

    full_write(stream, &buf, "Failed to send message")
}

pub fn send_fetch(stream: &mut TcpStream, chat: &Hash) -> Res {
    let mut buf = [0; PADDING_SIZE + FCH_SIZE + PADDING_SIZE];
    buf[..PADDING_SIZE].copy_from_slice(&FETCH_PADDING);
    buf[PADDING_SIZE+FCH_CHAT_ID..][..CHAT_ID_SIZE].copy_from_slice(chat);
    buf[PADDING_SIZE+FCH_SIZE..][..PADDING_SIZE].copy_from_slice(&END_PADDING);

    full_write(stream, &buf, "Failed to send fetch")
}

pub fn send_query(
    stream: &mut TcpStream,
    chat: &Hash,
    forwards: bool,
    count: u8,
    id: u32,
) -> Res {
    if count > 0x7f || id > 0xffffff { return Err("Query input too large") }

    let mut buf = [0; PADDING_SIZE + QRY_SIZE + PADDING_SIZE];
    buf[..PADDING_SIZE].copy_from_slice(&QUERY_PADDING);
    buf[PADDING_SIZE+QRY_CHAT_ID..][..CHAT_ID_SIZE].copy_from_slice(chat);
    buf[PADDING_SIZE+QRY_ARGS] = if forwards {count | 0x80} else {count};
    buf[PADDING_SIZE+QRY_MSG_ID..][..MSG_ID_SIZE].copy_from_slice(&id.to_be_bytes()[1..]);
    buf[PADDING_SIZE+QRY_SIZE..][..PADDING_SIZE].copy_from_slice(&END_PADDING);

    full_write(stream, &buf, "Failed to send query")
}
