use text::bytes_lines;

use crate::constants::*;
use features::log;
use std::cmp::{max, min};

#[cfg(not(test))]
macro_rules! test_dbg {
    ($($arg:tt)*) => {};
}

#[cfg(test)]
macro_rules! test_dbg {
    ($($arg:tt)*) => {dbg!($($arg)*)};
}

pub trait Dimensions {
    fn get() -> (usize, usize);
}

pub struct ScreenDim;

#[macro_export]
macro_rules! impl_dimensions {
    (($w: expr, $h: expr): $type: ty) => {
        impl Dimensions for $type {
            fn get() -> (usize, usize) {
                ($w, $h)
            }
        }
    };
}

impl_dimensions!((SCREEN_WIDTH, SCREEN_HEIGHT): ScreenDim);

//The type parameter lets us have Framebuffers of diferent dimensions
pub type Framebuffer = FramebufferInternal<ScreenDim>;

use std::marker::PhantomData;

pub struct FramebufferInternal<D: Dimensions> {
    pub buffer: Vec<u32>,
    _marker: std::marker::PhantomData<D>,
}

impl<D: Dimensions> Default for FramebufferInternal<D> {
    fn default() -> Self {
        let mut buffer = Vec::new();
        let (w, h) = D::get();
        buffer.resize(w * h, PALETTE[0]);

        FramebufferInternal {
            buffer,
            _marker: PhantomData,
        }
    }
}

impl<D: Dimensions> PartialEq for FramebufferInternal<D> {
    fn eq(&self, other: &FramebufferInternal<D>) -> bool {
        &self.buffer[..] == &other.buffer[..]
    }
}

impl<D: Dimensions> Eq for FramebufferInternal<D> {}

macro_rules! red {
    ($colour:expr) => {
        $colour & 0xFF
    };
}

macro_rules! green {
    ($colour:expr) => {
        ($colour & 0xFF_00) >> 8
    };
}

macro_rules! blue {
    ($colour:expr) => {
        ($colour & 0xFF_00_00) >> 16
    };
}

macro_rules! alpha {
    ($colour:expr) => {
        ($colour & 0xFF_00_00_00) >> 24
    };
}

macro_rules! colour {
    ($red:expr, $green:expr, $blue:expr, $alpha:expr) => {
        $red | $green << 8 | $blue << 16 | $alpha << 24
    };
}

macro_rules! set_alpha {
    ($colour:expr, $alpha:expr) => {
        ($colour & 0x00_FF_FF_FF) | $alpha << 24
    };
}

#[allow(dead_code)]
impl<D: Dimensions> FramebufferInternal<D> {
    pub fn new() -> FramebufferInternal<D> {
        Self::default()
    }

    pub fn xy_to_i<U1: Into<usize>, U2: Into<usize>>(x: U1, y: U2) -> usize {
        y.into().saturating_mul(D::get().0).saturating_add(x.into())
    }

    pub fn draw_filled_rect(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        colour: u32,
    ) {
        let one_past_right_edge = x + width;
        let one_past_bottom_edge = y + height;

        for current_y in y..one_past_bottom_edge {
            for current_x in x..one_past_right_edge {
                let i = Self::xy_to_i(current_x, current_y);
                if i < self.buffer.len() {
                    self.buffer[i] = colour;
                }
            }
        }
    }

    pub fn draw_rect(&mut self, x: usize, y: usize, width: usize, height: usize, colour: u32) {
        let one_past_right_edge = x + width;
        let one_past_bottom_edge = y + height;

        for current_y in y..one_past_bottom_edge {
            {
                let i = Self::xy_to_i(x, current_y);
                if i < self.buffer.len() {
                    self.buffer[i] = colour;
                }
            }

            {
                let i = Self::xy_to_i(one_past_right_edge - 1, current_y);
                if i < self.buffer.len() {
                    self.buffer[i] = colour;
                }
            }
        }

        for current_x in x..one_past_right_edge {
            {
                let i = Self::xy_to_i(current_x, y);
                if i < self.buffer.len() {
                    self.buffer[i] = colour;
                }
            }

            {
                let i = Self::xy_to_i(current_x, one_past_bottom_edge - 1);
                if i < self.buffer.len() {
                    self.buffer[i] = colour;
                }
            }
        }
    }

    pub fn draw_rect_with_shader<F>(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        shader: F,
    ) where
        F: Fn(usize, usize, usize, usize) -> u32,
    {
        let one_past_right_edge = x + width;
        let one_past_bottom_edge = y + height;

        for current_y in y..one_past_bottom_edge {
            {
                let i = Self::xy_to_i(x, current_y);
                if i < self.buffer.len() {
                    self.buffer[i] = shader(x, current_y, width, height);
                }
            }

            {
                let i = Self::xy_to_i(one_past_right_edge - 1, current_y);
                if i < self.buffer.len() {
                    self.buffer[i] = shader(x, current_y, width, height);
                }
            }
        }

        for current_x in x..one_past_right_edge {
            {
                let i = Self::xy_to_i(current_x, y);
                if i < self.buffer.len() {
                    self.buffer[i] = shader(current_x, y, width, height);
                }
            }

            {
                let i = Self::xy_to_i(current_x, one_past_bottom_edge - 1);
                if i < self.buffer.len() {
                    self.buffer[i] = shader(current_x, y, width, height);
                }
            }
        }
    }

    pub fn clear(&mut self) {
        for i in 0..self.buffer.len() {
            self.buffer[i] = 0;
        }
    }

    pub fn clear_to(&mut self, colour: u32) {
        for i in 0..self.buffer.len() {
            self.buffer[i] = colour;
        }
    }

