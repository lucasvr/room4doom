use gameplay::m_random;
use gamestate_traits::RenderTarget;
use render_traits::PixelBuffer;

#[derive(Debug)]
pub(crate) struct Wipe {
    y: Vec<i32>,
    height: i32,
    width: i32,
}

impl Wipe {
    pub(crate) fn new(width: i32, height: i32) -> Self {
        let mut y = Vec::with_capacity(width as usize);
        y.push(-(m_random() % 16));

        for i in 1..width as usize {
            let r = (m_random() % 3) - 1;
            y.push(y[i - 1] + r);
            if y[i] > 0 {
                y[i] = 0;
            } else if y[i] <= -16 {
                y[i] = -15;
            }
        }

        Self { y, height, width }
    }

    fn do_melt_pixels(
        &mut self,
        disp_buf: &mut impl PixelBuffer, // Display from this buffer
        draw_buf: &mut impl PixelBuffer, // Draw to this buffer
    ) -> bool {
        let mut done = true;
        let f = (disp_buf.height() / 200) as i32;
        for x in 0..self.width as usize {
            if self.y[x] < 0 {
                // This is the offset to start with, sort of like a timer
                self.y[x] += 1;
                done = false;
            } else if self.y[x] < self.height {
                let mut dy = if self.y[x] < (16 * f) {
                    self.y[x] + 1
                } else {
                    8 * f
                };
                if self.y[x] + dy >= self.height {
                    dy = self.height - self.y[x];
                }

                let mut y = self.y[x] as usize;
                for _ in (0..dy).rev() {
                    let px = draw_buf.read_softbuf_pixel(x, y);
                    disp_buf.set_pixel(x, y, (px.0, px.1, px.2, px.3));
                    y += 1;
                }
                self.y[x] += dy;

                for c in 0..=self.height - self.y[x] - dy {
                    let y = self.height - c - dy;
                    let px = disp_buf.read_softbuf_pixel(x, y as usize);
                    disp_buf.set_pixel(x, (self.height - c) as usize, (px.0, px.1, px.2, px.3));
                }
                done = false;
            }
        }
        done
    }

    pub(crate) fn do_melt(
        &mut self,
        disp_buf: &mut RenderTarget, // Display from this buffer
        draw_buf: &mut RenderTarget, // Draw to this buffer
    ) -> bool {
        match disp_buf.render_type() {
            render_traits::RenderType::Software => {
                let disp_buf = unsafe { disp_buf.software_unchecked() };
                let draw_buf = unsafe { draw_buf.software_unchecked() };
                self.do_melt_pixels(disp_buf, draw_buf)
            }
            render_traits::RenderType::SoftOpenGL => {
                let disp_buf = unsafe { disp_buf.soft_opengl_unchecked() };
                let draw_buf = unsafe { draw_buf.soft_opengl_unchecked() };
                self.do_melt_pixels(disp_buf, draw_buf)
            }
            render_traits::RenderType::OpenGL => todo!(),
            render_traits::RenderType::Vulkan => todo!(),
        }
    }
}
