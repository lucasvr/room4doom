#![feature(const_fn_floating_point_arithmetic)]

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use angle::Angle;
use glam::Vec2;
pub mod angle;
pub mod d_main;
pub mod d_thinker;
pub mod doom_def;
pub mod entities;
pub mod flags;
pub mod game;
pub mod info;
pub mod input;
pub mod level;
pub mod map_data;
pub mod p_enemy;
pub mod p_lights;
pub mod p_local;
pub mod p_map;
pub mod p_map_object;
pub mod p_player_sprite;
pub mod p_spec;
pub mod player;
pub mod r_bsp;
pub mod r_segs;
pub mod sounds;
pub mod tic_cmd;
pub mod timestep;

/// R_PointToDist
fn point_to_dist(x: f32, y: f32, to: Vec2) -> f32 {
    let mut dx = (x - to.x()).abs();
    let mut dy = (y - to.y()).abs();

    if dy > dx {
        let temp = dx;
        dx = dy;
        dy = temp;
    }

    let dist = (dx.powi(2) + dy.powi(2)).sqrt();
    dist
}

/// R_ScaleFromGlobalAngle
// All should be in rads
fn scale(
    visangle: Angle,
    rw_normalangle: Angle,
    rw_distance: f32,
    view_angle: Angle,
) -> f32 {
    static MAX_SCALEFACTOR: f32 = 64.0;
    static MIN_SCALEFACTOR: f32 = 0.00390625;

    let anglea = Angle::new(FRAC_PI_2 + visangle.rad() - view_angle.rad()); // CORRECT
    let angleb = Angle::new(FRAC_PI_2 + visangle.rad() - rw_normalangle.rad()); // CORRECT

    let sinea = anglea.sin(); // not correct?
    let sineb = angleb.sin();

    //            projection
    //m_iDistancePlayerToScreen = m_HalfScreenWidth / HalfFOV.GetTanValue();
    let p = 160.0 / (FRAC_PI_4).tan();
    let num = p * sineb; // oof a bit
    let den = rw_distance * sinea;

    let mut scale = num / den;

    if scale > MAX_SCALEFACTOR {
        scale = MAX_SCALEFACTOR;
    } else if MIN_SCALEFACTOR > scale {
        scale = MIN_SCALEFACTOR;
    }
    scale
}