    //see http://members.chello.at/~easyfilter/bresenham.html
    pub fn draw_crisp_circle(&mut self, x_mid: usize, y_mid: usize, radius: usize, colour: u32) {
        if x_mid < radius || y_mid < radius {
            return;
        }
        let mut r = radius as isize;
        let mut x = -r;
        let mut y = 0isize;
        let mut err = 2 - 2 * r; /* II. Quadrant */
        while {
            self.buffer
                [Self::xy_to_i((x_mid as isize - x) as usize, (y_mid as isize + y) as usize)] =
                colour; /*   I. Quadrant */
            self.buffer
                [Self::xy_to_i((x_mid as isize - y) as usize, (y_mid as isize - x) as usize)] =
                colour; /*  II. Quadrant */
            self.buffer
                [Self::xy_to_i((x_mid as isize + x) as usize, (y_mid as isize - y) as usize)] =
                colour; /* III. Quadrant */
            self.buffer
                [Self::xy_to_i((x_mid as isize + y) as usize, (y_mid as isize + x) as usize)] =
                colour; /*  IV. Quadrant */
            r = err;
            if r <= y {
                y += 1;
                err += y * 2 + 1; /* e_xy+e_y < 0 */
            }
            if r > x || err > y {
                x += 1;
                err += x * 2 + 1; /* e_xy+e_x > 0 or no 2nd y-step */
            }

            x < 0
        } {}
    }

    #[inline]
    //see https://stackoverflow.com/a/12016968/4496839
    pub fn blend(&mut self, i: usize, colour: u32) {
        let background = self.buffer[i];
        let alpha = alpha!(colour) + 1;
        let inv_alpha = 256 - alpha!(colour);
        self.buffer[i] = colour!(
            (alpha * red!(colour) + inv_alpha * red!(background)) >> 8,
            (alpha * green!(colour) + inv_alpha * green!(background)) >> 8,
            (alpha * blue!(colour) + inv_alpha * blue!(background)) >> 8,
            0xFF
        );
    }

    #[inline]
    pub fn blend_xy(&mut self, x: usize, y: usize, colour: u32) {
        self.blend(Self::xy_to_i(x, y), colour);
    }

    // from https://fgiesen.wordpress.com/2013/02/08/triangle-rasterization-in-practice/
    //"the result of [orient_2d] fits inside a (2p+2)-bit signed integer"
    // Solving for p = 8 gives 2 * 8 + 2 = 18 so we need at least a 32 bit number.
    fn orient_2d(ax: u8, ay: u8, bx: u8, by: u8, cx: u8, cy: u8) -> i32 {
        (bx as i32 - ax as i32) * (cy as i32 - ay as i32)
            - (by as i32 - ay as i32) * (cx as i32 - ax as i32)
    }

    // see parts 6, 7, and 8 of
    // https://fgiesen.wordpress.com/2013/02/17/optimizing-sw-occlusion-culling-index/
    pub fn draw_filled_triangle(
        &mut self,
        x0: u8,
        y0: u8,
        x1: u8,
        y1: u8,
        x2: u8,
        y2: u8,
        colour: u32,
    ) {
        //The rest of this function assumes the points are in in counter-clockwise order.
        let is_clockwise = Self::orient_2d(x0, y0, x1, y1, x2, y2) < 0;
        let (x1, y1, x2, y2) = if is_clockwise {
            (x2, y2, x1, y1)
        } else {
            (x1, y1, x2, y2)
        };

        // Compute triangle bounding box
        let min_x = min(x0, min(x1, x2));
        let min_y = min(y0, min(y1, y2));
        let max_x = max(x0, max(x1, x2));
        let max_y = max(y0, max(y1, y2));

        // Triangle setup
        let a_01 = y0 as i32 - y1 as i32;
        let b_01 = x1 as i32 - x0 as i32;
        let a_12 = y1 as i32 - y2 as i32;
        let b_12 = x2 as i32 - x1 as i32;
        let a_20 = y2 as i32 - y0 as i32;
        let b_20 = x0 as i32 - x2 as i32;

        // Barycentric coordinates at minX/minY corner
        let (px, py) = (min_x, min_y);
        let mut w0_row = Self::orient_2d(x1, y1, x2, y2, px, py);
        let mut w1_row = Self::orient_2d(x2, y2, x0, y0, px, py);
        let mut w2_row = Self::orient_2d(x0, y0, x1, y1, px, py);

        // Rasterize
        for py in min_y..=max_y {
            // Barycentric coordinates at start of row
            let mut w0 = w0_row;
            let mut w1 = w1_row;
            let mut w2 = w2_row;

            for px in min_x..=max_x {
                // If p is on or inside all edges, render pixel.
                if w0 | w1 | w2 >= 0 {
                    self.buffer[Self::xy_to_i(px as usize, py as usize)] = colour;
                }

                // One step to the right
                w0 += a_12;
                w1 += a_20;
                w2 += a_01;
            }

            // One row step
            w0_row += b_12;
            w1_row += b_20;
            w2_row += b_01;
        }
    }

    pub fn draw_filled_quad(
        &mut self,
        x0: u8,
        y0: u8,
        x1: u8,
        y1: u8,
        x2: u8,
        y2: u8,
        x3: u8,
        y3: u8,
        colour: u32,
    ) {
        self.draw_filled_triangle(x0, y0, x1, y1, x2, y2, colour);
        self.draw_filled_triangle(x1, y1, x2, y2, x3, y3, colour);
    }

