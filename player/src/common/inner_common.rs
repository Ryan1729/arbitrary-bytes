pub use common::project_common::*;

#[derive(Default)]
pub struct GameState {
    pub byte_index: usize,
    pub need_first_render: bool,
}

pub const BYTES: &[u8] = include_bytes!("inner_common.rs");
