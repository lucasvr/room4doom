//! Exposes an API of sorts that allows crates for things like statusbar and
//! intermission screens to get certain information they require or cause a
//! gamestate change.

pub mod util;

pub use gameplay::{
    m_random, AmmoType, Card, GameMode, PlayerCheat, PlayerStatus, PowerType, Skill,
    WBPlayerStruct, WBStartStruct, WeaponType, TICRATE, WEAPON_INFO,
};
pub use render_target::{PixelBuffer, RenderType};
pub use sdl2::keyboard::Scancode;
pub use sdl2::{self};
pub use sound_traits::{MusTrack, SfxName};

use wad::types::{WadPalette, WadPatch};
use wad::WadData;

/// The current state of the game-exe: whether we are playing, gazing at the
/// intermission screen, the game-exe final animation, or a demo.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum GameState {
    ForceWipe = -1,
    /// The state the game will spend most of its time in is `GameState::Level`
    /// as this is where all of the actual gameplay happens (including net +
    /// deathmatch play).
    Level,
    Intermission,
    Finale,
    /// The second most seen state is `GameState::Demo` which plays back
    /// recorded demos and is the default startup mode.
    Demo,
}

/// Universal game traits. To be implemented by the Game
pub trait GameTraits {
    /// Helper to start a new game, e.g, from menus
    fn defered_init_new(&mut self, skill: Skill, episode: usize, map: usize);

    /// A lot of things in Doom are dependant on knowing which of the game
    /// releases is currently being played. Commercial (Doom II) contains
    /// demons that Doom doesn't have, and Doom contains intermission
    /// screens that Doom II doesn't have (for example).
    fn get_mode(&self) -> GameMode;

    /// Ask the game to load this save
    fn load_game(&mut self, name: String);

    /// Ask the game to save to this slot with this name
    fn save_game(&mut self, name: String, slot: usize);

    /// Pauses the game-loop (generally stops gameplay input and thinkers
    /// running)
    fn toggle_pause_game(&mut self);

    /// Exit the game (there will be no confirmation)
    fn quit_game(&mut self);

    /// A basic sound starter
    fn start_sound(&mut self, sfx: SfxName);

    /// Change to or play this music track
    fn change_music(&self, music: MusTrack);

    /// Tell the game that the level is completed and the next level or state
    /// should begin
    fn level_done(&mut self);

    fn finale_done(&mut self);

    /// Fetch the end-of-level information
    fn level_end_info(&self) -> &WBStartStruct;

    /// Fetch the end-of-level player statistics (player 1)
    fn player_end_info(&self) -> &WBPlayerStruct;

    /// Fetch the basic player statistics (player 1)
    fn player_status(&self) -> PlayerStatus;

    /// Takes the player message waiting and replaces with None
    fn player_msg_take(&mut self) -> Option<String>;

    fn get_wad_data(&self) -> &WadData;

    // TODO: get and set settings Struct
}

/// To be implemented by machination type things (HUD, Map, Statusbar)
pub trait MachinationTrait {
    /// Possibly initialise the machination
    fn init(&mut self, game: &impl GameTraits);

    /// Return true if the responder took the event
    fn responder(&mut self, sc: Scancode, game: &mut impl GameTraits) -> bool;

    /// Responds to changes in the game or affects game.
    fn ticker(&mut self, game: &mut impl GameTraits) -> bool;

    fn get_palette(&self) -> &WadPalette;

    /// Draw this Machination to the `PixelBuf`.
    fn draw(&mut self, buffer: &mut dyn PixelBuffer);

    /// Free method, requires `get_palette()` to be implemented
    fn draw_patch_pixels(&self, patch: &WadPatch, x: i32, y: i32, pixels: &mut dyn PixelBuffer) {
        let mut xtmp = 0;
        let mut ytmp = 0;

        let f = pixels.size().height() / 200;
        for column in patch.columns.iter() {
            for n in 0..f {
                for p in column.pixels.iter() {
                    let colour = self.get_palette().0[*p];
                    for _ in 0..f {
                        pixels.set_pixel(
                            (x + xtmp - n - patch.left_offset as i32).unsigned_abs() as usize,
                            (y + ytmp + column.y_offset * f).unsigned_abs() as usize,
                            &colour.0,
                        );
                        ytmp += 1;
                    }
                }
                ytmp = 0;

                if column.y_offset == 255 {
                    xtmp += 1;
                }
            }
        }
    }
}