    //see http://members.chello.at/easyfilter/bresenham.c
    pub fn draw_circle(&mut self, x_mid: usize, y_mid: usize, radius: usize, colour: u32) {
        if x_mid < radius || y_mid < radius {
            return;
        }
        let xm = x_mid as isize;
        let ym = y_mid as isize;

        /* II. quadrant from bottom left to top right */
        let mut x: isize = -(radius as isize);
        let mut y: isize = 0;

        let mut alpha;

        /* error of 1.step */
        let mut err: isize = 2 - 2 * (radius as isize);

        //equivalent to 2 * radius - 1
        let diameter = 1 - err;
        while {
            /* get blend value of pixel */
            alpha = 255 * isize::abs(err - 2 * (x + y) - 2) / diameter;

            {
                let new_colour = set_alpha!(colour, 255 - (alpha as u32));

                /*   I. Quadrant */
                self.blend_xy((xm - x) as usize, (ym + y) as usize, new_colour);
                /*  II. Quadrant */
                self.blend_xy((xm - y) as usize, (ym - x) as usize, new_colour);
                /* III. Quadrant */
                self.blend_xy((xm + x) as usize, (ym - y) as usize, new_colour);
                /*  IV. Quadrant */
                self.blend_xy((xm + y) as usize, (ym + x) as usize, new_colour);
            }

            /* remember values */
            let e2 = err;
            let x2 = x;

            /* x step */
            if err + y > 0 {
                alpha = 255 * (err - 2 * x - 1) / diameter;

                /* outward pixel */
                if alpha < 256 {
                    let new_colour = set_alpha!(colour, 255 - (alpha as u32));

                    self.blend_xy((xm - x) as usize, (ym + y + 1) as usize, new_colour);
                    self.blend_xy((xm - y - 1) as usize, (ym - x) as usize, new_colour);
                    self.blend_xy((xm + x) as usize, (ym - y - 1) as usize, new_colour);
                    self.blend_xy((xm + y + 1) as usize, (ym + x) as usize, new_colour);
                }
                x += 1;
                err += x * 2 + 1;
            }

            /* y step */
            if e2 + x2 <= 0 {
                alpha = 255 * (2 * y + 3 - e2) / diameter;

                /* inward pixel */
                if alpha < 256 {
                    let new_colour = set_alpha!(colour, 255 - (alpha as u32));
                    self.blend_xy((xm - x2 - 1) as usize, (ym + y) as usize, new_colour);
                    self.blend_xy((xm - y) as usize, (ym - x2 - 1) as usize, new_colour);
                    self.blend_xy((xm + x2 + 1) as usize, (ym - y) as usize, new_colour);
                    self.blend_xy((xm + y) as usize, (ym + x2 + 1) as usize, new_colour);
                }

                y += 1;
                err += y * 2 + 1;
            }
            x < 0
        } {}
    }

    pub fn draw_filled_circle(&mut self, x_mid: usize, y_mid: usize, radius: usize, colour: u32) {
        if x_mid < radius || y_mid < radius {
            return;
        }
        let xm = x_mid as isize;
        let ym = y_mid as isize;

        /* II. quadrant from bottom left to top right */
        let mut x: isize = -(radius as isize);
        let mut y: isize = 0;

        let mut alpha;

        /* error of 1.step */
        let mut err: isize = 2 - 2 * (radius as isize);

        //equivalent to 2 * radius - 1
        let diameter = 1 - err;
        while {
            /* get blend value of pixel */
            alpha = 255 * isize::abs(err - 2 * (x + y) - 2) / diameter;

            {
                let new_colour = set_alpha!(colour, 255 - (alpha as u32));

                /*   I. Quadrant */
                self.blend_xy((xm - x) as usize, (ym + y) as usize, new_colour);
                /*  II. Quadrant */
                self.blend_xy((xm - y) as usize, (ym - x) as usize, new_colour);
                /* III. Quadrant */
                self.blend_xy((xm + x) as usize, (ym - y) as usize, new_colour);
                /*  IV. Quadrant */
                self.blend_xy((xm + y) as usize, (ym + x) as usize, new_colour);
            }

            /* remember values */
            let e2 = err;
            let x2 = x;

            /* x step */
            if err + y > 0 {
                alpha = 255 * (err - 2 * x - 1) / diameter;

                /* outward pixel */
                if alpha < 256 {
                    let new_colour = set_alpha!(colour, 255 - (alpha as u32));

                    self.blend_xy((xm - x) as usize, (ym + y + 1) as usize, new_colour);
                    self.blend_xy((xm - y - 1) as usize, (ym - x) as usize, new_colour);
                    self.blend_xy((xm + x) as usize, (ym - y - 1) as usize, new_colour);
                    self.blend_xy((xm + y + 1) as usize, (ym + x) as usize, new_colour);
                }
                x += 1;
                err += x * 2 + 1;
            }

            /* y step */
            if e2 + x2 <= 0 {
                /* inward pixels */

                let mut current_x;
                let mut current_y;

                current_x = (xm - x2 - 1) as usize;
                current_y = (ym + y) as usize;
                while current_x > x_mid || current_y > y_mid {
                    self.buffer[Self::xy_to_i(current_x, current_y)] = colour;

                    current_x -= 1;
                    current_y -= 1;
                }

                current_x = (xm + y) as usize;
                current_y = (ym + x2 + 1) as usize;
                while current_x > x_mid || current_y < y_mid {
                    self.buffer[Self::xy_to_i(current_x, current_y)] = colour;

                    current_x -= 1;
                    current_y += 1;
                }

                current_x = (xm - y) as usize;
                current_y = (ym - x2 - 1) as usize;
                while current_x < x_mid || current_y > y_mid {
                    self.buffer[Self::xy_to_i(current_x, current_y)] = colour;

                    current_x += 1;
                    current_y -= 1;
                }

                current_x = (xm + x2 + 1) as usize;
                current_y = (ym - y) as usize;
                while current_x < x_mid || current_y < y_mid {
                    self.buffer[Self::xy_to_i(current_x, current_y)] = colour;

                    current_x += 1;
                    current_y += 1;
                }

                y += 1;
                err += y * 2 + 1;
            }

            x < 0
        } {}

        self.buffer[Self::xy_to_i(x_mid, y_mid)] = colour;
    }

