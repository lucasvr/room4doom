//! Shooting and aiming.

use std::f32::consts::FRAC_PI_2;

use glam::Vec3;
use sound_traits::SfxName;

use crate::doom_def::{MAXRADIUS, MELEERANGE};
use crate::env::specials::shoot_special_line;
use crate::info::{StateNum, MOBJINFO};
use crate::level::map_data::BSPTrace;
use crate::level::map_defs::LineDef;
use crate::utilities::{p_random, path_traverse, point_to_angle_2, Intercept, PortalZ};
use crate::{Angle, LineDefFlags, MapObjKind, MapObject, MapPtr};

use super::{MapObjFlag, PT_ADDLINES, PT_ADDTHINGS};

impl MapObject {
    /// P_ExplodeMissile
    pub(crate) fn p_explode_missile(&mut self) {
        self.momxyz = Vec3::default();
        self.set_state(MOBJINFO[self.kind as usize].deathstate);

        self.tics -= p_random() & 3;

        if self.tics < 1 {
            self.tics = 1;
        }

        self.flags &= !(MapObjFlag::Missile as u32);

        if self.info.deathsound != SfxName::None {
            self.start_sound(self.info.deathsound);
        }
    }

    pub(crate) fn get_shoot_bsp_trace(&self, distance: f32) -> BSPTrace {
        let xy2 = self.xyz + self.angle.unit_vec3() * distance;
        // Use a radius for shooting to enable a sort of swept volume to capture more
        // subsectors as demons might overlap from a subsector that isn't caught
        // otherwise (for example demon might be in one subsector but overlap
        // with radius in to a subsector the bullet passes through).
        let mut bsp_trace = BSPTrace::new_line(self.xyz, xy2, 20.0);
        let mut count = 0;
        let level = unsafe { &mut *self.level };
        bsp_trace.find_intercepts(level.map_data.start_node(), &level.map_data, &mut count);
        bsp_trace
    }

    pub(crate) fn aim_line_attack(
        &mut self,
        distance: f32,
        bsp_trace: &mut BSPTrace,
    ) -> Option<AimResult> {
        let xy2 = self.xyz + self.angle.unit_vec3() * distance;

        // set up traverser
        let mut aim_traverse = SubSectTraverse::new(
            // can't shoot outside view angles
            100.0 / 160.0,
            -100.0 / 160.0,
            //
            distance,
            self.xyz.z + (self.height as i32 >> 1) as f32 + 8.0,
        );

        let level = unsafe { &mut *self.level };
        path_traverse(
            self.xyz,
            xy2,
            PT_ADDLINES | PT_ADDTHINGS,
            level,
            |t| aim_traverse.check_aim(self, t),
            bsp_trace,
        );

        aim_traverse.result()
    }

    /// `shoot_line_attack` is preceeded by `aim_line_attack` in many cases, so
    /// the `BSPTrace` can be shared between the two.
    pub(crate) fn shoot_line_attack(
        &mut self,
        attack_range: f32,
        angle: Angle,
        aim_slope: f32,
        damage: f32,
        bsp_trace: &mut BSPTrace,
    ) {
        let mut shoot_traverse = ShootTraverse::new(
            aim_slope,
            attack_range,
            damage,
            self.xyz.z + (self.height as i32 >> 1) as f32 + 8.0,
            bsp_trace.origin,
            angle.unit_vec3() * (bsp_trace.endpoint - bsp_trace.origin).length(),
            self.level().sky_num,
        );

        let xy2 = Vec3::new(
            self.xyz.x + attack_range * angle.cos(),
            self.xyz.y + attack_range * angle.sin(),
            self.xyz.z + aim_slope,
        );

        let level = unsafe { &mut *self.level };
        path_traverse(
            self.xyz,
            xy2,
            PT_ADDLINES | PT_ADDTHINGS,
            level,
            |intercept| shoot_traverse.resolve(self, intercept),
            bsp_trace,
        );
    }

