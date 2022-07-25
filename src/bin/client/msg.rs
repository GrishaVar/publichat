use std::{fmt, time::Duration};

use rand;

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
    pub fn new(  // parse server's bytes into message text
        bytes: MessageOut,
        chat_key: &Hash,  // consider passing in pre-loaded AES thing
    ) -> Result<Self, &'static str> {
        let server_time = u64::from_be_bytes(bytes[MSG_OUT_TIME..][..TIME_SIZE].try_into().unwrap());
        let mut cypher: Cypher = bytes[MSG_OUT_CYPHER..][..CYPHER_SIZE].try_into().unwrap();
        let signature: Signature = bytes[MSG_OUT_SIG..][..SIGNATURE_SIZE].try_into().unwrap();

        // decrypt chat in-place
        apply_aes(chat_key, &mut cypher);
        let message_data = cypher;  // rename variable for clarity

        // TODO: magic numbers
        let received_chat_key = &message_data[..4];
        let client_time = u64::from_be_bytes(message_data[4..][..8].try_into().unwrap());
        let pub_key = &message_data[4+8..][..32];  
        let padded_msg = &message_data[4+8+32..];

        // find padding end
        let pad_start = padded_msg.iter()
            .rposition(|&b| b == chat_key[0])
            .ok_or("Invalid pad: indicator not found")?;
        let message = &padded_msg[..pad_start];
        // TODO: magic numbers ^

        // assign varified randomly
        let verified =
            server_time.abs_diff(client_time) < 10 * 1000  // no more than 10 sec  // TODO: magic numbers
            && received_chat_key == &chat_key[..4];
            // TODO: && signature valid

        // assert utf8
        if std::str::from_utf8(message).is_err() {
            return Err("Non-utf8 message!")
        }

        // build string
        let cached_str_repr = {
            let (v_start, v_end) = match verified {
                true  => ("\x1B[32m✔\x1B[0m", ""),
                false => ("\x1B[31m✗", "\x1B[0m"),
            };
            let user = &base64::encode(pub_key)[..15];
            let time = Duration::from_millis(server_time).as_secs();
            // let msg = String::from_utf8_lossy(&cypher[..pad_length as usize]);
            let msg = std::str::from_utf8(message).unwrap();
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

    pub fn make_cypher(
        text: &str,
        chat_key: &Hash,
        // pub_key: &Hash
    ) -> Result<Cypher, ()> {
        let mut res = [0; CYPHER_SIZE];
        if text.len() > 396 - 1 { return Err(()) }  // msg too long

        use std::time::{SystemTime, UNIX_EPOCH};
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH).unwrap()
            .as_millis()  // TODO: convert to u64?
            .to_be_bytes();

        // copy in basic data
        res[..4].copy_from_slice(&chat_key[..4]);
        res[4..][..8].copy_from_slice(&time[8..]);
        res[4+8..][..HASH_SIZE].copy_from_slice(&[0; HASH_SIZE]);  // TODO: add public key

        // copy in message
        res[4+8+HASH_SIZE..][..text.len()].copy_from_slice(text.as_bytes());

        // padding
        let pad_start_pos = 4+8+HASH_SIZE+text.len();
        res[pad_start_pos] = chat_key[0];  // pad indicator
        res[pad_start_pos+1..].fill_with(|| {
            let mut res = rand::random::<u8>();
            while res == chat_key[0] { res = rand::random(); }
            res  // TODO: potentially non-terminating! ^
        });

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
