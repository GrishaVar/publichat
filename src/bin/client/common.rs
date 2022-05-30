use std::{time::Duration, collections::VecDeque};

use crate::msg::Message;
use crate::Hash;

pub const FQ_DELAY: Duration = Duration::from_millis(200);
const DISP_FPS: u64 = 1;
pub const _DISP_DELAY: Duration = Duration::from_millis(1000 / DISP_FPS);

pub struct GlobalState {
    pub queue: VecDeque<Message>,
    pub chat_key: Hash,
    pub chat_id: Hash,
    pub user_id: Hash,
    pub min_id: u32,
    pub max_id: u32,  // inclusive
}