    pub fn sspr(
        &mut self,
        sprite_x: u8,
        sprite_y: u8,
        sprite_w: u8,
        sprite_h: u8,
        display_x: u8,
        display_y: u8,
    ) {
        const S_WIDTH: usize = GFX_WIDTH as usize;
        const D_WIDTH: usize = SCREEN_WIDTH as usize;

        let s_w = sprite_w as usize;
        let s_h = sprite_h as usize;

        let s_x = sprite_x as usize;
        let s_y = sprite_y as usize;

        let d_x = display_x as usize;
        let d_y = display_y as usize;

        let d_x_max = d_x + s_w;
        let d_y_max = d_y + s_h;

        let mut current_s_y = s_y;
        for y in d_y..d_y_max {
            let mut current_s_x = s_x;
            for x in d_x..d_x_max {
                let colour = GFX[current_s_x + current_s_y * S_WIDTH] as usize;
                //make purple transparent
                if colour != 4 {
                    let index = x + y * D_WIDTH;
                    if index < self.buffer.len() {
                        self.buffer[index] = PALETTE[colour];
                    }
                }
                current_s_x += 1;
            }
            current_s_y += 1;
        }
    }

    pub fn sspr_flip_both(
        &mut self,
        sprite_x: u8,
        sprite_y: u8,
        sprite_w: u8,
        sprite_h: u8,
        display_x: u8,
        display_y: u8,
    ) {
        const S_WIDTH: usize = GFX_WIDTH as usize;
        const D_WIDTH: usize = SCREEN_WIDTH as usize;

        let s_w = sprite_w as usize;
        let s_h = sprite_h as usize;

        let s_x = sprite_x as usize;
        let s_y = sprite_y as usize;

        let d_x = display_x as usize;
        let d_y = display_y as usize;

        let d_x_max = d_x + s_w;
        let d_y_max = d_y + s_h;

        let mut current_s_y = s_y + s_h - 1;
        for y in d_y..d_y_max {
            let mut current_s_x = s_x + s_w - 1;
            for x in d_x..d_x_max {
                let colour = GFX[current_s_x + current_s_y * S_WIDTH] as usize;
                //make purple transparent
                if colour != 2 {
                    let index = x + y * D_WIDTH;
                    if index < self.buffer.len() {
                        self.buffer[index] = PALETTE[colour];
                    }
                }
                current_s_x -= 1;
            }
            current_s_y -= 1;
        }
    }

    pub fn spr(&mut self, sprite_number: u8, x: u8, y: u8) {
        let (sprite_x, sprite_y) = get_sprite_xy(sprite_number);
        self.sspr(sprite_x, sprite_y, SPRITE_SIZE, SPRITE_SIZE, x, y);
    }

    pub fn spr_flip_both(&mut self, sprite_number: u8, x: u8, y: u8) {
        let (sprite_x, sprite_y) = get_sprite_xy(sprite_number);
        self.sspr_flip_both(sprite_x, sprite_y, SPRITE_SIZE, SPRITE_SIZE, x, y);
    }

    pub fn print(&mut self, bytes: &[u8], x: u8, mut y: u8, colour: u8) {
        for line in bytes_lines(bytes) {
            self.print_line(line, x, y, colour);
            y = y.saturating_add(FONT_SIZE);
        }
    }

    pub fn print_line(&mut self, bytes: &[u8], mut x: u8, y: u8, colour: u8) {
        let mut bytes_iter = bytes.iter();

        while let Some(&c) = bytes_iter.next() {
            let (sprite_x, sprite_y) = get_char_xy(c);
            self.print_char_raw(sprite_x, sprite_y, FONT_SIZE, FONT_SIZE, x, y, colour);
            x = x.saturating_add(FONT_ADVANCE);
        }
    }

    pub fn print_line_raw(&mut self, bytes: &[u8], mut x: u8, y: u8, colour: u8) {
        for &c in bytes {
            let (sprite_x, sprite_y) = get_char_xy(c);
            self.print_char_raw(sprite_x, sprite_y, FONT_SIZE, FONT_SIZE, x, y, colour);
            x = x.saturating_add(FONT_ADVANCE);
        }
    }

    pub fn print_single_line_number(&mut self, number: usize, x: u8, y: u8, colour: u8) {
        self.print_line_raw(number.to_string().as_bytes(), x, y, colour);
    }

    pub fn print_char(&mut self, character: u8, x: u8, y: u8, colour: u8) {
        let (sprite_x, sprite_y) = get_char_xy(character);
        self.print_char_raw(sprite_x, sprite_y, FONT_SIZE, FONT_SIZE, x, y, colour);
    }

    fn print_char_raw(
        &mut self,
        sprite_x: u8,
        sprite_y: u8,
        sprite_w: u8,
        sprite_h: u8,
        display_x: u8,
        display_y: u8,
        colour: u8,
    ) {
        const S_WIDTH: usize = FONT_WIDTH as usize;
        const D_WIDTH: usize = SCREEN_WIDTH as usize;

        let s_w = sprite_w as usize;
        let s_h = sprite_h as usize;

        let s_x = sprite_x as usize;
        let s_y = sprite_y as usize;

        let d_x = display_x as usize;
        let d_y = display_y as usize;

        let d_x_max = d_x + s_w;
        let d_y_max = d_y + s_h;

        let mut current_s_y = s_y;
        for y in d_y..d_y_max {
            let mut current_s_x = s_x;
            for x in d_x..d_x_max {
                let foxt_pixel_colour = FONT[current_s_x + current_s_y * S_WIDTH] as usize;
                //make black transparent
                if foxt_pixel_colour != 0 {
                    let index = x + y * D_WIDTH;
                    if index < self.buffer.len() {
                        self.buffer[index] = PALETTE[colour as usize & 15];
                    }
                }
                current_s_x += 1;
            }
            current_s_y += 1;
        }
    }

    pub fn full_window(&mut self) {
        self.window(0, 0, SCREEN_WIDTH as u8, SCREEN_HEIGHT as u8);
    }

    pub fn center_half_window(&mut self) {
        self.window(
            SCREEN_WIDTH as u8 / 4,
            SCREEN_HEIGHT as u8 / 4,
            SCREEN_WIDTH as u8 / 2,
            SCREEN_HEIGHT as u8 / 2,
        );
    }

    pub fn window(&mut self, x: u8, y: u8, w: u8, h: u8) {
        self.nine_slice(WINDOW_TOP_LEFT, x, y, w, h);
    }

