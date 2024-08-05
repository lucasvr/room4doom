mod cheats;
mod cli;
mod config;
mod d_main;
mod timestep;
mod wipe;

use cli::*;
use config::MusicType;
use dirs::{cache_dir, data_dir};
use gamestate_traits::sdl2;
use std::env::set_var;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use d_main::d_doom_loop;
use env_logger::fmt::Color;
use gamestate::Game;

use crate::config::UserConfig;
use gameplay::log;
use input::Input;

use wad::WadData;

const BASE_DIR: &str = "room4doom/";

#[cfg(feature = "sdl2-snd")]
fn setup_timidity(user_config: &UserConfig, wad: &WadData) {
    use crate::log::{info, warn};

    const SOUND_DIR: &str = "room4doom/sound/";
    const TIMIDITY_CFG: &str = "timidity.cfg";

    let music_type = user_config.music_type;
    let gus_mem = user_config.gus_mem_size;

    if music_type == MusicType::FluidSynth {
        set_var("SDL_MIXER_DISABLE_FLUIDSYNTH", "0");
        info!("Using fluidsynth for sound");
        return;
    }
    if let Some(mut path) = data_dir() {
        path.push(SOUND_DIR);
        if path.exists() {
            let mut cache_dir = cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
            cache_dir.push(TIMIDITY_CFG);
            if let Some(cfg) = sound::timidity::make_timidity_cfg(wad, path, gus_mem) {
                let mut file = File::create(cache_dir.as_path()).unwrap();
                file.write_all(&cfg).unwrap();
                set_var("SDL_MIXER_DISABLE_FLUIDSYNTH", "1");
                set_var("TIMIDITY_CFG", cache_dir.as_path());
                info!("Using timidity for sound");
            } else {
                warn!("Sound fonts were missing, using fluidsynth instead");
            }
        } else {
            info!("No sound fonts installed to {:?}", path);
            info!("Using fluidsynth for sound");
        }
    }
}

#[cfg(not(feature = "sdl2-snd"))]
fn setup_timidity(user_config: &UserConfig, wad: &WadData) {
}

/// The main `game-exe` crate should take care of initialising a few things
fn main() -> Result<(), Box<dyn Error>> {
    let mut options: CLIOptions = argh::from_env();

    let mut logger = env_logger::Builder::new();
    logger
        .target(env_logger::Target::Stdout)
        .format(move |buf, record| {
            let mut style = buf.style();
            let colour = match record.level() {
                log::Level::Error => Color::Red,
                log::Level::Warn => Color::Yellow,
                log::Level::Info => Color::Green,
                log::Level::Debug => Color::Magenta,
                log::Level::Trace => Color::Magenta,
            };
            style.set_color(colour);

            if let Some(level) = options.verbose {
                if level == log::Level::Debug {
                    writeln!(
                        buf,
                        "{}: {}: {}",
                        style.value(record.level()),
                        record.target(),
                        record.args()
                    )
                } else {
                    //record.target().split("::").last().unwrap_or("")
                    writeln!(buf, "{}: {}", style.value(record.level()), record.args())
                }
            } else {
                writeln!(buf, "{}: {}", style.value(record.level()), record.args())
            }
        })
        .filter(None, options.verbose.unwrap_or(log::LevelFilter::Warn))
        .init();

    let mut user_config = UserConfig::load();

    let sdl_ctx = sdl2::init()?;
    let _snd_ctx = sdl_ctx.audio()?;
    let video_ctx = sdl_ctx.video()?;

    let events = sdl_ctx.event_pump()?;
    let input = Input::new(events, (&user_config.input).into());

    user_config.sync_cli(&mut options);
    user_config.write();

    let mut window = video_ctx
        .window("ROOM for DOOM", user_config.width, user_config.height)
        .fullscreen_desktop()
        .opengl()
        .build()?;
    let _gl_ctx = window.gl_create_context()?;
    let gl_ctx = unsafe {
        golem::Context::from_glow(golem::glow::Context::from_loader_function(|s| {
            video_ctx.gl_get_proc_address(s) as *const _
        }))
        .unwrap()
    };

    let gl_attr = video_ctx.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);

    let wad = WadData::new(user_config.iwad.clone().into());
    setup_timidity(&user_config, &wad);

    let game = Game::new(
        options.clone().into(),
        wad,
        user_config.sfx_vol,
        user_config.mus_vol,
    );

    if let Some(fullscreen) = options.fullscreen {
        if fullscreen {
            window.set_fullscreen(sdl2::video::FullscreenType::True)?;
        } else {
            window.set_fullscreen(sdl2::video::FullscreenType::Off)?;
        }
    }
    window.show();

    sdl_ctx.mouse().show_cursor(false);
    sdl_ctx.mouse().set_relative_mouse_mode(true);
    sdl_ctx.mouse().capture(true);

    d_doom_loop(game, input, window, gl_ctx, options)?;
    Ok(())
}
