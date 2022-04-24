use std::{fmt, time::Duration};

use aes::{Aes256, cipher::{KeyIvInit, StreamCipher}};
use ctr::Ctr128BE;

use publichat::constants::*;

type AesCtr = Ctr128BE<Aes256>;

#[derive(Debug)]
pub struct Message {
    time: Duration,
    user: Hash,
    text: Contents,
    length: u8,
    verified: bool,
}

impl Message {
    pub fn new(
        bytes: [u8; MSG_OUT_SIZE],
        aes_key: &Hash,  // consider passing in pre-loaded AES thing
    ) -> Result<Self, &'static str> {
        let time = u64::from_be_bytes(bytes[MSG_OUT_TIME..][..TIME_SIZE].try_into().unwrap());
        let user: Rsa = bytes[MSG_OUT_RSA..][..RSA_SIZE].try_into().unwrap();
        let mut cypher: Contents = bytes[MSG_OUT_CYPHER..][..CYPHER_SIZE].try_into().unwrap();

        let mut iv = [0u8; 16];
        iv[15] = 1;

        // decrypt chat in-place
        let mut decrypter = AesCtr::new(aes_key.into(), &iv.into());
        decrypter.apply_keystream(&mut cypher);

        // find padding end
        let length = CYPHER_SIZE as u8 - cypher[CYPHER_SIZE-1];

        // assert utf8
        if std::str::from_utf8(&cypher[..length as usize]).is_ok() {
            Ok(Self {
                time: Duration::from_millis(time),
                user,
                text: cypher,
                length,
                verified: true,  // TODO: update when signatures come around
            })
        } else {
            Err("Non-utf8 message")
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: consider caching?
        let (v_start, v_end) = match self.verified {
            true  => ("\x1B[32m✔\x1B[0m", ""),
            false => ("\x1B[31m✗", "\x1B[0m"),
        };
        let user = &base64::encode(self.user)[..10];
        let time = self.time.as_secs();
        let msg = std::str::from_utf8(&self.text[..self.length as usize]).unwrap();
        write!(
            f,
            "{v_start} {user} @ {time}: {msg}{v_end}",
        )
    }
}