    pub fn button(&mut self, x: u8, y: u8, w: u8, h: u8) {
        self.nine_slice(BUTTON_TOP_LEFT, x, y, w, h);
    }

    pub fn button_hot(&mut self, x: u8, y: u8, w: u8, h: u8) {
        self.nine_slice(BUTTON_HOT_TOP_LEFT, x, y, w, h);
    }

    pub fn button_pressed(&mut self, x: u8, y: u8, w: u8, h: u8) {
        self.nine_slice(BUTTON_PRESSED_TOP_LEFT, x, y, w, h);
    }

    #[allow(non_snake_case)]
    pub fn nine_slice(&mut self, top_left: u8, x: u8, y: u8, w: u8, h: u8) {
        let TOP_LEFT: u8 = top_left;
        let TOP: u8 = TOP_LEFT + 1;
        let TOP_RIGHT: u8 = TOP + 1;

        let MIDDLE_LEFT: u8 = TOP_LEFT + SPRITES_PER_ROW;
        let MIDDLE: u8 = TOP + SPRITES_PER_ROW;
        let MIDDLE_RIGHT: u8 = TOP_RIGHT + SPRITES_PER_ROW;

        let BOTTOM_LEFT: u8 = MIDDLE_LEFT + SPRITES_PER_ROW;
        let BOTTOM: u8 = MIDDLE + SPRITES_PER_ROW;
        let BOTTOM_RIGHT: u8 = MIDDLE_RIGHT + SPRITES_PER_ROW;

        let after_left_corner = x.saturating_add(SPRITE_SIZE);
        let before_right_corner = x.saturating_add(w).saturating_sub(SPRITE_SIZE);

        let below_top_corner = y.saturating_add(SPRITE_SIZE);
        let above_bottom_corner = y.saturating_add(h).saturating_sub(SPRITE_SIZE);

        for fill_y in (below_top_corner..above_bottom_corner).step_by(SPRITE_SIZE as _) {
            for fill_x in (after_left_corner..before_right_corner).step_by(SPRITE_SIZE as _) {
                self.spr(MIDDLE, fill_x, fill_y);
            }
        }

        for fill_x in (after_left_corner..before_right_corner).step_by(SPRITE_SIZE as _) {
            self.spr(TOP, fill_x, y);
            self.spr(BOTTOM, fill_x, above_bottom_corner);
        }

        for fill_y in (below_top_corner..above_bottom_corner).step_by(SPRITE_SIZE as _) {
            self.spr(MIDDLE_LEFT, x, fill_y);
            self.spr(MIDDLE_RIGHT, before_right_corner, fill_y);
        }

        self.spr(TOP_LEFT, x, y);
        self.spr(TOP_RIGHT, before_right_corner, y);
        self.spr(BOTTOM_LEFT, x, above_bottom_corner);
        self.spr(BOTTOM_RIGHT, before_right_corner, above_bottom_corner);
    }

    #[allow(non_snake_case)]
    pub fn bottom_six_slice(&mut self, top_left: u8, x: u8, y: u8, w: u8, h: u8) {
        let TOP_LEFT: u8 = top_left;
        let TOP: u8 = TOP_LEFT + 1;
        let TOP_RIGHT: u8 = TOP + 1;

        let MIDDLE_LEFT: u8 = TOP_LEFT + SPRITES_PER_ROW;
        let MIDDLE: u8 = TOP + SPRITES_PER_ROW;
        let MIDDLE_RIGHT: u8 = TOP_RIGHT + SPRITES_PER_ROW;

        let BOTTOM_LEFT: u8 = MIDDLE_LEFT + SPRITES_PER_ROW;
        let BOTTOM: u8 = MIDDLE + SPRITES_PER_ROW;
        let BOTTOM_RIGHT: u8 = MIDDLE_RIGHT + SPRITES_PER_ROW;

        let after_left_corner = x.saturating_add(SPRITE_SIZE);
        let before_right_corner = x.saturating_add(w).saturating_sub(SPRITE_SIZE);

        let below_top_corner = y.saturating_add(SPRITE_SIZE);
        let above_bottom_corner = y.saturating_add(h).saturating_sub(SPRITE_SIZE);

        for fill_y in (below_top_corner..above_bottom_corner).step_by(SPRITE_SIZE as _) {
            for fill_x in (after_left_corner..before_right_corner).step_by(SPRITE_SIZE as _) {
                self.spr(MIDDLE, fill_x, fill_y);
            }
        }

        for fill_x in (after_left_corner..before_right_corner).step_by(SPRITE_SIZE as _) {
            self.spr(MIDDLE, fill_x, y);
            self.spr(BOTTOM, fill_x, above_bottom_corner);
        }

        for fill_y in (below_top_corner..above_bottom_corner).step_by(SPRITE_SIZE as _) {
            self.spr(MIDDLE_LEFT, x, fill_y);
            self.spr(MIDDLE_RIGHT, before_right_corner, fill_y);
        }

        self.spr(MIDDLE_LEFT, x, y);
        self.spr(MIDDLE_RIGHT, before_right_corner, y);
        self.spr(BOTTOM_LEFT, x, above_bottom_corner);
        self.spr(BOTTOM_RIGHT, before_right_corner, above_bottom_corner);
    }

    #[allow(non_snake_case)]
    fn three_slice(&mut self, left_edge: u8, x: u8, y: u8, w: u8) {
        let LEFT: u8 = left_edge;
        let MIDDLE: u8 = LEFT + 1;
        let RIGHT: u8 = MIDDLE + 1;

        let after_left_corner = x.saturating_add(SPRITE_SIZE);
        let before_right_corner = x.saturating_add(w).saturating_sub(SPRITE_SIZE);

        self.spr(LEFT, x, y);

        for fill_x in (after_left_corner..before_right_corner).step_by(SPRITE_SIZE as _) {
            self.spr(MIDDLE, fill_x, y);
        }

        self.spr(RIGHT, before_right_corner, y);
    }

