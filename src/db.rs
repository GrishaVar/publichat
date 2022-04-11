use std::io::{Seek, SeekFrom, Read, BufReader, Write};
use std::path::Path;
use std::fs::OpenOptions;

use crate::MessageSt;
use crate::constants::*;
use crate::helpers::{Res, read_exact};

const MSG_ST_SIZE_U32: u32 = MSG_ST_SIZE as u32;
const MSG_ST_SIZE_U64: u64 = MSG_ST_SIZE as u64;
const NEG_MSG_ST_SIZE: i64 = -(MSG_ST_SIZE as i64);
// naming vars after their types is bad, but this makes life much easier later.

const EMPTY_RESPONSE: (u32, Vec<MessageSt>) = (0, Vec::new());  // doesn't allocate
const MAX_FILE_SIZE: u64 = u32::MAX as u64 * MSG_ST_SIZE_U64;  // approx 700 GB

pub fn push(path: &Path, msg: &[u8; MSG_ST_SIZE]) -> Res {
    let mut file = OpenOptions::new()
        .append(true)  // no reading or writing, only append
        .create(true)  // create file if it doesn't already exist
        .open(path)
        .map_err(|_| "Failed to open file")?;
    file.write(msg).map_err(|_| "Failed to write to file")?;
    Ok(())
}

pub fn fetch(
    path: &Path,
    count: u8,
) -> Result<(u32, Vec<MessageSt>), &'static str> {
    // Returns vec of the last `count` messages and the id of the first one.
    let mut file = match OpenOptions::new().read(true).open(path) {
        Ok(file) => file,
        _ => return Ok(EMPTY_RESPONSE),  // no file => no contents
    };

    if file.seek(SeekFrom::End(i64::from(count) * NEG_MSG_ST_SIZE)).is_err() {
        file.seek(SeekFrom::Start(0)).map_err(|_| "Failed to seek from start")?;
    }

    let pos = file.stream_position().map_err(|_| "Failed to read stream pos")?;
    if pos > MAX_FILE_SIZE { return Err("Too many messages in one file!") }

    let id = u32::try_from(pos / MSG_ST_SIZE_U64).unwrap();  // can't fail
    if pos != (id * MSG_ST_SIZE_U32).into() { return Err("File corruption") }

    let mut file = BufReader::new(file);
    let mut res = Vec::with_capacity(count.into());
    let mut buff = [0; MSG_ST_SIZE];
    for _ in 0..count {  // todo: precompute count
        if file.read_exact(&mut buff).is_err() { break }
        res.push(buff);
    }

    Ok((id, res))
}


pub fn query(
    path: &Path,
    id: u32,  // from which message
    mut count: u8,  // how many messages
    forward: bool,  // search forward or backward in time
) -> Result<(u32, Vec<MessageSt>), &'static str> {
    if count == 0 {return Ok(EMPTY_RESPONSE)}  // nothing to return
    if !forward && id == 0 {return Ok(EMPTY_RESPONSE)}  // nothing behind 0
    if count > MAX_FETCH_AMOUNT {count = MAX_FETCH_AMOUNT}  // request too many, return max amount

    let mut file = match OpenOptions::new().read(true).open(path) {
        Ok(file) => file,
        _ => return Ok(EMPTY_RESPONSE),  // no file => no contents
    };

    let db_size = file.metadata().map_err(|_| "Failed to get metadata")?.len() as u32;
    let db_len  = db_size / MSG_ST_SIZE_U32;
    assert_eq!(db_len * MSG_ST_SIZE_U32, db_size);  // todo: remove?

    if id > db_len {return Ok(EMPTY_RESPONSE)} // outside of range, return nothing
    if forward && id >= db_len-1 {return Ok(EMPTY_RESPONSE)}  // nothing ahead of db_len

    let count: u32 = count.into();
    let (start, len) = match forward {
        true => (id + 1, count.min(db_len - id - 1)),  // don't overshoot
        false => match id.checked_sub(count) {
            Some(start) => (start, count),  // fits perfectly
            None => (0, id),  // too far left, get first id messages
        }
    };

    file.seek(SeekFrom::Start(start as u64 * MSG_ST_SIZE_U64))
        .map_err(|_| "Failed to seek")?;
    let mut file = BufReader::new(file);
    let mut res = Vec::with_capacity(len as usize);
    let mut buf = [0; MSG_ST_SIZE];
    for _ in 0..len {
        read_exact(&mut file, &mut buf, "Failed to read from db (query)")?;
        res.push(buf);
    }

    Ok((start, res))
}