    /// Source is the creature that caused the explosion at spot(self).
    ///
    /// Doom function name `P_RadiusAttack`
    pub fn radius_attack(&mut self, damage: f32) {
        // source is self.target
        // bsp_count is just for debugging BSP descent depth/width
        let mut bsp_count = 0;
        let dist = damage + MAXRADIUS;
        let mut bsp_trace = BSPTrace::new_radius(self.xyz, dist);

        let level = unsafe { &mut *self.level };
        bsp_trace.find_intercepts(level.map_data.start_node(), &level.map_data, &mut bsp_count);

        let sub_sectors = level.map_data.subsectors_mut();
        level.valid_count = level.valid_count.wrapping_add(1);
        for n in bsp_trace.intercepted_subsectors() {
            let ssect = &mut sub_sectors[*n as usize];

            // Check things in subsectors
            if !ssect.sector.run_mut_func_on_thinglist(|thing| {
                self.radius_damage_other(thing, damage, level.valid_count)
            }) {
                return;
            }
        }
    }

    /// Cause damage to other thing if in radius of self
    fn radius_damage_other(&mut self, other: &mut MapObject, damage: f32, valid: usize) -> bool {
        if other.valid_count == valid {
            return true;
        }
        other.valid_count = valid;

        if other.flags & MapObjFlag::Shootable as u32 == 0 {
            return true;
        }

        if matches!(other.kind, MapObjKind::MT_CYBORG | MapObjKind::MT_SPIDER) {
            return true;
        }

        // Could just use vector lengths but it changes Doom behaviour...
        let dx = (other.xyz.x - self.xyz.x).abs();
        let dy = (other.xyz.y - self.xyz.y).abs();
        let mut dist = if dx > dy {
            dx - other.radius - self.radius
        } else {
            dy - other.radius - self.radius
        };

        if dist < 0.0 {
            dist = 0.0;
        }

        if dist >= damage {
            return true; // out of range of blowy
        }

        if self.check_sight_target(other) {
            other.p_take_damage(None, None, false, (damage - dist) as i32);
        }
        true
    }

    pub(crate) fn bullet_slope(
        &mut self,
        distance: f32,
        bsp_trace: &mut BSPTrace,
    ) -> Option<AimResult> {
        let mut bullet_slope = self.aim_line_attack(distance, bsp_trace);
        let old_angle = self.angle;
        if bullet_slope.is_none() {
            self.angle += 5.625f32.to_radians();
            bullet_slope = self.aim_line_attack(distance, bsp_trace);
            if bullet_slope.is_none() {
                self.angle -= 11.25f32.to_radians();
                bullet_slope = self.aim_line_attack(distance, bsp_trace);
            }
        }
        self.angle = old_angle;

        bullet_slope
    }

    pub(crate) fn gun_shot(
        &mut self,
        accurate: bool,
        distance: f32,
        bullet_slope: Option<AimResult>,
        bsp_trace: &mut BSPTrace,
    ) {
        let damage = 5.0 * (p_random() % 3 + 1) as f32;
        let mut angle = self.angle;

        if !accurate {
            angle += (((p_random() - p_random()) >> 5) as f32).to_radians();
        }

        if let Some(res) = bullet_slope {
            self.shoot_line_attack(distance, angle, res.aimslope, damage, bsp_trace);
        } else {
            self.shoot_line_attack(distance, angle, 0.0, damage, bsp_trace);
        }
    }

    /// Try to attack along a line using the previous `AimResult` and
    /// `BSPTrace`.
    pub(crate) fn line_attack(
        &mut self,
        damage: f32,
        distance: f32,
        angle: Angle,
        bullet_slope: Option<AimResult>,
        bsp_trace: &mut BSPTrace,
    ) {
        if let Some(res) = bullet_slope {
            self.shoot_line_attack(distance, angle, res.aimslope, damage, bsp_trace);
        } else {
            self.shoot_line_attack(distance, angle, 0.0, damage, bsp_trace);
        }
    }

    /// Get a `BSPTrace` for the selected point. It uses the shooters radius to
    /// get visibility as opposed to using the victims radius (this should
    /// be changed to the reverse).
    ///
    /// The trace currently ignores the Z component as Doom is still 2D
    pub(crate) fn get_sight_bsp_trace(&self, xy2: Vec3) -> BSPTrace {
        // Use a radius for shooting to enable a sort of swept volume to capture more
        // subsectors as demons might overlap from a subsector that isn't caught
        // otherwise (for example demon might be in one subsector but overlap
        // with radius in to a subsector the bullet passes through).
        let mut bsp_trace = BSPTrace::new_line(self.xyz, xy2, self.radius);
        let mut count = 0;
        let level = unsafe { &mut *self.level };
        bsp_trace.find_intercepts(level.map_data.start_node(), &level.map_data, &mut count);
        bsp_trace
    }

