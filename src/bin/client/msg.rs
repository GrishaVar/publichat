use std::{str, fmt, time::{Duration, SystemTime, UNIX_EPOCH}};

use crossterm::style::{Stylize, Color};
use rand::Rng;

use publichat::constants::*;  // TODO: Signature defined twice

use crate::crypt::*;

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
        let signature = bytes[MSG_OUT_SIG..][..SIGNATURE_SIZE].try_into().unwrap();

        // prepare for signature check before cypher gets decrypted
        let hashed_cypher = sha::hash(cypher.as_slice());

        // decrypt chat in-place
        aes::apply(chat_key, &mut cypher);
        let message_data = cypher;  // rename variable for clarity

        // deconstruct message_data
        // TODO: magic numbers
        let received_chat_key = &message_data[..4];
        let client_time = u64::from_be_bytes(message_data[4..][..8].try_into().unwrap());
        let pub_key = &message_data[4+8..][..32].try_into().unwrap();
        let padded_msg = &message_data[4+8+32..];

        // find padding
        let pad_start = padded_msg.iter()
            .rposition(|&b| b == chat_key[0])
            .ok_or("Invalid pad: indicator not found")?;
        let message = &padded_msg[..pad_start];
        // TODO: magic numbers ^

        // verify message, prep verification mark
        let verified =
            received_chat_key == &chat_key[..4]
            && server_time.abs_diff(client_time) < 10 * 1000  // no more than 10 sec  // TODO: magic numbers
            && ed25519::verify(&hashed_cypher, pub_key, signature)?;
        let v_mark = if verified { '✔'.green() } else { '✗'.red().rapid_blink() };

        // prep username string
        let user = &base64::encode(pub_key)[..15];
        let colour = Color::from((
            // user colour taken from last three bytes of public key
            pub_key[32-3],  // TODO: magic numbers
            pub_key[32-2],
            pub_key[32-1],
        ));
        let user_c = user.on(colour).with(w_or_b(&colour));

        // prep time string
        let time = Duration::from_millis(server_time);
        let time_s = {  // TODO: use date/time-related crate (?)
            let time_sec = time.as_secs();

            let hour = (time_sec / 3600) % 24;
            let min = (time_sec / 60) % 60;
            let sec = time_sec % 60;

            format!("{hour}:{min}:{sec}")
        };

        // prep message string
        let msg = str::from_utf8(message).map_err(|_| "Non-utf8 message!")?;

        // build string
        let cached_str_repr = format!("{v_mark} {user_c} {time_s} {msg}");

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
        pub_key: &Hash
    ) -> Result<Cypher, ()> {
        let mut res = [0; CYPHER_SIZE];
        if text.len() > 396 - 1 { return Err(()) }  // msg too long

        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH).unwrap()
            .as_millis()  // TODO: convert to u64?
            .to_be_bytes();

        // copy in basic data
        res[..4].copy_from_slice(&chat_key[..4]);
        res[4..][..8].copy_from_slice(&time[8..]);
        res[4+8..][..HASH_SIZE].copy_from_slice(pub_key);

        // copy in message
        res[4+8+HASH_SIZE..][..text.len()].copy_from_slice(text.as_bytes());

        // padding
        let mut rng = rand::thread_rng();
        let pad_start_pos = 4+8+HASH_SIZE+text.len();
        res[pad_start_pos] = chat_key[0];  // pad indicator
        res[pad_start_pos+1..].fill_with(||
            rng.gen_range(1u8..=0xff).wrapping_add(chat_key[0])
        );

        // AES
        aes::apply(chat_key, &mut res);

        Ok(res)
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.repr)
    }
}

fn w_or_b(colour: &Color) -> Color {  // TODO: where should this function be?
    // Return white for dark colours, black for light colours
    return if let Color::Rgb{r, g, b} = colour {
        let is_dark = (
              0.299 * f32::from(*r)
            + 0.587 * f32::from(*g)
            + 0.114 * f32::from(*b)
        ) < 150.0;
        if is_dark { Color::White } else { Color::Black }
    } else { unreachable!("w_or_b called on non-rgb colour") }
}
