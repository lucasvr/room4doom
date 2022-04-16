//! Doom source name `p_switch`

use log::{debug, error, warn};
use sound_sdl2::SndServerTx;
use sound_traits::{SfxEnum, SoundObjPosition};

use super::{
    ceiling::{ev_do_ceiling, CeilingKind},
    doors::{ev_do_door, ev_vertical_door, DoorKind},
    floor::{ev_build_stairs, ev_do_floor, FloorKind, StairKind},
    lights::ev_turn_light_on,
    mobj::MapObject,
    platforms::{ev_do_platform, PlatKind},
};

use crate::{
    doom_def::Card,
    level::{flags::LineDefFlags, map_defs::LineDef},
    pic::{Button, ButtonWhere},
    DPtr,
};

const BUTTONTIME: u32 = 35;

/// Doom function name `P_StartButton`
pub fn start_button(
    line: DPtr<LineDef>,
    bwhere: ButtonWhere,
    texture: usize,
    timer: u32,
    button_list: &mut Vec<Button>,
) {
    for b in button_list.iter() {
        if b.timer != 0 && b.line == line {
            return;
        }
    }

    for b in button_list.iter_mut() {
        // Re-use an existing one
        if b.timer == 0 {
            debug!("Re-using existing button struct for {:?}", line.as_ptr());
            b.line = line;
            b.bwhere = bwhere;
            b.texture = texture;
            b.timer = timer;
            // TODO: buttonlist[i].soundorg = &line->frontsector->soundorg;
            return;
        }
    }
    debug!("Using new button struct for {:?}", line.as_ptr());
    button_list.push(Button {
        line,
        bwhere,
        texture,
        timer,
    });
}

pub(crate) fn start_line_sound(line: &LineDef, sfx: SfxEnum, snd: &SndServerTx) {
    let sfx_origin = *line.v1 + *line.v1 - *line.v2;
    snd.send(sound_traits::SoundAction::StartSfx {
        origin: SoundObjPosition::new(
            line as *const LineDef as usize,
            (sfx_origin.x(), sfx_origin.y()),
            0.0,
        ),
        sfx,
    })
    .unwrap();
}

/// Doom function name `P_ChangeSwitchTexture`
pub fn change_switch_texture(
    mut line: DPtr<LineDef>,
    use_again: bool,
    switch_list: &[usize],
    button_list: &mut Vec<Button>,
    snd: &SndServerTx,
) {
    let mut sfx = SfxEnum::swtchx;
    if !use_again {
        line.special = 0;
        sfx = SfxEnum::swtchn;
    }

    let tex_top = line.front_sidedef.toptexture;
    let tex_mid = line.front_sidedef.midtexture;
    let tex_low = line.front_sidedef.bottomtexture;

    for i in 0..switch_list.len() {
        let sw = switch_list[i];
        if sw == tex_top {
            start_line_sound(&line, sfx, snd);
            line.front_sidedef.toptexture = switch_list[i ^ 1];
            if use_again {
                start_button(
                    line,
                    ButtonWhere::Top,
                    switch_list[i],
                    BUTTONTIME,
                    button_list,
                );
            }
            return;
        } else if sw == tex_mid {
            start_line_sound(&line, sfx, snd);
            line.front_sidedef.midtexture = switch_list[i ^ 1];
            if use_again {
                start_button(
                    line,
                    ButtonWhere::Middle,
                    switch_list[i],
                    BUTTONTIME,
                    button_list,
                );
            }
            return;
        } else if sw == tex_low {
            start_line_sound(&line, sfx, snd);
            line.front_sidedef.bottomtexture = switch_list[i ^ 1];
            if use_again {
                start_button(
                    line,
                    ButtonWhere::Bottom,
                    switch_list[i],
                    BUTTONTIME,
                    button_list,
                );
            }
            return;
        }
    }
}

