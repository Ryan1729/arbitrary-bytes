use common::inner_common::BYTES;
use common::*;

#[inline]
pub fn update_and_render(framebuffer: &mut Framebuffer, state: &mut GameState, input: Input) {
    if state.need_first_render {
        render_from_msb(&BYTES, &mut framebuffer.buffer, state.byte_index);
        state.need_first_render = false;
        return;
    }

    if input.pressed_this_frame(Button::Right) {
        render_from_lsb(&BYTES, &mut framebuffer.buffer, state.byte_index);
    } else if input.pressed_this_frame(Button::Left) {
        render_from_msb(&BYTES, &mut framebuffer.buffer, state.byte_index);
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
                BLUE,
                GREEN,
                RED,
                YELLOW,
                PURPLE,
                PURPLE,
                BLUE,
                GREY,
                GREEN,
                WHITE,
                RED,
                BLUE,
                RED,
                PURPLE,
                BLACK,
                GREEN,
                // and again
                BLUE,
                GREEN,
                RED,
                YELLOW,
                PURPLE,
                PURPLE,
                BLUE,
                GREY,
                GREEN,
                WHITE,
                RED,
                BLUE,
                RED,
                PURPLE,
                BLACK,
                GREEN
            ]),
            pretty!(buffer)
        );
    }
}
