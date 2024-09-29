//! A generic `PixelBuf` that can be drawn to and is blitted to screen by the
//! game, and a generic `PlayRenderer` for rendering the players view of the
//! level.

#[cfg(feature = "softopengl")]
mod softopengl;
#[cfg(feature = "software")]
mod software;

#[cfg(feature = "softopengl")]
pub use softopengl::*;
#[cfg(feature = "software")]
pub use software::*;

use gameplay::{Level, PicData, Player};

const CHANNELS: usize = 4;

#[derive(Debug, Default, PartialEq, PartialOrd, Clone, Copy)]
pub enum RenderType {
    /// Purely software. Typically used with blitting a framebuffer maintained
    /// in memory directly to screen using SDL2
    #[cfg(feature = "software")]
    Software(Option<SoftFramebuffer>),
    /// Software framebuffer blitted to screen using OpenGL (and can use
    /// shaders)
    #[cfg(feature = "softopengl")]
    SoftOpenGL(Option<SoftOpenGL>),
    /// User-defined renderer.
    #[default]
    UserDefined,
    /// OpenGL
    OpenGL,
    /// Vulkan
    Vulkan,
}

pub trait PixelBuffer {
    fn size(&self) -> &BufferSize;
    fn clear(&mut self);
    fn clear_with_colour(&mut self, colour: &[u8; 4]);
    fn set_pixel(&mut self, x: usize, y: usize, rgba: &[u8; 4]);
    fn read_pixel(&self, x: usize, y: usize) -> [u8; 4];
    fn read_pixels(&mut self) -> &mut [u8];
}

pub struct BufferSize {
    width: usize,
    height: usize,
    width_i32: i32,
    height_i32: i32,
    width_f32: f32,
    height_f32: f32,
    half_width: i32,
    half_height: i32,
    half_width_f32: f32,
    half_height_f32: f32,
}

impl BufferSize {
    pub const fn width(&self) -> i32 {
        self.width_i32
    }

    pub const fn height(&self) -> i32 {
        self.height_i32
    }

    pub const fn half_width(&self) -> i32 {
        self.half_width
    }

    pub const fn half_height(&self) -> i32 {
        self.half_height
    }

    pub const fn width_usize(&self) -> usize {
        self.width
    }

    pub const fn height_usize(&self) -> usize {
        self.height
    }

    pub const fn width_f32(&self) -> f32 {
        self.width_f32
    }

    pub const fn height_f32(&self) -> f32 {
        self.height_f32
    }

    pub const fn half_width_f32(&self) -> f32 {
        self.half_width_f32
    }

    pub const fn half_height_f32(&self) -> f32 {
        self.half_height_f32
    }
}

pub struct Buffer {
    size: BufferSize,
    /// Total length is width * height * CHANNELS, where CHANNELS is RGB bytes
    buffer: Vec<u8>,
    stride: usize,
}

impl Buffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            size: BufferSize {
                width,
                height,
                width_i32: width as i32,
                height_i32: height as i32,
                half_width: width as i32 / 2,
                half_height: height as i32 / 2,
                width_f32: width as f32,
                height_f32: height as f32,
                half_width_f32: width as f32 / 2.0,
                half_height_f32: height as f32 / 2.0,
            },
            buffer: vec![0; (width * height) * CHANNELS],
            stride: width * CHANNELS,
        }
    }
}

impl PixelBuffer for Buffer {
    fn size(&self) -> &BufferSize {
        &self.size
    }

    fn clear(&mut self) {
        self.buffer
            .chunks_mut(4)
            .for_each(|n| n.copy_from_slice(&[0, 0, 0, 255]));
    }

    fn clear_with_colour(&mut self, colour: &[u8; 4]) {
        self.buffer
            .chunks_mut(4)
            .for_each(|n| n.copy_from_slice(colour));
    }

    fn set_pixel(&mut self, x: usize, y: usize, rgba: &[u8; 4]) {
        // Shitty safeguard. Need to find actual cause of fail
        #[cfg(feature = "safety_check")]
        if x >= self.size.width || y >= self.size.height {
            dbg!(x, y);
            panic!();
        }

        let pos = y * self.stride + x * CHANNELS;
        #[cfg(not(feature = "safety_check"))]
        unsafe {
            self.buffer
                .get_unchecked_mut(pos..pos + 4)
                .copy_from_slice(rgba);
        }
        #[cfg(feature = "safety_check")]
        self.buffer[pos..pos + 4].copy_from_slice(rgba);
    }

    /// Read the colour of a single pixel at X|Y

    fn read_pixel(&self, x: usize, y: usize) -> [u8; 4] {
        let pos = y * self.stride + x * CHANNELS;
        let mut slice = [0u8; 4];
        slice.copy_from_slice(&self.buffer[pos..pos + 4]);
        slice
    }

    /// Read the full buffer

    fn read_pixels(&mut self) -> &mut [u8] {
        &mut self.buffer
    }
}

/// A structure holding display data
pub struct RenderTarget {
    pub width: usize,
    pub height: usize,
    pub render_type: RenderType,
    pub buffer: Buffer,
}

impl RenderTarget {
    // TODO: should we return the pixelbuffer directly?
    pub fn pixel_buffer(&mut self) -> &mut dyn PixelBuffer {
        &mut self.buffer
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn render_type(&self) -> RenderType {
        self.render_type
    }
}

pub trait PlayRenderer {
    /// Drawing the full player view to the `PixelBuf`.
    ///
    /// Doom function name `R_RenderPlayerView`
    fn render_player_view(
        &mut self,
        player: &Player,
        level: &Level,
        pic_data: &mut PicData,
        buf: &mut RenderTarget,
    );
}

// TODO: somehow test with gl context
// #[cfg(test)]
// mod tests {
//     use crate::PixelBuf;

//     #[test]
//     fn write_read_pixel() {
//         let mut pixels = PixelBuf::new(320, 200, true);

//         pixels.set_pixel(10, 10, 255, 10, 3, 255);
//         pixels.set_pixel(319, 199, 25, 10, 3, 255);

//         let px = pixels.read_pixel(10, 10);
//         assert_eq!(px, (255, 10, 3, 0));

//         let px = pixels.read_pixel(319, 199);
//         assert_eq!(px, (25, 10, 3, 0));
//     }
// }
