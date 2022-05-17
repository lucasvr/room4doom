//! Display the end-of-level statistics for the player and the next level's name

use crate::defs::{
    animations, AnimType, Animation, Patches, State, MAP_POINTS, SHOW_NEXT_LOC_DELAY,
};
use gameplay::{m_random, TICRATE};
use gamestate_traits::{
    GameMode, GameTraits, MachinationTrait, MusTrack, PixelBuf, Scancode, WBPlayerStruct,
    WBStartStruct,
};
use log::warn;
use wad::{
    lumps::{WadPalette, WadPatch},
    WadData,
};

mod defs;
mod loc_state;
mod no_state;
mod stat_state;

const EP4_BG: &str = "INTERPIC";
const COMMERCIAL_BG: &str = "INTERPIC";
const TITLE_Y: i32 = 2;

pub struct Intermission {
    palette: WadPalette,
    bg_patches: Vec<WadPatch>,
    yah_patches: Vec<WadPatch>,
    /// 0 or 1 (left/right). Splat is 2
    yah_idx: usize,
    level_names: Vec<Vec<WadPatch>>,
    animations: Vec<Vec<Animation>>,
    current_bg: usize,
    /// General counter for animated BG
    bg_count: i32,
    mode: GameMode,
    // info updated by ticker
    player_info: WBPlayerStruct,
    level_info: WBStartStruct,

    pointer_on: bool,
    count: i32,
    state: State,
    /// General patches not specific to retail/commercial/registered
    patches: Patches,
}

impl Intermission {
    pub fn new(mode: GameMode, wad: &WadData) -> Self {
        let palette = wad.playpal_iter().next().unwrap();

        let mut level_names = Vec::new();
        let mut bg_patches = Vec::new();
        let mut yah_patches = Vec::new();
        if mode == GameMode::Commercial {
            let lump = wad.get_lump(COMMERCIAL_BG).unwrap();
            bg_patches.push(WadPatch::from_lump(lump));

            let mut names_patches = Vec::new();
            for m in 0..32 {
                let name = format!("CWILV{m:0>2}");
                let lump = wad.get_lump(&name).unwrap();
                names_patches.push(WadPatch::from_lump(lump));
            }
            level_names.push(names_patches);
        } else {
            for e in 0..3 {
                if mode == GameMode::Shareware && e > 0 {
                    break;
                }
                let lump = wad.get_lump(&format!("WIMAP{e}")).unwrap();
                bg_patches.push(WadPatch::from_lump(lump));

                let mut names_patches = Vec::new();
                for m in 0..9 {
                    let name = format!("WILV{e}{m}");
                    let lump = wad.get_lump(&name).unwrap();
                    names_patches.push(WadPatch::from_lump(lump));
                }
                level_names.push(names_patches);
            }

            let lump = wad.get_lump("WIURH0").unwrap();
            yah_patches.push(WadPatch::from_lump(lump));
            let lump = wad.get_lump("WIURH1").unwrap();
            yah_patches.push(WadPatch::from_lump(lump));
            let lump = wad.get_lump("WISPLAT").unwrap();
            yah_patches.push(WadPatch::from_lump(lump));
        }

        if mode == GameMode::Retail {
            let lump = wad.get_lump(EP4_BG).unwrap();
            bg_patches.push(WadPatch::from_lump(lump));

            let mut names_patches = Vec::new();
            for m in 0..9 {
                let name = format!("WILV3{m}");
                let lump = wad.get_lump(&name).unwrap();
                names_patches.push(WadPatch::from_lump(lump));
            }
            level_names.push(names_patches);
        }

        // load all the BG animations
        let mut anims = animations();
        for (e, anims) in anims.iter_mut().enumerate() {
            if mode == GameMode::Shareware && e > 0 {
                break;
            }
            for (l, anim) in anims.iter_mut().enumerate() {
                for i in 0..anim.num_of {
                    // episode, level, anim_num
                    if let Some(lump) = wad.get_lump(&format!("WIA{e}{l:0>2}{i:0>2}")) {
                        anim.patches.push(WadPatch::from_lump(lump));
                    } else if mode != GameMode::Commercial {
                        warn!("Missing WIA{e}{l:0>2}{i:0>2}");
                    }
                }
            }
        }

        Self {
            palette,
            bg_patches,
            level_names,
            animations: anims,
            yah_patches,
            yah_idx: 0,
            current_bg: 0,
            bg_count: 0,
            mode,
            player_info: WBPlayerStruct::default(),
            level_info: WBStartStruct::default(),
            pointer_on: true,
            count: SHOW_NEXT_LOC_DELAY * TICRATE,
            state: State::None,
            patches: Patches::new(wad),
        }
    }

