extern crate features;
use features::{log, GLOBAL_ERROR_LOGGER, GLOBAL_LOGGER};
extern crate platform_types;
use platform_types::{Button, Input, Speaker, State, StateParams, SFX};
extern crate rendering;
use rendering::{Framebuffer, BLACK, BLUE, GREEN, GREY, PALETTE, PURPLE, RED, WHITE, YELLOW};

macro_rules! d {
    () => {
        Default::default()
    };
}

pub struct EntireState {
    pub game_state: GameState,
    pub framebuffer: Framebuffer,
    pub input: Input,
    pub speaker: Speaker,
}

impl EntireState {
    pub fn new((seed, logger, error_logger): StateParams) -> Self {
        let framebuffer = Framebuffer::new();

        unsafe {
            GLOBAL_LOGGER = logger;
            GLOBAL_ERROR_LOGGER = error_logger;
        }

        EntireState {
            game_state: GameState::new(),
            framebuffer,
            input: Input::new(),
            speaker: Speaker::new(),
        }
    }
}

impl State for EntireState {
    fn frame(&mut self, handle_sound: fn(SFX)) {
        update_and_render(
            &mut self.framebuffer,
            &mut self.game_state,
            self.input,
            &mut self.speaker,
        );

        self.input.previous_gamepad = self.input.gamepad;

        for request in self.speaker.drain() {
            handle_sound(request);
        }
    }

    fn press(&mut self, button: Button::Ty) {
        if self.input.previous_gamepad.contains(button) {
            //This is meant to pass along the key repeat, if any.
            //Not sure if rewriting history is the best way to do this.
            self.input.previous_gamepad.remove(button);
        }

        self.input.gamepad.insert(button);
    }

    fn release(&mut self, button: Button::Ty) {
        self.input.gamepad.remove(button);
    }

    fn get_frame_buffer(&self) -> &[u32] {
        &self.framebuffer.buffer
    }

    fn update_bytes(&mut self, bytes: Vec<u8>) {
        self.game_state.bytes = bytes;
        self.game_state.render_mode = match self.game_state.render_mode {
            RenderMode::Quadrilateral(_) => RenderMode::Quadrilateral(d!()),
            RenderMode::ThreeBitsPerPixel(_) => RenderMode::ThreeBitsPerPixel(d!()),
        };
    }
}

impl GameState {
    pub fn new() -> GameState {
        GameState {
            bytes: DEFAULT_BYTES.to_vec(),
            ..d!()
        }
    }
}

#[derive(Default)]
pub struct QuadrilateralState {
    pub byte_index: usize,
}

#[derive(Default)]
pub struct ThreeBitsPerPixelState {
    pub byte_index: usize,
    pub done_first_render: bool,
}

pub enum RenderMode {
    Quadrilateral(QuadrilateralState),
    ThreeBitsPerPixel(ThreeBitsPerPixelState),
}

impl Default for RenderMode {
    fn default() -> RenderMode {
        RenderMode::Quadrilateral(d!())
    }
}

#[derive(Default)]
pub struct GameState {
    pub render_mode: RenderMode,
    pub bytes: Vec<u8>,
}

pub const DEFAULT_BYTES: &[u8] = include_bytes!("../../../test/full_screen_quad.txt"); //include_bytes!("player.rs");

#[inline]
pub fn update_and_render(
    framebuffer: &mut Framebuffer,
    state: &mut GameState,
    input: Input,
    speaker: &mut Speaker,
) {
    if input.pressed_this_frame(Button::Start) {
        state.render_mode = match state.render_mode {
            RenderMode::Quadrilateral(_) => RenderMode::ThreeBitsPerPixel(d!()),
            RenderMode::ThreeBitsPerPixel(_) => RenderMode::Quadrilateral(d!()),
        };
    }

    match state.render_mode {
        RenderMode::Quadrilateral(ref mut q_state) => {
            update_and_render_quadrilateral(framebuffer, q_state, input, speaker, &state.bytes)
        }
        RenderMode::ThreeBitsPerPixel(ref mut tbbp_state) => {
            update_and_render_three_bits_per_pixel(
                framebuffer,
                tbbp_state,
                input,
                speaker,
                &state.bytes,
            )
        }
    }
}