    pub fn row(&mut self, x: u8, y: u8, w: u8) {
        self.three_slice(ROW_LEFT_EDGE, x, y, w);
    }

    pub fn row_hot(&mut self, x: u8, y: u8, w: u8) {
        self.three_slice(ROW_HOT_LEFT_EDGE, x, y, w);
    }

    pub fn row_pressed(&mut self, x: u8, y: u8, w: u8) {
        self.three_slice(ROW_PRESSED_LEFT_EDGE, x, y, w);
    }

    pub fn row_marker(&mut self, x: u8, y: u8, w: u8) {
        self.three_slice(ROW_MARKER_LEFT_EDGE, x, y, w);
    }

    pub fn checkbox(&mut self, x: u8, y: u8, checked: bool) {
        self.spr(
            if checked {
                checkbox::CHECKED
            } else {
                checkbox::UNCHECKED
            },
            x,
            y,
        );
    }

    pub fn checkbox_hot(&mut self, x: u8, y: u8, checked: bool) {
        self.spr(
            if checked {
                checkbox::HOT_CHECKED
            } else {
                checkbox::HOT_UNCHECKED
            },
            x,
            y,
        );
    }

    pub fn checkbox_pressed(&mut self, x: u8, y: u8, checked: bool) {
        self.spr(
            if checked {
                checkbox::PRESSED_CHECKED
            } else {
                checkbox::PRESSED_UNCHECKED
            },
            x,
            y,
        );
    }

    pub fn hexagon(&mut self, x: u8, y: u8, interior: u32, outline: u32) {
        for hex_y in 0..8 {
            for hex_x in 0..8 {
                self.hexagon_set_pixel(x, y, hex_x, hex_y, interior, outline);
            }
        }
    }

    pub fn hexagon_left(&mut self, x: u8, y: u8, interior: u32, outline: u32) {
        for hex_y in 0..8 {
            for hex_x in 0..4 {
                self.hexagon_set_pixel(x, y, hex_x, hex_y, interior, outline);
            }
        }
    }
    pub fn hexagon_right(&mut self, x: u8, y: u8, interior: u32, outline: u32) {
        for hex_y in 0..8 {
            for hex_x in 4..8 {
                self.hexagon_set_pixel(x, y, hex_x, hex_y, interior, outline);
            }
        }
    }

    pub fn hexagon_set_pixel(
        &mut self,
        x: u8,
        y: u8,
        hex_x: u8,
        hex_y: u8,
        interior: u32,
        outline: u32,
    ) {
        let c = Self::hexagon_match(HEXAGON[(hex_y * 8 + hex_x) as usize], interior, outline);
        if c > 0 {
            self.buffer[Self::xy_to_i(
                //if we don't `& 0b11` here then the hexagon is drawn 4 to the right of `x`
                //when the right half of the hexagon is drawn.
                x.wrapping_add(hex_x & 0b11),
                y.wrapping_add(hex_y),
            )] = c;
        }
    }

    pub fn hexagon_match(colour_spec: u8, interior: u32, outline: u32) -> u32 {
        match colour_spec {
            1 => interior,
            2 => outline,
            _ => 0,
        }
    }
}

#[cfg_attr(rustfmt, rustfmt_skip)]
pub const HEXAGON: [u8; 64] = [
    0, 0, 0, 0, 2, 2, 2, 2,
    0, 0, 0, 2, 1, 1, 1, 2,
    0, 0, 2, 1, 1, 1, 1, 2,
    0, 2, 1, 1, 1, 1, 1, 2,
    2, 1, 1, 1, 1, 1, 2, 0,
    2, 1, 1, 1, 1, 2, 0, 0,
    2, 1, 1, 1, 2, 0, 0, 0,
    2, 2, 2, 2, 0, 0, 0, 0,
];

pub fn get_sprite_xy(sprite_number: u8) -> (u8, u8) {
    (
        (sprite_number % SPRITES_PER_ROW) * SPRITE_SIZE,
        (sprite_number / SPRITES_PER_ROW) * SPRITE_SIZE,
    )
}

