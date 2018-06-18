use colours::{BLACK, BLUE, GREEN, GREY, PURPLE, RED, WHITE, YELLOW};
use common::project_common::{Colour, SCREEN_HEIGHT, SCREEN_WIDTH};
use inner_common::*;

pub struct Framebuffer {
    pub buffer: Vec<u32>,
}

impl PartialEq for Framebuffer {
    fn eq(&self, other: &Framebuffer) -> bool {
        &self.buffer[..] == &other.buffer[..]
    }
}

impl Eq for Framebuffer {}

#[allow(dead_code)]
impl Framebuffer {
    pub fn new() -> Framebuffer {
        Framebuffer::default()
    }
}

impl Default for Framebuffer {
    fn default() -> Self {
        let mut buffer = Vec::new();
        buffer.resize(SCREEN_WIDTH * SCREEN_HEIGHT, BLACK);

        Framebuffer { buffer }
    }
}
