use std::io::{Write, Read};

pub type Res = Result<(), &'static str>;

pub fn full_write(stream: &mut impl Write, buf: &[u8], err: &'static str) -> Res {
    // writes buffer to stream and flushes it
    match stream.write(buf).and(stream.flush()) {
        Ok(_) => Ok(()),
        Err(_) => Err(err),
    }
}

pub fn read_exact(stream: &mut impl Read, buf: &mut [u8], err: &'static str) -> Res {
    match stream.read_exact(buf) {
        Ok(_) => Ok(()),
        Err(_) => Err(err),
    }
}
