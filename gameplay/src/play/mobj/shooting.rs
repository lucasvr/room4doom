use glam::Vec2;

use crate::{
    info::{SfxEnum, MOBJINFO},
    level::map_data::BSPTrace,
    play::{
        specials::shoot_special_line,
        utilities::{p_random, path_traverse, Intercept, MAXRADIUS},
    },
    DPtr, LineDefFlags, MapObject, MapObjectType,
};

use super::{MapObjectFlag, PT_ADDLINES, PT_ADDTHINGS};

impl MapObject {
    /// P_ExplodeMissile
    fn p_explode_missile(&mut self) {
        self.momxy = Vec2::default();
        self.z = 0.0;
        self.set_state(MOBJINFO[self.kind as usize].deathstate);

        self.tics -= p_random() & 3;

        if self.tics < 1 {
            self.tics = 1;
        }

        self.flags &= !(MapObjectFlag::Missile as u32);

        if self.info.deathsound != SfxEnum::None {
            // TODO: S_StartSound (mo, mo->info->deathsound);
        }
    }

    pub(crate) fn aim_line_attack(&mut self, distance: f32) -> Option<AimResult> {
        let xy2 = self.xy + self.angle.unit() * distance;

        // Use a radius for shooting to enable a sort of swept volume to capture more subsectors as
        // demons might overlap from a subsector that isn't caught otherwise (for example demon
        // might be in one subsector but overlap with radius in to a subsector the bullet passes through).
        // NOTE: experiment. The bsp trace works via splitting plane intersection so probably not required.
        let mut bsp_trace = BSPTrace::new_line(self.xy, xy2, 20.0);
        let mut count = 0;
        let level = unsafe { &mut *self.level };
        bsp_trace.find_intercepts(level.map_data.start_node(), &level.map_data, &mut count);
        //bsp_trace.nodes = level.map_data.get_nodes().iter().enumerate().map(|(i,_)| i as u16).collect();

        // set up traverser
        let mut aim_traverse = AimTraverse::new(
            // can't shoot outside view angles
            100.0 / 160.0,
            -100.0 / 160.0,
            //
            distance,
            self.z + (self.height as i32 >> 1) as f32 + 8.0,
        );

        path_traverse(
            self.xy,
            xy2,
            PT_ADDLINES | PT_ADDTHINGS,
            level,
            |t| aim_traverse.check(self, t),
            &mut bsp_trace,
        );

        aim_traverse.result()
    }

    /// Source is the creature that caused the explosion at spot(self).
    ///
    /// Doom functrion name `P_RadiusAttack`
    pub fn radius_attack(&mut self, damage: f32) {
        // source is self.target
        // bsp_count is just for debugging BSP descent depth/width
        let mut bsp_count = 0;
        let dist = damage + MAXRADIUS;
        let mut bsp_trace = BSPTrace::new_radius(self.xy, dist);

        let level = unsafe { &mut *self.level };
        bsp_trace.find_intercepts(level.map_data.start_node(), &level.map_data, &mut bsp_count);

        let sub_sectors = &mut level.map_data.subsectors;
        level.valid_count = level.valid_count.wrapping_add(1);
        for n in bsp_trace.intercepted_subsectors() {
            let ssect = &mut sub_sectors[*n as usize];

            // Check things in subsectors
            if !ssect.sector.run_func_on_thinglist(|thing| {
                if thing.valid_count == level.valid_count {
                    return true;
                }
                thing.valid_count = level.valid_count;

                if thing.flags & MapObjectFlag::Shootable as u32 == 0 {
                    return true;
                }

                if matches!(
                    thing.kind,
                    MapObjectType::MT_CYBORG | MapObjectType::MT_SPIDER
                ) {
                    return true;
                }

                // Could just use vector lengths but it changes Doom behaviour...
                let dx = (thing.xy.x() - self.xy.x()).abs();
                let dy = (thing.xy.y() - self.xy.y()).abs();
                let mut dist = if dx > dy {
                    dx - thing.radius - self.radius
                } else {
                    dy - thing.radius - self.radius
                };

                if dist < 0.0 {
                    dist = 0.0;
                }

                if dist >= damage {
                    return true; // out of range of blowy
                }

                // TODO: P_CheckSight, use the existing BSPTrace.
                thing.p_take_damage(None, None, false, (damage - dist) as i32);
                true
            }) {
                return;
            }
        }
    }
}

pub(crate) struct AimResult {
    pub aimslope: f32,
    pub line_target: DPtr<MapObject>,
}

struct AimTraverse {
    top_slope: f32,
    bot_slope: f32,
    attack_range: f32,
    shootz: f32,
    result: Option<AimResult>,
}

impl AimTraverse {
    fn new(top_slope: f32, bot_slope: f32, attack_range: f32, shootz: f32) -> Self {
        Self {
            top_slope,
            bot_slope,
            attack_range,
            shootz,
            result: None,
        }
    }

    /// After `check()` is called, a result should be checked for
    fn check(&mut self, shooter: &mut MapObject, intercept: &mut Intercept) -> bool {
        if let Some(line) = intercept.line.as_mut() {
            // TODO: temporary, move this line to shoot traverse
            shoot_special_line(line.clone(), shooter);

            // Check if solid line and stop
            if line.flags & LineDefFlags::TwoSided as u32 == 0 {
                return false;
            }

            return true;
        } else if let Some(thing) = intercept.thing.as_mut() {
            // Don't shoot self
            if std::ptr::eq(shooter, thing.as_ref()) {
                return true;
            }
            // Corpse?
            if thing.flags & MapObjectFlag::Shootable as u32 == 0 {
                return true;
            }

            let dist = self.attack_range * intercept.frac;
            let mut thing_top_slope = (thing.z + thing.height - self.shootz) / dist;
            if thing_top_slope < self.bot_slope {
                return true; // Shot over
            }

            let mut thing_bot_slope = (thing.z - self.shootz) / dist;
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
