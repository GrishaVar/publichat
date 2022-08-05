use std::net::TcpStream;

use publichat::helpers::*;
use publichat::constants::*;

use crate::crypt::ed25519;

const fn pad_buf<const L: usize>(pad: [u8; PADDING_SIZE]) -> [u8; L] {
    // Create an empty buffer, put pad in the beginning
    // and END_PADDING at the end. This function is const!
    // TODO: move this somewhere general? Do the same for server after tui merge
    let mut buf = [0; L];
    let mut i = 0;
    while { i += 1; i <= PADDING_SIZE } {  // for-loops aren't const :/
        let j = i - 1;  // TODO: find a way to avoid this?
        buf[j] = pad[j];
        buf[buf.len() - PADDING_SIZE + j] = END_PADDING[j];
    }
    buf
}

const EMPTY_MSG_BUF: [u8; MSG_IN_SIZE + 2*PADDING_SIZE] = pad_buf(SEND_PADDING);
const EMPTY_FCH_BUF: [u8; FCH_SIZE + 2*PADDING_SIZE] = pad_buf(FETCH_PADDING);
const EMPTY_QRY_BUF: [u8; QRY_SIZE + 2*PADDING_SIZE] = pad_buf(QUERY_PADDING);

pub fn send_msg(  // TODO: create global "smrt" crate with en/decoding functions
    stream: &mut TcpStream,
    chat: &Hash,
    cypher: &Cypher,
    signature: &ed25519::SigBuffer,
) -> Res {
    // TODO: rename constants to something direction-agnostic
    let mut buf = EMPTY_MSG_BUF;

    let cont = &mut buf[PADDING_SIZE..][..MSG_IN_SIZE];
    cont[MSG_IN_CHAT_ID..][..CHAT_ID_SIZE].copy_from_slice(chat);
    cont[MSG_IN_CYPHER..][..CYPHER_SIZE].copy_from_slice(cypher);
    cont[MSG_IN_SIGNATURE..][..SIGNATURE_SIZE].copy_from_slice(signature);

    full_write(stream, &buf, "Failed to send message")
}

pub fn send_fetch(stream: &mut TcpStream, chat: &Hash) -> Res {
    let mut buf = EMPTY_FCH_BUF;

    let cont = &mut buf[PADDING_SIZE..][..FCH_SIZE];
    cont[FCH_CHAT_ID..][..CHAT_ID_SIZE].copy_from_slice(chat);

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
    let mut buf = EMPTY_QRY_BUF;

    let cont = &mut buf[PADDING_SIZE..][..QRY_SIZE];
    cont[QRY_CHAT_ID..][..CHAT_ID_SIZE].copy_from_slice(chat);
    cont[QRY_ARGS] = if forwards {count | 0x80} else {count};
    cont[QRY_MSG_ID..][..MSG_ID_SIZE].copy_from_slice(&id.to_be_bytes()[1..]);

    full_write(stream, &buf, "Failed to send query")
}
