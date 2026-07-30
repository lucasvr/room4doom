#[cfg(feature = "dummy-snd")]
#[path = "nosnd/src/lib.rs"]
pub mod nosnd;
#[cfg(feature = "dummy-snd")]
pub use nosnd::*;

#[cfg(any(feature = "sdl2-snd", feature = "rustysynth-snd"))]
#[path = "shared/info.rs"]
pub(crate) mod info;
#[cfg(any(feature = "sdl2-snd", feature = "rustysynth-snd"))]
#[path = "shared/mus2midi.rs"]
pub mod mus2midi;

#[cfg(feature = "sdl2-snd")]
#[path = "sdl2/src/lib.rs"]
pub mod sdl2;
#[cfg(feature = "sdl2-snd")]
pub use sdl2::*;

#[cfg(feature = "rustysynth-snd")]
#[path = "rustysynth/src/lib.rs"]
pub mod rustysynth;
#[cfg(feature = "rustysynth-snd")]
pub use rustysynth::*;
