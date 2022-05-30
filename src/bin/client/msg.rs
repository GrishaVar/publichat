use std::{fmt, time::Duration};

use publichat::constants::*;

use crate::crypt::apply_aes;

#[derive(Debug)]
pub struct Message {
    // time: Duration,
    // user: Hash,
    // text: Contents,
    // verified: bool,
    // pub length: u8,
    pub repr: String,  // TODO: duplicate storage?
    // TODO: consider just having strings instead of this struct
    // TODO: what do I do when the time needs to be displayed differently?
}

impl Message {
    pub fn new(
        bytes: [u8; MSG_OUT_SIZE],
        aes_key: &Hash,  // consider passing in pre-loaded AES thing
    ) -> Result<Self, &'static str> {
        let time = u64::from_be_bytes(bytes[MSG_OUT_TIME..][..TIME_SIZE].try_into().unwrap());
        let user: Rsa = bytes[MSG_OUT_RSA..][..RSA_SIZE].try_into().unwrap();
        let mut cypher: Contents = bytes[MSG_OUT_CYPHER..][..CYPHER_SIZE].try_into().unwrap();

        // decrypt chat in-place
        apply_aes(aes_key, &mut cypher);

        // find padding end
        let length = CYPHER_SIZE as u8 - cypher[CYPHER_SIZE-1];

        // assign varified randomly
        // TODO: update when signatures come around
        let verified = time & 255 > 255/10;  // approx 90% are verified
        let time = Duration::from_millis(time);

        // assert utf8
        if std::str::from_utf8(&cypher[..length as usize]).is_err() {
            return Err("Non-utf8 message!")
        }

        // build string
        let cached_str_repr = {
            let (v_start, v_end) = match verified {
                true  => ("\x1B[32m✔\x1B[0m", ""),
                false => ("\x1B[31m✗", "\x1B[0m"),
            };
            let user = &base64::encode(user)[..10];
            let time = time.as_secs();
            let msg = std::str::from_utf8(&cypher[..length as usize]).unwrap();
            format!("{v_start} {user} @ {time}: {msg}{v_end}")
        };

        Ok(Self {
            // time,
            // user,
            // text: cypher,
            // verified,
            // length,
            repr: cached_str_repr,
        })
    }

    pub fn make_cypher(text: &str, chat_key: &Hash) -> Result<Contents, ()> {
        let mut res = [0; CYPHER_SIZE];
        if text.len() > CYPHER_SIZE - 1 { return Err(()) }  // msg too long

        // padding
        let pad_chr = CYPHER_SIZE - text.len();
        let pad_chr = u8::try_from(pad_chr).unwrap();  // TODO: remove unwrap
        res[..text.len()].copy_from_slice(text.as_bytes());
        res[text.len()..].fill(pad_chr);

        // AES
        apply_aes(chat_key, &mut res);

        Ok(res)
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.repr)
    }
}
