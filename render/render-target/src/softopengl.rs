//! A generic `PixelBuf` that can be drawn to and is blitted to screen by the
//! game, and a generic `PlayRenderer` for rendering the players view of the
//! level.

pub mod shaders;

use golem::{ColorFormat, GolemError, Texture, TextureFilter};
use shaders::basic::Basic;
use shaders::cgwg_crt::Cgwgcrt;
use shaders::lottes_crt::LottesCRT;
use shaders::{ShaderDraw, Shaders};
use super::{Buffer, RenderTarget};

pub use golem::Context;
pub use shaders::Shaders;

/// A structure holding display data
pub struct SoftOpenGL {
    gl_texture: Texture,
    screen_shader: Box<dyn ShaderDraw>,
}

impl SoftOpenGL {
    fn new(width: usize, height: usize, gl_ctx: &Context, screen_shader: Shaders) -> Self {
        let mut gl_texture = Texture::new(gl_ctx).unwrap();
        gl_texture.set_image(None, width as u32, height as u32, ColorFormat::RGBA);

        Self {
            gl_texture,
            screen_shader: match screen_shader {
                Shaders::Basic => Box::new(Basic::new(gl_ctx)),
                Shaders::Lottes => Box::new(LottesCRT::new(gl_ctx)),
                Shaders::LottesBasic => Box::new(shaders::lottes_reduced::LottesCRT::new(gl_ctx)),
                Shaders::Cgwg => Box::new(Cgwgcrt::new(gl_ctx, width as u32, height as u32)),
            },
        }
    }

    pub const fn gl_texture(&self) -> &Texture {
        &self.gl_texture
    }

    pub fn set_gl_filter(&self) -> Result<(), GolemError> {
        self.gl_texture.set_minification(TextureFilter::Linear)?;
        self.gl_texture.set_magnification(TextureFilter::Linear)
    }

    pub fn copy_softbuf_to_gl_texture(&mut self, buffer: &Buffer) {
        self.gl_texture.set_image(
            Some(&buffer.buffer),
            buffer.size.width as u32,
            buffer.size.height as u32,
            ColorFormat::RGBA,
        );
    }
}

/// Backend-specific methods provided by the `RenderTarget` struct
impl RenderTarget {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            render_type: RenderType::SoftOpenGL(None),
            buffer: Buffer::new(width, height),
        }
    }

    pub fn with_gl(
        mut self,
        canvas: &Canvas<Window>,
        gl_ctx: &Context,
        screen_shader: Shaders,
    ) -> Self {
        let soft_opengl = SoftOpenGL::new(
            self.width,
            self.height,
            gl_ctx,
            screen_shader,
        );
        self.render_type = RenderType::SoftOpenGL(Some(soft_opengl));

        let wsize = canvas.window().drawable_size();
        let ratio = wsize.1 as f32 * 1.333;
        let xp = (wsize.0 as f32 - ratio) / 2.0;

        gl_ctx.set_viewport(xp as u32, 0, ratio as u32, wsize.1);
        self
    }

    pub fn soft_opengl(&mut self) -> Option<&mut SoftOpenGL> {
        match self.render_type {
            RenderType::SoftOpenGL(ref mut s) => s.as_mut(),
            _ => None,
        }
    }

    /// # Safety
    ///
    /// The opengl framebuffer must not be `None`. Only use if opengl is used.
    pub unsafe fn soft_opengl_unchecked(&mut self) -> &mut SoftOpenGL {
        match self.render_type {
            RenderType::SoftOpenGL(ref mut s) => s.as_mut().unwrap_unchecked(),
            _ => panic!("OpenGL framebuffer not set"),
        }
    }

    pub fn blit(&mut self, sdl_canvas: &mut Canvas<Window>) {
        let ogl = unsafe { self.soft_opengl_unchecked() };
        // shader.shader.clear();
        ogl.copy_softbuf_to_gl_texture(&self.buffer);
        ogl.screen_shader.draw(&ogl.gl_texture).unwrap();
        sdl_canvas.window().gl_swap_window();
    }
}