    /// Check if there is a clear line of sight to the selected point.
    ///
    /// Note that this doesn't take in to account the radius of the point.
    pub(crate) fn check_sight(
        &mut self,
        to_xy: Vec3,
        to_z: f32,
        to_height: f32,
        bsp_trace: &mut BSPTrace,
    ) -> bool {
        let z_start = self.xyz.z + (self.height as i32 >> 1) as f32 + 8.0;
        let mut sight_traverse =
            SubSectTraverse::new(to_z + to_height - z_start, to_z - z_start, 0.0, z_start);

        let level = unsafe { &mut *self.level };
        path_traverse(
            self.xyz,
            to_xy,
            PT_ADDLINES,
            level,
            |t| sight_traverse.check_traverse(t),
            bsp_trace,
        )
    }

    /// Check the target is within a minimum distance
    fn target_within_min_dist(&self, target: &MapObject) -> bool {
        // skip the BSP trace if too far away
        let dist = self.xyz.distance_squared(target.xyz);
        // approx 1500.0 units
        dist < 2185300.3
    }

    /// Iterate through the available live players and check if there is a LOS
    /// to one.
    pub(crate) fn look_for_players(&mut self, all_around: bool) -> bool {
        let mut see = 0;
        let stop = (self.lastlook - 1) & 3;

        self.lastlook = (self.lastlook - 1) & 3;
        for _ in 0..self.lastlook {
            self.lastlook = (self.lastlook - 1) & 3;
            if !self.level().players_in_game()[self.lastlook as usize] {
                continue;
            }
            see += 1;
            if see == 2 || self.lastlook == stop {
                return false;
            }

            if self.level().players()[self.lastlook as usize].status.health <= 0 {
                continue;
            }

            if let Some(target) = self.level().players()[self.lastlook as usize].mobj() {
                // skip the BSP trace if too far away
                if !self.target_within_min_dist(target) {
                    // approx 1500.0 units
                    continue;
                }

                let xyz = target.xyz;
                let z = target.xyz.z;
                let height = target.height;

                let mut bsp_trace = self.get_sight_bsp_trace(xyz);
                if !self.check_sight(xyz, z, height, &mut bsp_trace) {
                    continue;
                }

                if !all_around {
                    let xy_u = point_to_angle_2(xyz, self.xyz).unit_vec2(); // Using a unit vector to remove world
                    let v1 = self.angle.unit_vec2(); // Get a unit from mobj angle
                    let angle = v1.angle_to(xy_u).abs(); // then use glam to get angle between (it's +/- for .abs())
                    if angle > FRAC_PI_2 && self.xyz.distance(xyz) > MELEERANGE {
                        continue;
                    }
                }
            }

            let last_look = self.lastlook as usize;
            self.target = self.level_mut().players_mut()[last_look]
                .mobj_mut()
                .map(|m| m.thinker);
            return true;
        }
        false
    }

    /// Check if there is a clear line of sight to the selected target object.
    /// This checks teh '2D top-down' nature of Doom, followed by the Z
    /// (height) axis.
    pub(crate) fn check_sight_target(&mut self, target: &MapObject) -> bool {
        // skip the BSP trace if too far away
        if !self.target_within_min_dist(target) {
            return false;
        }
        let mut bsp_trace = self.get_sight_bsp_trace(target.xyz);
        self.check_sight(target.xyz, target.xyz.z, target.height, &mut bsp_trace)
    }

    pub(crate) fn check_melee_range(&mut self) -> bool {
        if let Some(target) = self.target {
            let target = unsafe { (*target).mobj() };

            let dist = self.xyz.distance(target.xyz);
            if dist >= MELEERANGE - 20.0 + target.radius {
                return false;
            }

            let mut bsp_trace = self.get_sight_bsp_trace(target.xyz);
            if self.check_sight(target.xyz, target.xyz.z, target.height, &mut bsp_trace) {
                return true;
            }
        }
        false
    }