/// P_UseSpecialLine
/// Called when a thing uses a special line.
/// Only the front sides of lines are usable.
pub fn p_use_special_line(_side: i32, line: DPtr<LineDef>, thing: &MapObject) -> bool {
    //  Switches that other things can activate
    if thing.player.is_none() {
        // never open secret doors
        if (line.flags as u32) & LineDefFlags::Secret as u32 != 0 {
            return false;
        }

        if let 1    // MANUAL DOOR RAISE
            | 32    // MANUAL BLUE
            | 33    // MANUAL RED
            | 34    // MANUAL YELLOW
            = line.special {
            // Nothing
        } else {
            return false;
        }
    }

    if thing.level.is_null() {
        panic!("Thing had a bad level pointer");
    }
    let level = unsafe { &mut *thing.level };
    match line.special {
        1        // Vertical Door
        | 26      // Blue Door/Locked
        | 27      // Yellow Door /Locked
        | 28      // Red Door /Locked

        | 31      // Manual door open
        | 32      // Blue locked door open
        | 33      // Red locked door open
        | 34      // Yellow locked door open

        | 117     // Blazing door raise
        | 118     // Blazing door open
        => {
            ev_vertical_door(line, thing, level);
        }
        11 => {
            change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            level.do_exit_level();
        }
        51 => {
            change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            level.do_secret_exit_level();
        }
        29 => {
            debug!("line-switch: vld_normal door!");
            if ev_do_door(line.clone(), DoorKind::Normal, level) {
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        50 => {
            debug!("line-switch: vld_close door!");
            if ev_do_door(line.clone(), DoorKind::Close, level) {
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        103 => {
            debug!("line-switch: vld_open door!");
            if ev_do_door(line.clone(), DoorKind::Open, level) {
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        111 => {
            debug!("line-switch: vld_blazeRaise door!");
            if ev_do_door(line.clone(), DoorKind::BlazeRaise, level) {
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        112 => {
            debug!("line-switch: vld_blazeOpen door!");
            if ev_do_door(line.clone(), DoorKind::BlazeOpen, level) {
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        113 => {
            debug!("line-switch: vld_blazeClose door!");
            if ev_do_door(line.clone(), DoorKind::BlazeClose, level) {
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        42 => {
            debug!("line-switch: vld_close door!");
            if ev_do_door(line.clone(), DoorKind::Close, level) {
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        61 => {
            debug!("line-switch: vld_open door!");
            if ev_do_door(line.clone(), DoorKind::Open, level) {
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        63 => {
            debug!("line-switch: vld_normal door!");
            if ev_do_door(line.clone(), DoorKind::Normal, level) {
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        114 => {
            debug!("line-switch: vld_blazeRaise door!");
            if ev_do_door(line.clone(), DoorKind::BlazeRaise, level) {
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        115 => {
            debug!("line-switch: vld_blazeOpen door!");
            if ev_do_door(line.clone(), DoorKind::BlazeOpen, level) {
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        116 => {
            debug!("line-switch: vld_blazeClose door!");
            if ev_do_door(line.clone(), DoorKind::BlazeClose, level) {
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        14 => {
            debug!("line-switch: raiseAndChange platform!");
            if ev_do_platform(line.clone(), PlatKind::RaiseAndChange,32, level){
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        15 => {
            debug!("line-switch: raiseAndChange platform!");
            if ev_do_platform(line.clone(), PlatKind::RaiseAndChange,24, level){
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        20 => {
            debug!("line-switch: raiseToNearestAndChange platform!");
            if ev_do_platform(line.clone(), PlatKind::RaiseToNearestAndChange,0, level){
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        21 => {
            debug!("line-switch: downWaitUpStay platform!");
            if ev_do_platform(line.clone(), PlatKind::DownWaitUpStay,0, level){
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        62 => {
            debug!("line-switch: downWaitUpStay platform!");
            if ev_do_platform(line.clone(), PlatKind::DownWaitUpStay, 1, level){
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        66 => {
            debug!("line-switch: raiseAndChange platform!");
            if ev_do_platform(line.clone(), PlatKind::RaiseAndChange, 24, level){
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        67 => {
            debug!("line-switch: raiseAndChange platform!");
            if ev_do_platform(line.clone(), PlatKind::RaiseAndChange, 32, level){
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        68 => {
            debug!("line-switch: raiseToNearestAndChange platform!");
            if ev_do_platform(line.clone(), PlatKind::RaiseToNearestAndChange, 0, level){
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        122 => {
            debug!("line-switch: blazeDWUS platform!");
            if ev_do_platform(line.clone(), PlatKind::BlazeDWUS, 0, level){
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        123 => {
            debug!("line-switch: blazeDWUS platform!");
            if ev_do_platform(line.clone(), PlatKind::BlazeDWUS, 0, level){
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        18 => {
            debug!("line-switch: raiseFloorToNearest floor!");
            if ev_do_floor(line.clone(), FloorKind::RaiseFloorToNearest, level){
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        23 => {
            debug!("line-switch: lowerFloorToLowest floor!");
            if ev_do_floor(line.clone(), FloorKind::LowerFloorToLowest, level){
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        71 => {
            debug!("line-switch: turboLower floor!");
            if ev_do_floor(line.clone(), FloorKind::TurboLower, level){
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        55 => {
            debug!("line-switch: raiseFloorCrush floor!");
            if ev_do_floor(line.clone(), FloorKind::RaiseFloorCrush, level){
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        101 => {
            debug!("line-switch: raiseFloor floor!");
            if ev_do_floor(line.clone(), FloorKind::RaiseFloor, level){
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        102 => {
            debug!("line-switch: lowerFloor floor!");
            if ev_do_floor(line.clone(), FloorKind::LowerFloor, level){
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        131 => {
            debug!("line-switch: lowerFloor floor!");
            if ev_do_floor(line.clone(), FloorKind::RaiseFloorTurbo, level){
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        140 => {
            debug!("line-switch: lowerFloor floor!");
            if ev_do_floor(line.clone(), FloorKind::RaiseFloor512, level){
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        45 => {
            debug!("line-switch: lowerFloor floor!");
            if ev_do_floor(line.clone(), FloorKind::LowerFloor, level){
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        60 => {
            debug!("line-switch: lowerFloorToLowest floor!");
            if ev_do_floor(line.clone(), FloorKind::LowerFloorToLowest, level){
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        64 => {
            debug!("line-switch: raiseFloor floor!");
            if ev_do_floor(line.clone(), FloorKind::RaiseFloor, level){
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        65 => {
            debug!("line-switch: raiseFloorCrush floor!");
            if ev_do_floor(line.clone(), FloorKind::RaiseFloorCrush, level){
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        69 => {
            debug!("line-switch: raiseFloorToNearest floor!");
            if ev_do_floor(line.clone(), FloorKind::RaiseFloorToNearest, level){
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        70 => {
            debug!("line-switch: turboLower floor!");
            if ev_do_floor(line.clone(), FloorKind::TurboLower, level){
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        132 => {
            debug!("line-switch: raiseFloorTurbo floor!");
            if ev_do_floor(line.clone(), FloorKind::RaiseFloorTurbo, level){
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        41 => {
            debug!("line-switch: lowerToFloor ceiling!");
            if ev_do_ceiling(line.clone(), CeilingKind::LowerToFloor, level){
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        49 => {
            debug!("line-switch: crushAndRaise ceiling!");
            if ev_do_ceiling(line.clone(), CeilingKind::CrushAndRaise, level){
                change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        43 => {
            debug!("line-switch: lowerToFloor ceiling!");
            if ev_do_ceiling(line.clone(), CeilingKind::LowerToFloor, level){
                change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
            }
        }
        138 => {
            debug!("line-switch: turn light on!");
            ev_turn_light_on(line.clone(), 255, level);
            change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
        }
        139 => {
            debug!("line-switch: turn light off!");
            ev_turn_light_on(line.clone(), 35, level);
            change_switch_texture(line, true, &level.switch_list, &mut level.button_list, &level.snd_command);
        }
        7 => {
            debug!(
                "line-switch #{}: build 8 stair steps",
                line.special
            );
            ev_build_stairs(line.clone(), StairKind::Build8, level);
            change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
        }
        127 => {
            debug!(
                "line-switch #{}: build 16 stair steps turbo",
                line.special
            );
            ev_build_stairs(line.clone(), StairKind::Turbo16, level);
            change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
        }
        9 => {
            error!("line-special #{}: EV_DoDonut not implemented", line.special);
            change_switch_texture(line, false, &level.switch_list, &mut level.button_list, &level.snd_command);
        }
        // BLUE KEY
        133 | 99 => {
            if let Some(player) = thing.player {
                let player = unsafe { &mut *player };
                let cards = player.cards;
                if cards[Card::Bluecard as usize] || cards[Card::Blueskull as usize] {
                    change_switch_texture(line.clone(), line.special == 99, &level.switch_list, &mut level.button_list, &level.snd_command);
                    ev_vertical_door(line, thing, level);
                    // TODO: p->message = DEH_String(PD_BLUEO);
                    //player.message = Some(BLUE)
			        if let Some(mobj) = player.mobj {
                        unsafe {
                            (*mobj).start_sound(SfxEnum::oof);
                        }
                    }
                }
            }
        }
        // RED KEY
        134 | 135 => {
            if let Some(player) = thing.player {
                let player = unsafe { &mut *player };
                let cards = player.cards;
                if cards[Card::Redcard as usize] || cards[Card::Redskull as usize] {
                    change_switch_texture(line.clone(), line.special == 134, &level.switch_list, &mut level.button_list, &level.snd_command);
                    ev_vertical_door(line, thing, level);
                    // TODO: p->message = DEH_String(PD_BLUEO);
			        if let Some(mobj) = player.mobj {
                        unsafe {
                            (*mobj).start_sound(SfxEnum::oof);
                        }
                    }
                }
            }
        }
        // YELLOW KEY
        136 | 137 => {
            if let Some(player) = thing.player {
                let player = unsafe { &mut *player };
                let cards = player.cards;
                if cards[Card::Yellowcard as usize] || cards[Card::Yellowskull as usize] {
                    change_switch_texture(line.clone(), line.special == 136, &level.switch_list, &mut level.button_list, &level.snd_command);
                    ev_vertical_door(line, thing, level);
                    // TODO: p->message = DEH_String(PD_BLUEO);
			        if let Some(mobj) = player.mobj {
                        unsafe {
                            (*mobj).start_sound(SfxEnum::oof);
                        }
                    }
                }
            }
        }
        _ => {
            warn!("Invalid or unimplemented line switch: {}", line.special);
        }
    }
    false
}
