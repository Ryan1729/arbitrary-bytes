//in pixels
pub const SCREEN_WIDTH: usize = 256;
pub const SCREEN_HEIGHT: usize = 240;
pub const SCREEN_LENGTH: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

// reportedly colourblind friendly colours
// https://twitter.com/ea_accessible/status/968595073184092160
pub mod colours {
    pub const BLUE: u32 = 0xFFE15233;
    pub const GREEN: u32 = 0xFF6EB030;
    pub const RED: u32 = 0xFF4949DE;
    pub const YELLOW: u32 = 0xFF37B9FF;
    pub const PURPLE: u32 = 0xFF543353;
    pub const GREY: u32 = 0xFF8B7D5A;
    pub const GRAY: u32 = GREY;
    pub const WHITE: u32 = 0xFFEEEEEE;
    pub const BLACK: u32 = 0xFF222222;
}
pub use colours::*;

#[derive(Clone, Copy)]
pub enum Colour {
    Blue,
    Green,
    Red,
    Yellow,
    Purple,
    Grey,
    White,
    Black,
}
pub use Colour::*;

pub const COLOUR_COUNT: usize = 8;

impl From<Colour> for u32 {
    fn from(c: Colour) -> Self {
        match c {
            Blue => BLUE,
            Green => GREEN,
            Red => RED,
            Yellow => YELLOW,
            Purple => PURPLE,
            Grey => GREY,
            White => WHITE,
            Black => BLACK,
        }
    }
}

impl From<u32> for Colour {
    fn from(n: u32) -> Self {
        match n {
            BLUE => Blue,
            GREEN => Green,
            RED => Red,
            YELLOW => Yellow,
            PURPLE => Purple,
            GREY => Grey,
            WHITE => White,
            BLACK => Black,
            _ => Grey,
        }
    }
}

impl From<Colour> for usize {
    fn from(c: Colour) -> Self {
        match c {
            Blue => 0,
            Green => 1,
            Red => 2,
            Yellow => 3,
            Purple => 4,
            Grey => 5,
            White => 6,
            Black => 7,
        }
    }
}

impl From<usize> for Colour {
    fn from(n: usize) -> Self {
        match n {
            0 => Blue,
            1 => Green,
            2 => Red,
            3 => Yellow,
            4 => Purple,
            5 => Grey,
            6 => White,
            7 => Black,
            _ => Grey,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Appearance(pub u8);

impl Default for Appearance {
    fn default() -> Self {
        Appearance(EMPTY_APPEARANCE)
    }
}

use std::fmt;

impl fmt::Display for Appearance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Appearance({})", self.0)
    }
}

impl From<Appearance> for Colour {
    fn from(Appearance(n): Appearance) -> Self {
        match n & 0b111 {
            0 => Blue,
            1 => Green,
            2 => Red,
            3 => Yellow,
            4 => Purple,
            5 => Grey,
            6 => White,
            7 => Black,
            _ => Grey,
        }
    }
}

impl From<Colour> for Appearance {
    fn from(c: Colour) -> Self {
        Appearance(match c {
            Blue => 0,
            Green => 1,
            Red => 2,
            Yellow => 3,
            Purple => 4,
            Grey => 5,
            White => 6,
            Black => 7,
        })
    }
}

impl From<Appearance> for Shape {
    fn from(Appearance(n): Appearance) -> Self {
        match n & 0b11000 {
            0b01000 => FilledRectangle,
            0b10000 => Circle,
            0b11000 => FilledCircle,
            _ => Rectangle,
        }
    }
}

impl From<Shape> for Appearance {
    fn from(s: Shape) -> Self {
        Appearance(match s {
            FilledRectangle => 0b01000,
            Circle => 0b10000,
            FilledCircle => 0b11000,
            Rectangle => 0,
        })
    }
}

use std::ops::BitOr;

impl BitOr<Appearance> for Appearance {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Appearance(self.0 | rhs.0)
    }
}

impl BitOr<Shape> for Appearance {
    type Output = Self;

    fn bitor(self, rhs: Shape) -> Self {
        Appearance(self.0) | Appearance::from(rhs)
    }
}

impl BitOr<Colour> for Appearance {
    type Output = Self;

    fn bitor(self, rhs: Colour) -> Self {
        Appearance(self.0) | Appearance::from(rhs)
    }
}

impl BitOr<Colour> for Shape {
    type Output = Appearance;

    fn bitor(self, rhs: Colour) -> Self::Output {
        Appearance::from(self) | Appearance::from(rhs)
    }
}

impl BitOr<Shape> for Colour {
    type Output = Appearance;

    fn bitor(self, rhs: Shape) -> Self::Output {
        Appearance::from(self) | Appearance::from(rhs)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Rectangle,
    FilledRectangle,
    Circle,
    FilledCircle,
}
pub use Shape::*;

pub const SHAPE_COUNT: usize = 8;

impl From<usize> for Shape {
    fn from(n: usize) -> Self {
        match n {
            0 => FilledRectangle,
            1 => Circle,
            2 => FilledCircle,
            _ => Rectangle,
        }
    }
}

pub type Variety = u8;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

pub mod card {
    //in pixels
    pub const SPACING: u8 = 2;
    pub const WIDTH: u8 = 25;
    pub const HEIGHT: u8 = 35;

    pub fn grid_to_screen((x, y): (u8, u8)) -> (u8, u8) {
        (
            x.saturating_mul((WIDTH + SPACING) + SPACING),
            y.saturating_mul(HEIGHT / 2),
        )
    }

    pub fn screen_to_grid((x, y): (u8, u8)) -> (u8, u8) {
        (x / ((WIDTH + SPACING) + SPACING), y / (HEIGHT / 2))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use quickcheck::TestResult;

        quickcheck! {
            fn smaller_type_inversion(pos: (u8, u8)) -> TestResult {
                if pos.0 > 8 || pos.1 > 15 {
                    return TestResult::discard();
                }

                TestResult::from_bool(pos == screen_to_grid(grid_to_screen(pos)))
            }
        }
    }
}

pub const EMPTY_APPEARANCE: u8 = 255;
