use std::net::TcpStream;

use publichat::helpers::*;
use publichat::buffers::{
    cypher::Buf as CypherBuf,
    hash::Buf as HashBuf,
    msg_in_c as msg_in,
    fetch,
    query,
};

use crate::crypt::ed25519::SigBuf;

pub fn send_msg(
    stream: &mut TcpStream,
    chat: &HashBuf,
    cypher: &CypherBuf,
    signature: &SigBuf,
) -> Res {
    // TODO: rename constants to something direction-agnostic
    // TODO: create global buffer builder functions
    let mut buf = msg_in::PREPAD;
    let (cid_buf, cy_buf, sig_buf) = msg_in::pad_split_mut(&mut buf);

    cid_buf.copy_from_slice(chat);
    cy_buf.copy_from_slice(cypher);
    sig_buf.copy_from_slice(signature);

    full_write(stream, &buf, "Failed to send message")
}

pub fn send_fetch(stream: &mut TcpStream, chat: &HashBuf) -> Res {
    let mut buf = fetch::PREPAD;
    let (cid_buf,) = fetch::pad_split_mut(&mut buf);

    cid_buf.copy_from_slice(chat);

    full_write(stream, &buf, "Failed to send fetch")
}

pub fn send_query(
    stream: &mut TcpStream,
    chat: &HashBuf,
    forwards: bool,
    count: u8,
    id: u32,
) -> Res {
    if count > 0x7f || id > 0xffffff { return Err("Query input too large") }
    let mut buf = query::PREPAD;
    let (cid_buf, args_buf, mid_buf) = query::pad_split_mut(&mut buf);

    cid_buf.copy_from_slice(chat);
    args_buf[0] = if forwards {count | 0x80} else {count};
    mid_buf.copy_from_slice(&id.to_be_bytes()[1..]);

    full_write(stream, &buf, "Failed to send query")
}