pub fn update_and_render_quadrilateral(
    framebuffer: &mut Framebuffer,
    state: &mut QuadrilateralState,
    input: Input,
    speaker: &mut Speaker,
    bytes: &[u8],
) {
    if state.byte_index == 0 {
        framebuffer.clear_to(PALETTE[PALETTE.len() - 1]);
    }

    macro_rules! extract_or_zero {
        ($i: expr) => {
            if $i >= bytes.len() {
                0
            } else {
                bytes[$i]
            }
        };
    }

    log!((state.byte_index, bytes));

    let u8s = [
        extract_or_zero!(state.byte_index + 0),
        extract_or_zero!(state.byte_index + 1),
        extract_or_zero!(state.byte_index + 2),
        extract_or_zero!(state.byte_index + 3),
        extract_or_zero!(state.byte_index + 4),
        extract_or_zero!(state.byte_index + 5),
        extract_or_zero!(state.byte_index + 6),
        extract_or_zero!(state.byte_index + 7),
    ];

    framebuffer.draw_filled_quad(
        u8s[0],
        u8s[1],
        u8s[2],
        u8s[3],
        u8s[4],
        u8s[5],
        u8s[6],
        u8s[7],
        PALETTE[(state.byte_index / 8) % PALETTE.len()],
    );

    state.byte_index += 8;
    if state.byte_index >= bytes.len() {
        // TODO should we fade out or something?
        state.byte_index = 0;
    }
}

pub fn update_and_render_three_bits_per_pixel(
    framebuffer: &mut Framebuffer,
    state: &mut ThreeBitsPerPixelState,
    input: Input,
    speaker: &mut Speaker,
    bytes: &[u8],
) {
    if !state.done_first_render {
        render_from_msb(bytes, &mut framebuffer.buffer, state.byte_index);
        state.done_first_render = true;
        return;
    }

    if input.pressed_this_frame(Button::Right) {
        render_from_lsb(bytes, &mut framebuffer.buffer, state.byte_index);
    } else if input.pressed_this_frame(Button::Left) {
        render_from_msb(bytes, &mut framebuffer.buffer, state.byte_index);
    }
}

macro_rules! advance {
    ($i:ident, $max:expr) => {
        $i += 1;
        if $i >= $max {
            break;
        }
    };
}

fn render_from_lsb(bytes: &[u8], buffer: &mut [u32], byte_index: usize) {
    let len = buffer.len();
    if byte_index >= len {
        return;
    }

    let mut i = 0;
    let mut iter = bytes.iter().cloned().cycle().skip(byte_index);

    /*
    b = bit we want.
    . = bit we don't want yet.

    .....bbb
    000..bbb
    000000bb | .......b << 2
    0....bbb
    0000.bbb
    0000000b | ......bb << 1
    00...bbb
    00000bbb
    .....bbb
    */

    loop {
        let mut byte = iter.next().unwrap();
        buffer[i] = colour_from_byte(byte);

        advance!(i, len);

        byte >>= 3;

        buffer[i] = colour_from_byte(byte);

        advance!(i, len);

        byte >>= 3;

        {
            let mut merged_byte = byte;

            byte = iter.next().unwrap();

            merged_byte |= (byte & 0b1) << 2;

            buffer[i] = colour_from_byte(merged_byte);

            advance!(i, len);

            byte >>= 1;
        }

        buffer[i] = colour_from_byte(byte);

        advance!(i, len);

        byte >>= 3;

        buffer[i] = colour_from_byte(byte);

        advance!(i, len);

        byte >>= 3;

        {
            let mut merged_byte = byte;

            byte = iter.next().unwrap();

            merged_byte |= (byte & 3) << 1;

            byte >>= 2;

            buffer[i] = colour_from_byte(merged_byte);

            advance!(i, len);
        }

        buffer[i] = colour_from_byte(byte);

        advance!(i, len);

        byte >>= 3;

        buffer[i] = colour_from_byte(byte);

        advance!(i, len);
    }
}

