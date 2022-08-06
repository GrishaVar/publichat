macro_rules! rep { ($_:tt; $($r:tt)+) => {$($r)+}; }  // repeat _ times

macro_rules! build_buf {
    (
        $name:ident;        // identifier (name) followed by a `;`
        $($len:expr),+      // 1 or more expressions (lengths) separated by commas
        $(;$($rest:item)+)? // optional: `;` followed by one or more tt's
    ) => {
        pub mod $name {
            use crate::constants::*;

            pub const SIZE: usize = $( $len + )* 0;
            pub type Buf = [u8; SIZE];
            pub const DEFAULT: Buf = [0; SIZE];
            // const LEN: usize = $( ig!{[1] $len} + )* 0;
            // const SIZES: [usize; Self::LEN] = [ $( $len, )* ];

            type NTuple<'a> = ($( rep!($len; &'a [u8]), )*);
            pub fn split(buf: &Buf) -> NTuple {
                let mut _b = buf.as_slice();
                ($({let (cur, new) = _b.split_at($len); _b = new; cur},)*)
            }

            type NTupleMut<'a> = ($( rep!($len; &'a mut [u8]), )*);
            pub fn split_mut(buf: &mut Buf) -> NTupleMut {
                let mut _b = buf.as_mut_slice();
                ($({let (cur, new) = _b.split_at_mut($len); _b = new; cur},)*)
            }

            $( $( $rest )+ )?
        }
    };
}

use crate::constants::{PADDING_SIZE, END_PADDING}; 
const fn pad_buf<const L: usize>(pad: [u8; PADDING_SIZE]) -> [u8; L] {
    // Create an empty buffer, put pad in the beginning
    // and END_PADDING at the end. This function is const!
    let mut buf = [0; L];
    let mut i = 0;
    while { i += 1; i <= PADDING_SIZE } {  // for-loops aren't const :/
        let j = i - 1;  // TODO: find a way to avoid this?
        buf[j] = pad[j];
        buf[buf.len() - PADDING_SIZE + j] = END_PADDING[j];
    }
    buf
}
macro_rules! prepad {  // apply pad_buf
    ($pad:expr) => {
        pub type PadBuf = [u8; SIZE + 2*PADDING_SIZE];
        pub const PREPAD: PadBuf = super::pad_buf($pad);
        pub fn pad_split(buf: &PadBuf) -> NTuple {
            split(buf[PADDING_SIZE..][..SIZE].try_into().unwrap())
        }
        pub fn pad_split_mut(buf: &mut PadBuf) -> NTupleMut {
            split_mut((&mut buf[PADDING_SIZE..][..SIZE]).try_into().unwrap())
        }
    };
}

// server-side
// TODO: use in server code
// TODO: combine cypher and sig into one block? Server doesn't need them separately
build_buf!(MsgSt; TIME_SIZE, CYPHER_SIZE, SIGNATURE_SIZE);

// server -> client
build_buf!(MsgHead; PADDING_SIZE, 1, MSG_ID_SIZE, 1;
    pub use crate::constants::MSG_PADDING as PAD;  // includes padding
);
build_buf!(MsgOut; TIME_SIZE, CYPHER_SIZE, SIGNATURE_SIZE);

// client -> server
build_buf!(Fetch; CHAT_ID_SIZE; prepad!(FETCH_PADDING););
build_buf!(Query; CHAT_ID_SIZE, 1, MSG_ID_SIZE; prepad!(QUERY_PADDING););
build_buf!(MsgIn; CHAT_ID_SIZE, CYPHER_SIZE, SIGNATURE_SIZE; prepad!(SEND_PADDING););

// client-side
build_buf!(Cypher; CYPHER_CHAT_KEY_SIZE, TIME_SIZE, HASH_SIZE, CYPHER_PAD_MSG_SIZE);

// misc
// splitting not needed, make for consistency
// TODO: optionally skip functions?
build_buf!(Hash; HASH_SIZE);
build_buf!(Signature; SIGNATURE_SIZE);
