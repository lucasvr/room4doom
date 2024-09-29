#[cfg(not(feature = "sdl2-input"))]
#[path = "dummy/src/lib.rs"]
pub mod dummy;
#[cfg(not(feature = "sdl2-input"))]
pub use dummy::*;

#[cfg(feature = "sdl2-input")]
#[path = "sdl2/src/lib.rs"]
pub mod sdl2;
#[cfg(feature = "sdl2-input")]
pub use sdl2::*;

pub mod config {
    use nanoserde::{DeRon, SerRon};
    use super::{MouseButton, Scancode};

    #[derive(Debug, Clone, DeRon, SerRon)]
    pub struct InputConfig {
        pub key_right: i32,
        pub key_left: i32,
        pub key_up: i32,
        pub key_down: i32,
        pub key_strafeleft: i32,
        pub key_straferight: i32,
        pub key_fire: i32,
        pub key_use: i32,
        pub key_strafe: i32,
        pub key_speed: i32,
        pub mousebfire: u8,
        pub mousebstrafe: u8,
        pub mousebforward: u8,
    }

    impl Default for InputConfig {
        fn default() -> Self {
            InputConfig {
                key_right: Scancode::Right as i32,
                key_left: Scancode::Left as i32,

                key_up: Scancode::W as i32,
                key_down: Scancode::S as i32,
                key_strafeleft: Scancode::A as i32,
                key_straferight: Scancode::D as i32,
                key_fire: Scancode::RCtrl as i32,
                key_use: Scancode::Space as i32,
                key_strafe: Scancode::RAlt as i32,
                key_speed: Scancode::LShift as i32,

                mousebfire: MouseButton::Left as u8,
                mousebstrafe: MouseButton::Middle as u8,
                mousebforward: MouseButton::Right as u8,
            }
        }
    }
}