fn render_from_msb(bytes: &[u8], buffer: &mut [u32], byte_index: usize) {
    let len = buffer.len();
    if byte_index >= len {
        return;
    }

    let mut i = 0;
    let mut iter = bytes.iter().cloned().cycle().skip(byte_index);

    /*
    b = bit we want.
    . = bit we don't want yet.

    bbb.....
    bbb..000
    bb000000 | b....... >> 7
    0bbb....
    0000bbb.
    0000000b | bb...... >> 6
    00bbb...
    00000bbb
    bbb.....
    */

    loop {
        let mut byte = iter.next().unwrap();
        buffer[i] = colour_from_byte(byte >> 5);

        advance!(i, len);

        byte <<= 3;

        buffer[i] = colour_from_byte(byte >> 5);

        advance!(i, len);

        byte <<= 3;

        {
            let mut merged_byte = byte;

            byte = iter.next().unwrap();

            merged_byte |= (byte & 0b1000_0000) >> 2;

            buffer[i] = colour_from_byte(merged_byte >> 5);

            advance!(i, len);

            byte <<= 1;
        }

        buffer[i] = colour_from_byte(byte >> 5);

        advance!(i, len);

        byte <<= 3;

        buffer[i] = colour_from_byte(byte >> 5);

        advance!(i, len);

        byte <<= 3;

        {
            let mut merged_byte = byte;

            byte = iter.next().unwrap();

            merged_byte |= (byte & 0b1100_0000) >> 1;

            byte <<= 2;

            buffer[i] = colour_from_byte(merged_byte >> 5);

            advance!(i, len);
        }

        buffer[i] = colour_from_byte(byte >> 5);

        advance!(i, len);

        byte <<= 3;

        buffer[i] = colour_from_byte(byte >> 5);

        advance!(i, len);
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

#[cfg(test)]
mod tests {
    use super::*;

    //What does it mean!?!
    const double_rainbow: [u32; 16] = [
        BLUE, GREEN, RED, YELLOW, PURPLE, GREY, WHITE, BLACK, BLUE, GREEN, RED, YELLOW, PURPLE,
        GREY, WHITE, BLACK,
    ];

    #[derive(PartialEq, Eq)]
    struct PrettySlice<'a>(&'a [u32]);

    use std::fmt;

    impl<'a> fmt::Debug for PrettySlice<'a> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "[")?;

            let mut sep = "";
            for elem in self.0.iter() {
                let elem_string = match *elem {
                    BLUE => "Blue",
                    GREEN => "Green",
                    RED => "Red",
                    YELLOW => "Yellow",
                    PURPLE => "Purple",
                    GREY => "Grey",
                    WHITE => "White",
                    BLACK => "Black",
                    _ => "Unknown colour",
                };

                write!(f, "{}{}", sep, elem_string)?;
                sep = ", ";
            }

            write!(f, "]")
        }
    }

    macro_rules! pretty {
        ($expr:expr) => {
            PrettySlice(&$expr)
        };
    }

    #[test]
    fn lsb_0_to_7_twice() {
        let bytes = [
            0b10_001_000,
            0b1_100_011_0,
            0b111_110_10,
            0b10_001_000,
            0b1_100_011_0,
            0b111_110_10,
        ];

        let mut buffer = [0; 16];

        render_from_lsb(&bytes, &mut buffer, 0);

        assert_eq!(pretty!(double_rainbow), pretty!(buffer));
    }

    #[test]
    fn msb_0_to_7_twice() {
        let bytes = [
            0b000_001_01,
            0b0_011_100_1,
            0b01_110_111,
            0b000_001_01,
            0b0_011_100_1,
            0b01_110_111,
        ];

        let mut buffer = [0; 16];

        render_from_msb(&bytes, &mut buffer, 0);

        assert_eq!(pretty!(double_rainbow), pretty!(buffer));
    }

    #[test]
    fn msb_0_to_4_then_1_bit() {
        let bytes = [0b000_001_01, 0b0_011_100_1];

        let mut buffer = [0; 32];

        render_from_msb(&bytes, &mut buffer, 0);

        assert_eq!(
            pretty!([
                BLUE, GREEN, RED, YELLOW, PURPLE, PURPLE, BLUE, GREY, GREEN, WHITE, RED, BLUE, RED,
                PURPLE, BLACK, GREEN, // and again
                BLUE, GREEN, RED, YELLOW, PURPLE, PURPLE, BLUE, GREY, GREEN, WHITE, RED, BLUE, RED,
                PURPLE, BLACK, GREEN
            ]),
            pretty!(buffer)
        );
    }
}
