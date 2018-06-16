use common::*;
use common::inner_common::BYTES;

#[inline]
pub fn update_and_render(framebuffer: &mut Framebuffer, state: &mut GameState, input: Input) {
    for (i, byte) in BYTES.iter().cloned().cycle().skip(state.byte_index).take(SCREEN_LENGTH).enumerate() {
        framebuffer.buffer[i] = colour_from_byte(byte);
    }
}

fn colour_from_byte(byte: u8) -> u32 {
    match byte & 0b111 {
        0 => BLUE,
        1 => GREEN,
        2 => RED,
        3 => YELLOW,
        4 => PURPLE,
        5 => GREY,
        6 => WHITE,
        7 => BLACK,
        _ => 0,
    }
}
