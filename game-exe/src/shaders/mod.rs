use golem::GolemError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub mod basic;
pub mod cgwg_crt;
pub mod lottes_crt;
pub mod lottes_reduced;

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Serialize, Deserialize)]
pub enum Shaders {
    Lottes,
    LottesBasic,
    Cgwg,
    Basic,
    None,
}

impl Default for Shaders {
    fn default() -> Self {
        Self::Lottes
    }
}

impl FromStr for Shaders {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "lottes" => Ok(Shaders::Lottes),
            "lottesbasic" => Ok(Shaders::LottesBasic),
            "cgwg" => Ok(Shaders::Cgwg),
            "basic" => Ok(Shaders::Basic),
            "none" => Ok(Shaders::None),
            _ => Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Doh!")),
        }
    }
}

const GL_QUAD: [f32; 16] = [
    // position         vert_uv
    -1.0, -1.0, 0.0, 1.0, // bottom left
    1.0, -1.0, 1.0, 1.0, // bottom right
    1.0, 1.0, 1.0, 0.0, // top right
    -1.0, 1.0, 0.0, 0.0, // top left
];

const GL_QUAD_INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];

pub trait Drawer {
    fn clear(&self, ctx: &golem::Context) {
        ctx.set_clear_color(0.0, 0.0, 0.0, 1.0);
        ctx.clear();
    }

    fn set_tex_filter(&self) -> Result<(), GolemError>;

    /// The input buffer/image of Doom
    fn set_image_data(&mut self, input: &[u8], input_size: (u32, u32));

    fn draw(&mut self) -> Result<(), GolemError>;
}