    /// The closer the Actor gets to the Target the more they shoot
    pub(crate) fn check_missile_range(&mut self) -> bool {
        if let Some(target) = self.target {
            let target = unsafe { (*target).mobj() };

            // skip the BSP trace if too far away
            if !self.target_within_min_dist(target) {
                return false;
            }

            let mut bsp_trace = self.get_sight_bsp_trace(target.xyz);
            if !self.check_sight(target.xyz, target.xyz.z, target.height, &mut bsp_trace) {
                return false;
            }

            // Was just attacked, fight back!
            if self.flags & MapObjFlag::Justhit as u32 != 0 {
                self.flags &= !(MapObjFlag::Justhit as u32);
                return true;
            }

            if self.reactiontime != 0 {
                return false; // do not attack yet
            }

            let mut dist = self.xyz.distance(target.xyz) - 64.0;

            if self.info.meleestate == StateNum::None {
                dist -= 128.0; // no melee attack, so fire more
            }

            if self.kind == MapObjKind::MT_VILE && dist > 14.0 * 64.0 {
                return false; // too far away
            }

            if self.kind == MapObjKind::MT_UNDEAD {
                if dist < 196.0 {
                    return false; // Close in to punch
                }
                dist /= 2.0;
            }

            if matches!(
                self.kind,
                MapObjKind::MT_CYBORG | MapObjKind::MT_SPIDER | MapObjKind::MT_SKULL
            ) {
                dist /= 2.0;
            }

            if dist > 200.0 {
                dist = 200.0;
            }

            if self.kind == MapObjKind::MT_CYBORG && dist > 160.0 {
                dist = 160.0;
            }

            // All down to chance now
            if p_random() >= dist as i32 {
                return true;
            }
        }
        false
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AimResult {
    pub aimslope: f32,
    pub line_target: MapPtr<MapObject>,
}

#[derive(Debug)]
struct SubSectTraverse {
    top_slope: f32,
    bot_slope: f32,
    attack_range: f32,
    shootz: f32,
    result: Option<AimResult>,
}

impl SubSectTraverse {
    fn new(top_slope: f32, bot_slope: f32, attack_range: f32, shootz: f32) -> Self {
        Self {
            top_slope,
            bot_slope,
            attack_range,
            shootz,
            result: None,
        }
    }

    fn set_slope(&mut self, line: &LineDef, portal: &PortalZ, dist: f32) {
        if let Some(backsector) = line.backsector.as_ref() {
            if line.frontsector.floorheight != backsector.floorheight {
                let slope = (portal.bottom_z - self.shootz) / dist;
                if slope > self.bot_slope {
                    self.bot_slope = slope;
                }
            }

            if line.frontsector.ceilingheight != backsector.ceilingheight {
                let slope = (portal.top_z - self.shootz) / dist;
                if slope < self.top_slope {
                    self.top_slope = slope;
                }
            }
        }
    }

    /// Returns false if the intercept blocks the target. Does not require
    /// `self.attack_range` to be set.
    fn check_traverse(&mut self, intercept: &mut Intercept) -> bool {
        if let Some(line) = intercept.line.as_mut() {
            // Check if solid line and stop
            if line.flags & LineDefFlags::TwoSided as u32 == 0 {
                return false;
            }

            let portal = PortalZ::new(line);
            if portal.bottom_z >= portal.top_z {
                return false;
            }

            let dist = intercept.frac;
            self.set_slope(line, &portal, dist);

            if self.top_slope <= self.bot_slope {
                return false;
            }

            return true;
        }

        false
    }

    /// After `check()` is called, a result should be checked for
    fn check_aim(&mut self, shooter: &mut MapObject, intercept: &mut Intercept) -> bool {
        if let Some(line) = intercept.line.as_mut() {
            // Check if solid line and stop
            if line.flags & LineDefFlags::TwoSided as u32 == 0 {
                return false;
            }

            let portal = PortalZ::new(line);
            if portal.bottom_z >= portal.top_z {
                return false;
            }

            let dist = self.attack_range * intercept.frac;
            self.set_slope(line, &portal, dist);

            if self.top_slope <= self.bot_slope {
                return false;
            }

            return true;
        } else if let Some(thing) = intercept.thing.as_mut() {
            // Don't shoot self
            if std::ptr::eq(shooter, thing.as_ref()) {
                return true;
            }
            // Corpse?
            if thing.flags & MapObjFlag::Shootable as u32 == 0 {
                return true;
            }

            let dist = self.attack_range * intercept.frac;
            let mut thing_top_slope = (thing.xyz.z + thing.height - self.shootz) / dist;
            if thing_top_slope < self.bot_slope {
                return true; // Shot over
            }

            let mut thing_bot_slope = (thing.xyz.z - self.shootz) / dist;
            if thing_bot_slope > self.top_slope {
                return true; // Shot below
            }

            if thing_top_slope > self.top_slope {
                thing_top_slope = self.top_slope;
            }
            if thing_bot_slope < self.bot_slope {
                thing_bot_slope = self.bot_slope;
            }

            self.result = Some(AimResult {
                aimslope: (thing_top_slope + thing_bot_slope) / 2.0,
                line_target: thing.clone(),
            });
        }

        false
    }

    fn result(&mut self) -> Option<AimResult> {
        self.result.take()
    }
}

struct ShootTraverse {
    aim_slope: f32,
    attack_range: f32,
    damage: f32,
    shootz: f32,
    trace_xyz: Vec3,
    trace_dxyz: Vec3,
    sky_num: usize,
}

impl ShootTraverse {
    fn new(
        aim_slope: f32,
        attack_range: f32,
        damage: f32,
        shootz: f32,
        trace_xyz: Vec3,
        trace_dxyz: Vec3,
        sky_num: usize,
    ) -> Self {
        Self {
            aim_slope,
            attack_range,
            damage,
            shootz,
            trace_xyz,
            trace_dxyz,
            sky_num,
        }
    }

    fn hit_line(&self, shooter: &mut MapObject, frac: f32, line: &LineDef) {
        let frac = frac - (4.0 / self.attack_range);
        let x = self.trace_xyz.x + self.trace_dxyz.x * frac;
        let y = self.trace_xyz.y + self.trace_dxyz.y * frac;
        let z = self.shootz + self.aim_slope * frac * self.attack_range;

        if line.frontsector.ceilingpic == self.sky_num {
            if z > line.frontsector.ceilingheight {
                return;
            } else if let Some(back) = line.backsector.as_ref() {
                if z > back.ceilingheight {
                    return;
                }
            }
        }

        MapObject::spawn_puff(x, y, z as i32, self.attack_range, unsafe {
            &mut *shooter.level
        });
    }

    fn resolve(&mut self, shooter: &mut MapObject, intercept: &mut Intercept) -> bool {
        if let Some(line) = intercept.line.as_mut() {
            // TODO: temporary, move this line to shoot traverse
            if line.special != 0 {
                shoot_special_line(line.clone(), shooter);
            }

            // Check if solid line and stop
            if line.flags & LineDefFlags::TwoSided as u32 == 0 {
                self.hit_line(shooter, intercept.frac, line);
                return false;
            }

            let portal = PortalZ::new(line);
            let dist = self.attack_range * intercept.frac;

            if let Some(backsector) = line.backsector.as_ref() {
                if line.frontsector.floorheight != backsector.floorheight {
                    let slope = (portal.bottom_z - self.shootz) / dist;
                    if slope > self.aim_slope {
                        self.hit_line(shooter, intercept.frac, line);
                        return false;
                    }
                }

                if line.frontsector.ceilingheight != backsector.ceilingheight {
                    let slope = (portal.top_z - self.shootz) / dist;
                    if slope < self.aim_slope {
                        self.hit_line(shooter, intercept.frac, line);
                        return false;
                    }
                }
            }

            return true;
        } else if let Some(thing) = intercept.thing.as_mut() {
            // Don't shoot self
            if std::ptr::eq(shooter, thing.as_ref()) {
                return true;
            }
            // Corpse?
            if thing.flags & MapObjFlag::Shootable as u32 == 0 {
                return true;
            }

            let dist = self.attack_range * intercept.frac;
            let thing_top_slope = (thing.xyz.z + thing.height - self.shootz) / dist;
            if thing_top_slope < self.aim_slope {
                return true; // Shot over
            }

            let thing_bot_slope = (thing.xyz.z - self.shootz) / dist;
            if thing_bot_slope > self.aim_slope {
                return true; // Shot below
            }

            let frac = intercept.frac - (10.0 / self.attack_range);
            let x = self.trace_xyz.x + self.trace_dxyz.x * frac;
            let y = self.trace_xyz.y + self.trace_dxyz.y * frac;
            let z = self.shootz + self.aim_slope * frac * self.attack_range;

            if thing.flags & MapObjFlag::Noblood as u32 != 0 {
                MapObject::spawn_puff(x, y, z as i32, self.attack_range, unsafe {
                    &mut *thing.level
                })
            } else {
                MapObject::spawn_blood(x, y, z as i32, self.damage, unsafe { &mut *thing.level });
            }

            if self.damage > 0.0 {
                thing.p_take_damage(None, Some(shooter), false, self.damage as i32);
                return false;
            }
        }

        false
    }
}
