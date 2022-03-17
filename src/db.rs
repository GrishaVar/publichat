use std::io::{Seek, SeekFrom, Read, BufReader, Write};
use std::path::PathBuf;
use std::fs::OpenOptions;

use crate::MessageSt;
use crate::constants::*;

const FETCH_SIZE: u8 = 50;

pub fn push(path: &PathBuf, msg: &[u8; ST_SIZE]) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .append(true)  // no reading or writing, only append
        .create(true)  // create file if it doesn't already exist
        .open(path)?;
    file.write(msg)?;
    Ok(())
}

pub fn fetch_latest(path: &PathBuf, count: u8) -> std::io::Result<Vec<MessageSt>> {
    let mut file = match OpenOptions::new().read(true).open(path) {
        Ok(file) => file,
        _ => return Ok(Vec::new()),  // no file => no contents
    };

    if let Err(_) = file.seek(SeekFrom::End(-(count as i64))) {
        file.seek(SeekFrom::Start(0))?;
    }

    let mut file = BufReader::new(file);
    let mut res = Vec::with_capacity(count.into());
    let mut buff = [0; ST_SIZE];
    for _ in 0..count {
        if let Err(_) = file.read_exact(&mut buff) { break }
        res.push(buff);
    }

    Ok(res)
}


pub fn fetch(
    path: &PathBuf,
    id: u32,  // from which message
    count: u8,  // how many messages
    forward: bool,  // search forward of backward in time
) -> std::io::Result<Vec<MessageSt>> {
    if count == 0 {return Ok(Vec::new())}  // nothing to return
    if !forward && id == 0 {return Ok(Vec::new())}  // nothing behind 0
    if count > FETCH_SIZE {return Ok(Vec::new())}  // request too many, return nothing

    let mut file = match OpenOptions::new().read(true).open(path) {
        Ok(file) => file,
        _ => return Ok(Vec::new()),  // no file => no contents
    };

    let db_size = file.metadata()?.len() as u32;
    let db_len  = db_size / ST_SIZE as u32;
    assert_eq!(db_len * ST_SIZE as u32, db_size);  // todo: remove?

    if id > db_len {return Ok(Vec::new())} // outside of range, return nothing
    if forward && id >= db_len-1 {return Ok(Vec::new())}  // nothing ahead of db_len

    let count: u32 = count.into();
    let (start, len) = match forward {
        true => (id + 1, count.min(db_len - id - 1)),  // don't overshoot
        false => match id.checked_sub(count) {
            Some(start) => (start, count),  // fits perfectly
            None => (0, id),  // too far left, get first id messages
        }
    };

    // file.seek(SeekFrom::Start(30))?;
    file.seek(SeekFrom::Start(start as u64 * ST_SIZE as u64))?;
    let mut file = BufReader::new(file);
    let mut res = Vec::with_capacity(len as usize);
    let mut buff = [0; ST_SIZE];
    for _ in 0..len {
        file.read_exact(&mut buff)?;
        res.push(buff);
    }

    Ok(res)
}
