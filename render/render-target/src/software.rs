//! A generic `PixelBuf` that can be drawn to and is blitted to screen by the
//! game, and a generic `PlayRenderer` for rendering the players view of the
//! level.

use sdl2::rect::Rect;
use sdl2::render::TextureCreator;
use sdl2::video::WindowContext;

pub use sdl2::render::Canvas;
pub use sdl2::video::Window;
pub use sdl2::{pixels, surface};
use super::{Buffer, RenderTarget};

/// A structure holding display data
pub struct SoftwareFramebuffer {
    crop_rect: Rect,
    tex_creator: TextureCreator<WindowContext>,
}

impl SoftwareFramebuffer {
    fn new(canvas: &Canvas<Window>) -> Self {
        let wsize = canvas.window().drawable_size();
        // let ratio = wsize.1 as f32 * 1.333;
        // let xp = (wsize.0 as f32 - ratio) / 2.0;

        let tex_creator = canvas.texture_creator();
        Self {
            // crop_rect: Rect::new(xp as i32, 0, ratio as u32, wsize.1),
            crop_rect: Rect::new(0, 0, wsize.0, wsize.1),
            tex_creator,
        }
    }
}

/// Backend-specific methods provided by the `RenderTarget` struct
impl RenderTarget {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            render_type: RenderType::Software(None),
            buffer: Buffer::new(width, height),
        }
    }

    pub fn with_software(mut self, canvas: &Canvas<Window>) -> Self {
        let software = SoftwareFramebuffer::new(canvas);
        self.render_type = RenderType::Software(Some(software));
        self
    }

    pub fn software(&mut self) -> Option<&mut SoftwareFramebuffer> {
        match self.render_type {
            RenderType::Software(ref mut s) => s.as_mut(),
            _ => None,
        }
    }

    /// # Safety
    ///
    /// The software framebuffer must not be `None`. Only use if software is
    /// used.
    pub unsafe fn software_unchecked(&mut self) -> &mut SoftwareFramebuffer {
        match self.render_type {
            RenderType::Software(ref mut s) => s.as_mut().unwrap_unchecked(),
            _ => panic!("Software framebuffer not set"),
        }
    }

    pub fn blit(&mut self, sdl_canvas: &mut Canvas<Window>) {
        let w = self.width() as u32;
        let h = self.height() as u32;
        let render_buffer = unsafe { self.software_unchecked() };
        let texc = &render_buffer.tex_creator;
        let surf = surface::Surface::from_data(
                &mut self.buffer.buffer,
                w,
                h,
                4 * w,
                pixels::PixelFormatEnum::RGBA32,
            )
            .unwrap()
            .as_texture(texc)
            .unwrap();
        sdl_canvas
            .copy(&surf, None, Some(render_buffer.crop_rect))
            .unwrap();
        sdl_canvas.present();
    }
}
