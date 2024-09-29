#[cfg(feature = "dummy-snd")]
#[path = "nosnd/src/lib.rs"]
pub mod nosnd;
#[cfg(feature = "dummy-snd")]
pub use nosnd::*;

#[cfg(feature = "sdl2-snd")]
#[path = "sdl2/src/lib.rs"]
pub mod sdl2;
#[cfg(feature = "sdl2-snd")]
pub use sdl2::*;
