use std::io::{Write, Read};
use std::path::PathBuf;

pub type Res = Result<(), &'static str>;

pub fn full_write(stream: &mut impl Write, buf: &[u8], err: &'static str) -> Res {
    // writes buffer to stream and flushes it
    match stream.write(buf).and(stream.flush()) {
        Ok(_) => Ok(()),
        Err(_) => Err(err),
    }
}

pub fn read_exact(stream: &mut impl Read, buf: &mut [u8], err: &'static str) -> Res {
    stream.read_exact(buf).map_err(|_| err)
}

pub struct Globals {  // owns all its data!
    pub data_dir:    PathBuf,
    pub git_hash:    [u8; 40],
}

// only tls
#[cfg(all(not(feature = "minify"), feature = "tls"))]
pub const FILE_INDEX_HTML:  &[u8] = include_bytes!("../target/index-tls.html");
#[cfg(all(not(feature = "minify"), feature = "tls"))]
pub const FILE_MOBILE_HTML: &[u8] = include_bytes!("../target/mobile-tls.html");
#[cfg(all(not(feature = "minify"), feature = "tls"))]
pub const FILE_404_HTML:    &[u8] = include_bytes!("../target/404.html");