pub fn get_char_xy(sprite_number: u8) -> (u8, u8) {
    const SPRITES_PER_ROW: u8 = FONT_WIDTH as u8 / FONT_SIZE;

    (
        (sprite_number % SPRITES_PER_ROW) * FONT_SIZE,
        (sprite_number / SPRITES_PER_ROW) * FONT_SIZE,
    )
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

impl Rect {
    #[inline]
    pub fn point(&self) -> (u8, u8) {
        (self.x, self.y)
    }

    #[inline]
    pub fn dimensions(&self) -> (u8, u8) {
        (self.w, self.h)
    }
}

impl From<((u8, u8, u8, u8))> for Rect {
    #[inline]
    fn from((x, y, w, h): (u8, u8, u8, u8)) -> Self {
        Rect { x, y, w, h }
    }
}

impl From<Rect> for (u8, u8, u8, u8) {
    #[inline]
    fn from(Rect { x, y, w, h }: Rect) -> Self {
        (x, y, w, h)
    }
}

impl From<((u8, u8), (u8, u8))> for Rect {
    #[inline]
    fn from(((x, y), (w, h)): ((u8, u8), (u8, u8))) -> Self {
        Rect { x, y, w, h }
    }
}

impl From<Rect> for ((u8, u8), (u8, u8)) {
    #[inline]
    fn from(Rect { x, y, w, h }: Rect) -> Self {
        ((x, y), (w, h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use quickcheck::*;

    struct TinyDim;

    impl_dimensions!((8, 8): TinyDim);

    type Framebuffer = FramebufferInternal<TinyDim>;

    #[test]
    fn a_full_filled_quad_renders_correctly() {
        let mut framebuffer = Framebuffer::new();
        const C: u32 = PALETTE[1];
        assert_ne!(framebuffer.buffer[0], C); //precondiion

        let (w, h) = TinyDim::get();
        let (max_x, max_y) = (w as u8 - 1, h as u8 - 1);

        framebuffer.draw_filled_quad(0, 0, 0, max_y, max_x, 0, max_x, max_y, C);

        assert_eq!(framebuffer.buffer, vec![C; w * h]);
    }

    macro_rules! all_perms_quad_check {
        ($bytes: expr) => {
            //by hand
            quad_check(&[
                $bytes[0], $bytes[1], $bytes[2], $bytes[3], $bytes[4], $bytes[5], $bytes[6],
                $bytes[7],
            ]);
            quad_check(&[
                $bytes[0], $bytes[1], $bytes[2], $bytes[3], $bytes[6], $bytes[7], $bytes[4],
                $bytes[5],
            ]);
            quad_check(&[
                $bytes[0], $bytes[1], $bytes[4], $bytes[5], $bytes[2], $bytes[3], $bytes[6],
                $bytes[7],
            ]);
            quad_check(&[
                $bytes[0], $bytes[1], $bytes[4], $bytes[5], $bytes[6], $bytes[7], $bytes[2],
                $bytes[3],
            ]);
            quad_check(&[
                $bytes[0], $bytes[1], $bytes[6], $bytes[7], $bytes[2], $bytes[3], $bytes[4],
                $bytes[5],
            ]);
            quad_check(&[
                $bytes[0], $bytes[1], $bytes[6], $bytes[7], $bytes[4], $bytes[5], $bytes[2],
                $bytes[3],
            ]);

            // take the by hand section and:
            // replace 0 with x, 1 with y, then 2 with 0 and 3 with 1, then x with 2 and y with 3
            quad_check(&[
                $bytes[2], $bytes[3], $bytes[0], $bytes[1], $bytes[4], $bytes[5], $bytes[6],
                $bytes[7],
            ]);
            quad_check(&[
                $bytes[2], $bytes[3], $bytes[0], $bytes[1], $bytes[6], $bytes[7], $bytes[4],
                $bytes[5],
            ]);
            quad_check(&[
                $bytes[2], $bytes[3], $bytes[4], $bytes[5], $bytes[0], $bytes[1], $bytes[6],
                $bytes[7],
            ]);
            quad_check(&[
                $bytes[2], $bytes[3], $bytes[4], $bytes[5], $bytes[6], $bytes[7], $bytes[0],
                $bytes[1],
            ]);
            quad_check(&[
                $bytes[2], $bytes[3], $bytes[6], $bytes[7], $bytes[0], $bytes[1], $bytes[4],
                $bytes[5],
            ]);
            quad_check(&[
                $bytes[2], $bytes[3], $bytes[6], $bytes[7], $bytes[4], $bytes[5], $bytes[0],
                $bytes[1],
            ]);

            // take the by hand section and:
            // replace 0 with x, 1 with y, then 4 with 0 and 5 with 1, then x with 4 and y with 5
            quad_check(&[
                $bytes[4], $bytes[5], $bytes[2], $bytes[3], $bytes[0], $bytes[1], $bytes[6],
                $bytes[7],
            ]);
            quad_check(&[
                $bytes[4], $bytes[5], $bytes[2], $bytes[3], $bytes[6], $bytes[7], $bytes[0],
                $bytes[1],
            ]);
            quad_check(&[
                $bytes[4], $bytes[5], $bytes[0], $bytes[1], $bytes[2], $bytes[3], $bytes[6],
                $bytes[7],
            ]);
            quad_check(&[
                $bytes[4], $bytes[5], $bytes[0], $bytes[1], $bytes[6], $bytes[7], $bytes[2],
                $bytes[3],
            ]);
            quad_check(&[
                $bytes[4], $bytes[5], $bytes[6], $bytes[7], $bytes[2], $bytes[3], $bytes[0],
                $bytes[1],
            ]);
            quad_check(&[
                $bytes[4], $bytes[5], $bytes[6], $bytes[7], $bytes[0], $bytes[1], $bytes[2],
                $bytes[3],
            ]);

            // take the by hand section and:
            // replace 0 with x, 1 with y, then 6 with 0 and 7 with 1, then x with 6 and y with 7
            quad_check(&[
                $bytes[6], $bytes[7], $bytes[2], $bytes[3], $bytes[4], $bytes[5], $bytes[0],
                $bytes[1],
            ]);
            quad_check(&[
                $bytes[6], $bytes[7], $bytes[2], $bytes[3], $bytes[0], $bytes[1], $bytes[4],
                $bytes[5],
            ]);
            quad_check(&[
                $bytes[6], $bytes[7], $bytes[4], $bytes[5], $bytes[2], $bytes[3], $bytes[0],
                $bytes[1],
            ]);
            quad_check(&[
                $bytes[6], $bytes[7], $bytes[4], $bytes[5], $bytes[0], $bytes[1], $bytes[2],
                $bytes[3],
            ]);
            quad_check(&[
                $bytes[6], $bytes[7], $bytes[0], $bytes[1], $bytes[2], $bytes[3], $bytes[4],
                $bytes[5],
            ]);
            quad_check(&[
                $bytes[6], $bytes[7], $bytes[0], $bytes[1], $bytes[4], $bytes[5], $bytes[2],
                $bytes[3],
            ]);
        };
    }

    #[test]
    fn all_diamond_quads_render_correctly() {
        let (w, h) = TinyDim::get();
        let (max_x, max_y) = (w as u8 - 1, h as u8 - 1);
        let bytes = [
            max_x / 2,
            0,
            0,
            max_y / 2,
            max_x / 2,
            max_y,
            max_x,
            max_y / 2,
        ];

        all_perms_quad_check!(bytes);
    }

    #[test]
    fn a_quad_that_was_incorrectly_rendered_as_a_triangle_renders_correctly() {
        let (w, h) = TinyDim::get();
        let (max_x, max_y) = (w as u8 - 1, h as u8 - 1);

        let bytes = [max_x, 0, max_x, max_y / 4, 0, max_y / 2, max_x / 2, max_y];

        all_perms_quad_check!(bytes);
    }

    #[test]
    fn concave_crescent_quads_render_correctly() {
        let (w, h) = TinyDim::get();
        let (max_x, max_y) = (w as u8 - 1, h as u8 - 1);

        let bytes = [
            max_x / 2,
            0,
            0,
            max_y / 2,
            max_x / 2,
            max_y,
            max_x / 4,
            max_y / 2,
        ];

        all_perms_quad_check!(bytes);
    }

    fn quad_check(bytes: &[u8]) {
        let mut framebuffer = Framebuffer::new();
        const C: u32 = PALETTE[1];
        assert_ne!(framebuffer.buffer[0], C); //precondiions
        assert!(bytes.len() >= 8);

        framebuffer.draw_filled_quad(
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], C,
        );

        assert_eq!(
            framebuffer.buffer[Framebuffer::xy_to_i(bytes[0], bytes[1])],
            C,
            "bytes: {:?} at: {:?}",
            bytes,
            (bytes[0], bytes[1])
        );
        assert_eq!(
            framebuffer.buffer[Framebuffer::xy_to_i(bytes[2], bytes[3])],
            C,
            "bytes: {:?} at: {:?}",
            bytes,
            (bytes[2], bytes[3])
        );
        assert_eq!(
            framebuffer.buffer[Framebuffer::xy_to_i(bytes[4], bytes[5])],
            C,
            "bytes: {:?} at: {:?}",
            bytes,
            (bytes[4], bytes[5])
        );
        assert_eq!(
            framebuffer.buffer[Framebuffer::xy_to_i(bytes[6], bytes[7])],
            C,
            "bytes: {:?} at: {:?}",
            bytes,
            (bytes[6], bytes[7])
        );
    }
}

pub fn get_text_dimensions(bytes: &[u8]) -> (u8, u8) {
    let mut width: u8 = 0;
    let mut height: u8 = 0;
    for line in bytes_lines(bytes) {
        height = height.saturating_add(1);
        width = max(width, line.len() as u8);
    }

    width = width.saturating_mul(FONT_ADVANCE);
    height = height.saturating_mul(FONT_SIZE);

    (width, height)
}

pub fn center_line_in_rect<R: Into<Rect>>(text_length: u8, r: R) -> (u8, u8) {
    let Rect { x, y, w, h } = r.into();
    let middle_x = x + (w / 2);
    let middle_y = y + (h / 2);

    let text_x =
        (middle_x as usize).saturating_sub(text_length as usize * FONT_ADVANCE as usize / 2) as u8;
    let text_y = (middle_y as usize).saturating_sub(FONT_SIZE as usize / 2) as u8;

    (text_x, text_y)
}

pub fn center_rect_in_rect<R: Into<Rect>>((width, height): (u8, u8), r: R) -> (u8, u8) {
    let Rect { x, y, w, h } = r.into();
    let middle_x = x + (w / 2);
    let middle_y = y + (h / 2);

    let left_x = middle_x.saturating_sub(width / 2);
    let top_y = middle_y.saturating_sub(height / 2);

    (left_x, top_y)
}

#[cfg(test)]
mod text_tests {
    use super::*;
    use quickcheck::*;

    #[test]
    fn test_get_text_dimensions_then_center_rect_in_rect_matches_center_line_in_rect_for_a_single_line(
    ) {
        quickcheck(
                    get_text_dimensions_then_center_rect_in_rect_matches_center_line_in_rect_for_a_single_line
                        as fn(u8, (u8, u8, u8, u8)) -> TestResult,
                )
    }
    fn get_text_dimensions_then_center_rect_in_rect_matches_center_line_in_rect_for_a_single_line(
        char_count: u8,
        r: (u8, u8, u8, u8),
    ) -> TestResult {
        if char_count as usize * FONT_ADVANCE as usize > 255 {
            return TestResult::discard();
        }

        let rect: Rect = r.into();

        let line_point = center_line_in_rect(char_count, rect);

        let text = vec![b'A'; char_count as usize];

        let text_point = center_rect_in_rect(get_text_dimensions(&text), rect);
        assert_eq!(text_point, line_point);
        TestResult::from_bool(text_point == line_point)
    }

    #[test]
    fn test_center_rect_in_rect_actually_centers_when_possible() {
        quickcheck(
            center_rect_in_rect_actually_centers_when_possible
                as fn(((u8, u8), (u8, u8, u8, u8))) -> TestResult,
        )
    }
    fn center_rect_in_rect_actually_centers_when_possible(
        ((w, h), r): ((u8, u8), (u8, u8, u8, u8)),
    ) -> TestResult {
        let rect: Rect = r.into();

        if rect.w & 1 == 1 || w & 1 == 1 {
            return TestResult::discard();
        }

        let (x, _y) = center_rect_in_rect((w, h), rect);
        let left_side = x.saturating_sub(rect.x);
        let right_side = (rect.x + rect.w).saturating_sub(x + w);

        assert_eq!(left_side, right_side);
        TestResult::from_bool(left_side == right_side)
    }

    #[test]
    fn test_center_line_in_rect_actually_centers_when_possible() {
        assert!(FONT_ADVANCE & 1 == 0);
        quickcheck(
            center_line_in_rect_actually_centers_when_possible
                as fn((u8, (u8, u8, u8, u8))) -> TestResult,
        )
    }
    fn center_line_in_rect_actually_centers_when_possible(
        (length, r): (u8, (u8, u8, u8, u8)),
    ) -> TestResult {
        let rect: Rect = r.into();

        if rect.w & 1 == 1 || rect.w < FONT_ADVANCE || length >= (256 / FONT_ADVANCE as usize) as u8
        {
            return TestResult::discard();
        }
        let w = length * FONT_ADVANCE;

        let (x, _y) = center_line_in_rect(length, rect);
        let left_side = (x as usize).saturating_sub(rect.x as usize);
        let right_side =
            (rect.x as usize + rect.w as usize).saturating_sub(x as usize + w as usize);

        assert_eq!(left_side, right_side);
        TestResult::from_bool(left_side == right_side)
    }
}
