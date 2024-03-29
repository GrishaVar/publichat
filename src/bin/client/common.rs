use std::{time::Duration, collections::VecDeque};

use publichat::buffers::hash::Buf as HashBuf;
use crate::msg::Message;

pub const FQ_DELAY: Duration = Duration::from_millis(200);

const DISP_FPS: u64 = 100;
pub const _DISP_DELAY: Duration = Duration::from_millis(1000 / DISP_FPS);

pub struct GlobalState {
    pub queue: VecDeque<Message>,
    pub chat_key: HashBuf,
    pub chat_id: HashBuf,
    pub min_id: u32,
    pub max_id: u32,  // inclusive
}

pub const VERIFY_TOLERANCE_MS: u64 = 10 * 1000;  // time between server and client
pub const USER_ID_CHAR_COUNT: usize = 15;  // how many b64 chars are displayed
