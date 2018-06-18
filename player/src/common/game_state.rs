use inner_common::*;

impl GameState {
    pub fn new() -> GameState {
        GameState {
            byte_index: 0,
            need_first_render: true,
        }
    }
}
