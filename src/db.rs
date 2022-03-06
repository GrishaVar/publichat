use std::io::{Seek, SeekFrom, Read, BufReader, Write};
use std::path::PathBuf;
use std::fs::OpenOptions;

use crate::MessageSt;
use crate::constants::*;

const FETCH_SIZE: usize = 50;
const FETCH_BLOCK_SIZE: usize = FETCH_SIZE * ST_SIZE;

pub fn push(path: &PathBuf, msg: &[u8; ST_SIZE]) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .append(true)  // no reading or writing, only append
        .create(true)  // create file if it doesn't already exist
        .open(path)?;
    file.write(msg)?;
    Ok(())
}


pub fn fetch(path: &PathBuf, up_to: Option<usize>) -> std::io::Result<Vec<MessageSt>> {
    let mut file = match OpenOptions::new().read(true).open(path) {
        Ok(file) => file,
        _ => return Ok(vec![]),
    };

    let size: usize = file.metadata()?.len() as usize;
    let (len, rem) = (size / ST_SIZE, size % ST_SIZE);  // compiler!
    assert_eq!(rem, 0);  // todo: remove?

    if let Some(up_to) = up_to {  // skip if too far ahead
        if up_to as usize > len {
            return Ok(vec![])
        }
    }

    if size > FETCH_BLOCK_SIZE {  // no seeking if fewer than 50 messages
        if let Some(up_to) = up_to {
            // return 50 before up_to (no including)
            if up_to > FETCH_SIZE {  // too far behind => don't seek
                let from = (up_to - FETCH_SIZE) * ST_SIZE;
                file.seek(SeekFrom::Start(from as u64))?;
            }
        } else {
            // return last 50
            file.seek(SeekFrom::End(-((FETCH_SIZE * ST_SIZE) as i64)))?;
        }
    }

    let mut file = BufReader::new(file);  // speeds up read by 2-3x!
    let mut res = Vec::with_capacity(FETCH_SIZE);
    let mut buff = [0; ST_SIZE];
    for _ in 0..up_to.unwrap_or(FETCH_SIZE).min(FETCH_SIZE) {
        file.read_exact(&mut buff).unwrap();
        res.push(buff);
    }

    Ok(res)
}
