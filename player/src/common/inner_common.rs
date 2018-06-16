pub use common::project_common::*;

pub struct GameState {
    pub byte_index: usize,
}

pub const BYTES: &[u8] = include_bytes!("inner_common.rs");
