use std::net::TcpStream;

use publichat::helpers::*;
use publichat::buffers::{
    Cypher,
    MsgIn,
    Fetch,
    Query,
    Hash,
};

use crate::crypt::ed25519;

pub fn send_msg(
    stream: &mut TcpStream,
    chat: &Hash::Buf,
    cypher: &Cypher::Buf,
    signature: &ed25519::SigBuffer,
) -> Res {
    // TODO: rename constants to something direction-agnostic
    // TODO: create global buffer builder functions
    let mut buf = MsgIn::PREPAD;
    let (cid_buf, cy_buf, sig_buf) = MsgIn::pad_split_mut(&mut buf);

    cid_buf.copy_from_slice(chat);
    cy_buf.copy_from_slice(cypher);
    sig_buf.copy_from_slice(signature);

    full_write(stream, &buf, "Failed to send message")
}

pub fn send_fetch(stream: &mut TcpStream, chat: &Hash::Buf) -> Res {
    let mut buf = Fetch::PREPAD;
    let (cid_buf,) = Fetch::pad_split_mut(&mut buf);

    cid_buf.copy_from_slice(chat);

    full_write(stream, &buf, "Failed to send fetch")
}

pub fn send_query(
    stream: &mut TcpStream,
    chat: &Hash::Buf,
    forwards: bool,
    count: u8,
    id: u32,
) -> Res {
    if count > 0x7f || id > 0xffffff { return Err("Query input too large") }
    let mut buf = Query::PREPAD;
    let (cid_buf, args_buf, mid_buf) = Query::pad_split_mut(&mut buf);

    cid_buf.copy_from_slice(chat);
    args_buf[0] = if forwards {count | 0x80} else {count};
    mid_buf.copy_from_slice(&id.to_be_bytes()[1..]);

    full_write(stream, &buf, "Failed to send query")
}