    fn init_animated_bg(&mut self) {
        if self.mode == GameMode::Commercial || self.level_info.epsd > 2 {
            return;
        }

        for anim in self.animations[self.level_info.epsd as usize].iter_mut() {
            anim.counter = -1;
            // Next time to draw?
            match anim.kind {
                AnimType::Always => {
                    anim.next_tic = self.bg_count + 1 + (m_random() % anim.period);
                }
                AnimType::Random => {
                    anim.next_tic = self.bg_count + 1 + anim.data2 + (m_random() % anim.data1);
                }
                AnimType::Level => {
                    anim.next_tic = self.bg_count + 1;
                }
            }
        }
    }

    fn update_animated_bg(&mut self) {
        if self.mode == GameMode::Commercial || self.level_info.epsd > 2 {
            return;
        }

        for (i, anim) in self.animations[self.level_info.epsd as usize]
            .iter_mut()
            .enumerate()
        {
            if self.bg_count == anim.next_tic {
                match anim.kind {
                    AnimType::Always => {
                        anim.counter += 1;
                        if anim.counter >= anim.num_of {
                            anim.counter = 0;
                        }
                        anim.next_tic = self.bg_count + anim.period;
                    }
                    AnimType::Random => {
                        anim.counter += 1;
                        if anim.counter >= anim.num_of {
                            anim.counter = -1;
                            anim.next_tic = self.bg_count + anim.data2 + (m_random() % anim.data1);
                        } else {
                            anim.next_tic = self.bg_count + anim.period;
                        }
                    }
                    AnimType::Level => {
                        if !(self.state == State::StatCount && i == 7)
                            && self.level_info.next == anim.data1
                        {
                            anim.counter += 1;
                            if anim.counter == anim.num_of {
                                anim.counter -= 1;
                            }
                            anim.next_tic = self.bg_count + anim.period;
                        }
                    }
                }
            }
        }
    }

    fn draw_animated_bg(&self, buffer: &mut PixelBuf) {
        if self.mode == GameMode::Commercial || self.level_info.epsd > 2 {
            return;
        }

        for anim in self.animations[self.level_info.epsd as usize].iter() {
            if anim.counter >= 0 {
                self.draw_patch(
                    &anim.patches[anim.counter as usize],
                    anim.location.0,
                    anim.location.1,
                    buffer,
                );
            }
        }
    }
}

impl MachinationTrait for Intermission {
    fn init(&mut self, game: &impl GameTraits) {
        self.bg_count = 0;
        self.yah_idx = 0;
        self.current_bg = 0;
        self.pointer_on = true;
        self.state = State::None;

        self.player_info = game.player_end_info().clone();
        self.level_info = game.level_end_info().clone();
        self.current_bg = self.level_info.epsd as usize;

        // TODO: deathmatch stuff
        self.init_stats();
    }

    fn responder(&mut self, sc: Scancode, _game: &mut impl GameTraits) -> bool {
        if sc == Scancode::Return || sc == Scancode::Space {
            self.count = 0;
            return true;
        }
        false
    }

    fn ticker(&mut self, game: &mut impl GameTraits) -> bool {
        self.bg_count += 1;

        if self.bg_count == 1 {
            if self.mode == GameMode::Commercial {
                game.change_music(MusTrack::Dm2int);
            } else {
                game.change_music(MusTrack::Inter);
            }

            self.player_info.skills = if self.level_info.maxkills > 0 {
                (self.player_info.skills * 100) / self.level_info.maxkills
            } else {
                0
            };
            self.player_info.sitems = (self.player_info.sitems * 100) / self.level_info.maxitems;
            self.player_info.ssecret = (self.player_info.ssecret * 100) / self.level_info.maxsecret;
        }

        match self.state {
            State::StatCount => {
                self.update_stats();
            }
            State::NextLoc => {
                self.update_show_next_loc();
            }
            State::None => {
                self.update_no_state(game);
            }
        }

        false
    }

    fn get_palette(&self) -> &WadPalette {
        &self.palette
    }

    fn draw(&mut self, buffer: &mut PixelBuf) {
        // TODO: stats and next are two different screens.
        match self.state {
            State::StatCount => {
                self.draw_stats(buffer);
            }
            State::NextLoc => {
                self.draw_next_loc(buffer);
            }
            State::None => {
                self.draw_no_state(buffer);
            }
        }
    }
}
