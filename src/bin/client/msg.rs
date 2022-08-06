use std::{str, fmt, time::{Duration, SystemTime, UNIX_EPOCH}};

use crossterm::style::{Stylize, Color};
use rand::Rng;

use publichat::buffers::{Hash, Cypher, MsgOut};
use crate::crypt::*;
use crate::common::{
    VERIFY_TOLERANCE_MS,
    USER_ID_CHAR_COUNT,
};

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
        mut bytes: MsgOut::Buf,
        chat_key: &Hash::Buf,
    ) -> Result<Self, &'static str> {
        // deconstruct bytes
        let (st_buf, c_buf, s_buf) = MsgOut::split_mut(&mut bytes);

        // shadow to change types (unwraps CANNOT fail here; len check skipped!)
        let server_time = u64::from_be_bytes(st_buf.try_into().unwrap());
        let cypher: &mut Cypher::Buf = c_buf.try_into().unwrap();
        let signature: &ed25519::SigBuffer = (&*s_buf).try_into().unwrap();
            // TODO: the &* casts the `&mut [u8]` into a `&[u8]`. Ugly!

        // prepare for signature check before cypher gets decrypted
        let hashed_cypher = sha::hash(cypher.as_slice());

        // decrypt chat in-place
        aes::apply(chat_key, cypher);
        let cypher_data = cypher;  // rename variable for clarity

        // deconstruct msg_data
        let (ck_buf, ct_buf, pk_buf, msg_buf) = Cypher::split(cypher_data);

        // shadow to change types (unwraps CANNOT fail here; len check skipped!)
        let client_time = u64::from_be_bytes(ct_buf.try_into().unwrap());
        let pub_key: &Hash::Buf = pk_buf.try_into().unwrap();

        // find padding
        let pad_start = msg_buf.iter()
            .rposition(|&b| b == chat_key[0])
            .ok_or("Invalid pad: indicator not found")?;
        let message = &msg_buf[..pad_start];

        // verify message, prep verification mark
        let verified =
            chat_key.starts_with(ck_buf)
            && server_time.abs_diff(client_time) < VERIFY_TOLERANCE_MS
            && ed25519::verify(&hashed_cypher, pub_key, signature)?;
        let v_mark = if verified { '✔'.green() } else { '✗'.red().rapid_blink() };

        // prep username string
        let user = &base64::encode(pub_key)[..USER_ID_CHAR_COUNT];
        let colour = Color::from({
            // user colour taken from last three bytes of public key
            // 3 is an unavoidable magic number of colours in RGB,
            // lets hope humans don't evolve more cone cell types
            let c = &pub_key[pub_key.len()-3..];
            (c[0], c[1], c[2])
        });
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
        chat_key: &Hash::Buf,
        pub_key: &Hash::Buf,
    ) -> Result<Cypher::Buf, ()> {
        let time: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH).expect("Woah, get with the times!")
            .as_millis().try_into().expect("Alright, futureboy");
        
        let mut res = Cypher::DEFAULT;
        let (ck_buf, t_buf, pk_buf, msg_buf) = Cypher::split_mut(&mut res);
        
        if text.len() > msg_buf.len() - 1 { return Err(()) }  // msg too long

        // copy in basic data
        ck_buf.copy_from_slice(&chat_key[..ck_buf.len()]);
        t_buf.copy_from_slice(&time.to_be_bytes());
        pk_buf.copy_from_slice(pub_key);
        msg_buf[..text.len()].copy_from_slice(text.as_bytes());

        // padding
        let mut rng = rand::thread_rng();
        msg_buf[text.len()] = chat_key[0];  // pad indicator
        msg_buf[text.len()+1..].fill_with(||
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
