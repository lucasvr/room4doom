use crate::{Intermission, State};
use game_traits::{GameTraits, PixelBuf};
use log::info;

impl Intermission {
    pub(super) fn draw_no_state(&mut self, buffer: &mut PixelBuf) {
        self.pointer_on = true;
        self.draw_next_loc(buffer);
    }

    pub(super) fn init_no_state(&mut self) {
        self.state = State::None;
        self.count = 10;
    }

    pub(super) fn update_no_state(&mut self, game: &mut impl GameTraits) {
        self.update_animated_bg();

        let player = &self.player_info;
        let level = &self.level_info;

        self.count -= 1;
        if self.count <= 0 {
            info!("Player: Total Items: {}/{}", player.sitems, level.maxitems);
            info!("Player: Total Kills: {}/{}", player.skills, level.maxkills);
            info!(
                "Player: Total Secrets: {}/{}",
                player.ssecret, level.maxsecret
            );
            info!("Player: Level Time: {}", player.stime);
            game.world_done();
        }
    }
}
