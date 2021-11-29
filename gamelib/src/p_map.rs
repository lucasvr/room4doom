//!	Movement, collision handling.
//!	Shooting and aiming.
use std::f32::consts::{FRAC_2_PI, FRAC_PI_2};

use glam::Vec2;

use crate::angle::Angle;
use crate::flags::LineDefFlags;
use crate::level_data::level::Level;
use crate::level_data::map_data::BSPTrace;
use crate::level_data::map_defs::{BBox, LineDef, SlopeType};
use crate::p_local::MAXRADIUS;
use crate::p_map_object::{MapObject, MapObjectFlag};
use crate::p_map_util::{PortalZ, box_on_line_side, line_slide_direction};
use crate::DPtr;

const MAXSPECIALCROSS: i32 = 8;

/// The pupose of this struct is to record the highest and lowest points in a
/// subsector. When a mob crosses a seg it may be between floor/ceiling heights.
#[derive(Default)]
pub struct SubSectorMinMax {
    tmflags: u32,
    /// If "floatok" true, move would be ok
    /// if within "tmfloorz - tmceilingz".
    floatok: bool,
    min_floor_z: f32,
    max_ceil_z: f32,
    max_dropoff: f32,
    spec_hits: Vec<DPtr<LineDef>>,
}

impl MapObject {
    /// P_TryMove, merged with P_CheckPosition and using a more verbose/modern collision
    pub fn p_try_move(&mut self, ptryx: f32, ptryy: f32, level: &mut Level) -> bool {
        // P_CrossSpecialLine
        level.mobj_ctrl.floatok = false;

        let try_move = Vec2::new(ptryx, ptryy);

        level.mobj_ctrl.spec_hits.clear();
        level.mobj_ctrl.floatok = true;
        if !self.p_check_position(self.xy, try_move, level) {
            return false;
        }

        let ctrl = &mut level.mobj_ctrl;
        if self.flags & MapObjectFlag::MF_NOCLIP as u32 == 0 {
            if ctrl.max_ceil_z - ctrl.min_floor_z < self.height {
                return false;   // doesn't fit
            }
            ctrl.floatok = true;

            if self.flags & MapObjectFlag::MF_TELEPORT as u32 == 0 &&
                ctrl.max_ceil_z - self.z < self.height {
                    return false;   // mobj must lower itself to fit
            }

            if self.flags & MapObjectFlag::MF_TELEPORT as u32 == 0 &&
                ctrl.min_floor_z - self.z > 24.0 {
                    return false;   // too big a step up
            }

            if self.flags & (MapObjectFlag::MF_DROPOFF as u32 | MapObjectFlag::MF_FLOAT as u32) == 0 &&
                ctrl.min_floor_z - ctrl.max_dropoff > 24.0 {
                    return false;   // too big a step up
            }
        }

        // the move is ok,
        // so link the thing into its new position
        // P_UnsetThingPosition (thing);

        let old_xy = self.xy;

        self.floorz = ctrl.min_floor_z;
        self.ceilingz = ctrl.max_ceil_z;
        self.xy = try_move;

        // P_SetThingPosition (thing);

        if self.flags & (MapObjectFlag::MF_TELEPORT as u32 | MapObjectFlag::MF_NOCLIP as u32) != 0 {
            for ld in &ctrl.spec_hits {
                // see if the line was crossed
                let side = ld.point_on_side(&self.xy);
                let old_side = ld.point_on_side(&old_xy);
                if side != old_side && ld.special != 0 {
                    // TODO: P_CrossSpecialLine(ld - lines, oldside, thing);
                }
            }
        }
        true
    }

    // P_CheckPosition
    // This is purely informative, nothing is modified
    // (except things picked up).
    //
    // in:
    //  a mobj_t (can be valid or invalid)
    //  a position to be checked
    //   (doesn't need to be related to the mobj_t->x,y)
    //
    // during:
    //  special things are touched if MF_PICKUP
    //  early out on solid lines?
    //
    // out:
    //  newsubsec
    //  floorz
    //  ceilingz
    //  tmdropoffz
    //   the lowest point contacted
    //   (monsters won't move to a dropoff)
    //  speciallines[]
    //  numspeciallines
    //
    /// Check for things and lines contacts.
    ///
    /// `PIT_CheckLine` is called by an iterator over the blockmap parts contacted
    /// and this function checks if the line is solid, if not then it also sets
    /// the portal ceil/floor coords and dropoffs
    fn p_check_position(&mut self, origin: Vec2, endpoint: Vec2, level: &mut Level) -> bool {
        let left = endpoint.x() - self.radius;
        let right = endpoint.x() + self.radius;
        let top = endpoint.y() + self.radius;
        let bottom = endpoint.y() - self.radius;
        let tmbbox = BBox {
            top,
            bottom,
            left,
            right,
        };

        let ctrl = &mut level.mobj_ctrl;
        let newsubsec = level.map_data.point_in_subsector(endpoint);
        // The base floor / ceiling is from the subsector
        // that contains the point.
        ctrl.min_floor_z = newsubsec.sector.floorheight;
        ctrl.max_dropoff = newsubsec.sector.floorheight;
        ctrl.max_ceil_z = newsubsec.sector.ceilingheight;

        if self.flags & MapObjectFlag::MF_NOCLIP as u32 != 0 {
            return true;
        }

        // TODO: use the blockmap for checking lines
        // TODO: use a P_BlockThingsIterator
        // TODO: use a P_BlockLinesIterator - used to build a list of lines to check
        //       it also calls PIT_CheckLine on each line
        //       P_BlockLinesIterator is called mobj->radius^2

        // BSP walk to find all subsectors between two points
        // Pretty much replaces the block iterators
        let mut bsp_trace = BSPTrace::new(origin, endpoint, level.map_data.start_node());
        bsp_trace.find_ssect_intercepts(&level.map_data);
        let segs = level.map_data.get_segments();
        let sub_sectors = level.map_data.get_subsectors();


        let leadx;
        let leady;
        let trailx;
        let traily;

        if self.momxy.x() > 0.0 {
            leadx = self.xy.x() + self.radius;
            trailx = self.xy.x() - self.radius;
        } else {
            leadx = self.xy.x() - self.radius;
            trailx = self.xy.x() + self.radius;
        }

        if self.xy.y() > 0.0 {
            leady = self.xy.y() + self.radius;
            traily = self.xy.y() - self.radius;
        } else {
            leady = self.xy.y() - self.radius;
            traily = self.xy.y() + self.radius;
        }

        // The p_try_move calls check collisions -> p_check_position -> pit_check_line
        let mut bsp_trace = BSPTrace::new(Vec2::new(trailx, traily), Vec2::new(leadx, leady) + self.momxy, level.map_data.start_node());
        bsp_trace.find_ssect_intercepts(&level.map_data);

        bsp_trace.set_line(Vec2::new(leadx, traily), Vec2::new(leadx, traily) + self.momxy);
        bsp_trace.find_ssect_intercepts(&level.map_data);

        bsp_trace.set_line(Vec2::new(trailx, leady), Vec2::new(trailx, leady) + self.momxy);
        bsp_trace.find_ssect_intercepts(&level.map_data);

        for n in bsp_trace.intercepted_nodes() {
            let ssect = &sub_sectors[*n as usize];
            let start = ssect.start_seg as usize;
            let end = start + ssect.seg_count as usize;
            for seg in &segs[start..end] {
                if !self.pit_check_line(&tmbbox, ctrl, &seg.linedef) {
                    return false;
                }
            }
        }

        true
    }

    /// PIT_CheckLine
    /// Adjusts tmfloorz and tmceilingz as lines are contacted
    ///
    /// This has been adjusted to take a seg ref instead as the linedef info
    /// is directly accessible.
    fn pit_check_line(
        &mut self,
        tmbbox: &BBox,
        // point1: Vec2,
        // point2: Vec2,
        ctrl: &mut SubSectorMinMax,
        ld: &LineDef,
    ) -> bool {
        if tmbbox.right < ld.bbox.left
            || tmbbox.left > ld.bbox.right
            || tmbbox.top < ld.bbox.bottom
            || tmbbox.bottom > ld.bbox.top
        {
            return true;
        }

        // In OG Doom the function used to check if collided is P_BoxOnLineSide
        // this does very fast checks using the line slope, for example a
        // line that is horizontal or vertical checked against the top/bottom/left/right
        // of bbox.
        // If the line is a slope then if it's positive or negative determines which
        // box corners are used - Doom checks which side of the line each are on
        // using `P_PointOnLineSide`
        // If both are same side then there is no intersection.

        if box_on_line_side(&tmbbox, &ld) != -1 {
            return true;
        }

        if ld.backsector.is_none() {
            // one-sided line
            return false;
        }

        if self.flags & MapObjectFlag::MF_MISSILE as u32 == 0 {
            if ld.flags & LineDefFlags::Blocking as i16 != 0 {
                return false; // explicitly blocking everything
            }

            if self.player.is_none() && ld.flags & LineDefFlags::BlockMonsters as i16 != 0 {
                return false; // block monsters only
            }
        }

        // Find the smallest/largest etc if group of line hits
        let portal = PortalZ::new(&ld);
        if portal.top_z < ctrl.max_ceil_z {
            ctrl.max_ceil_z = portal.top_z;
            // TODO: ceilingline = ld;
        }
        // Find the highest floor point (for steps etc)
        if portal.bottom_z > ctrl.min_floor_z {
            ctrl.min_floor_z = portal.bottom_z;
        }
        // Find the lowest possible point in subsectors contacted
        if portal.lowest_z < ctrl.max_dropoff {
            ctrl.max_dropoff = portal.lowest_z;
        }

        if ld.special != 0 {
            ctrl.spec_hits.push(DPtr::new(&ld));
        }

        true
    }

    // P_SlideMove
    // Loop until get a good move or stopped
    pub fn p_slide_move(&mut self, level: &mut Level) {
        // let ctrl = &mut level.mobj_ctrl;
        let mut hitcount = 0;
        let mut best_slide_frac = 1.0;
        let mut slide_move = self.momxy;

        let leadx;
        let leady;
        let trailx;
        let traily;

        if self.momxy.x() > 0.0 {
            leadx = self.xy.x() + self.radius;
            trailx = self.xy.x() - self.radius;
        } else {
            leadx = self.xy.x() - self.radius;
            trailx = self.xy.x() + self.radius;
        }

        if self.xy.y() > 0.0 {
            leady = self.xy.y() + self.radius;
            traily = self.xy.y() - self.radius;
        } else {
            leady = self.xy.y() - self.radius;
            traily = self.xy.y() + self.radius;
        }

        // The p_try_move calls check collisions -> p_check_position -> pit_check_line
        let mut bsp_trace = BSPTrace::new(Vec2::new(leadx, leady), Vec2::new(leadx, leady) + self.momxy, level.map_data.start_node());
        bsp_trace.find_ssect_intercepts(&level.map_data);

        // bsp_trace.set_line(Vec2::new(leadx, traily), Vec2::new(leadx, traily) + self.momxy);
        // bsp_trace.find_ssect_intercepts(&level.map_data);

        // bsp_trace.set_line(Vec2::new(trailx, leady), Vec2::new(trailx, leady) + self.momxy);
        // bsp_trace.find_ssect_intercepts(&level.map_data);

        loop {
            if hitcount == 3 {
                self.stair_step(level);
                return;
            }

            let mut best_slide_line = None;
            let mut collided = false;
            for n in bsp_trace.intercepted_nodes() {
                let segs = level.map_data.get_segments();
                let sub_sectors = level.map_data.get_subsectors();

                let ssect = &sub_sectors[*n as usize];
                let start = ssect.start_seg as usize;
                let end = start + ssect.seg_count as usize;
                for seg in &segs[start..end] {
                    if seg.linedef.point_on_side(&Vec2::new(leadx, leady)) != seg.linedef.point_on_side(&(Vec2::new(leadx, leady) + self.momxy)) {
                        best_slide_line = Some(seg.linedef.clone());
                        collided = true;
                        break;
                    }
                    if seg.linedef.point_on_side(&Vec2::new(leadx, traily)) != seg.linedef.point_on_side(&(Vec2::new(leadx, traily) + self.momxy)) {
                        best_slide_line = Some(seg.linedef.clone());
                        collided = true;
                        break;
                    }

                    if seg.linedef.point_on_side(&Vec2::new(trailx, leady)) != seg.linedef.point_on_side(&(Vec2::new(trailx, leady) + self.momxy)) {
                        best_slide_line = Some(seg.linedef.clone());
                        collided = true;
                        break;
                    }
                }
                if collided {
                    break;
                }
            }


            // TODO: move up to the wall / stairstep

            slide_move = slide_move * best_slide_frac; // bestfrac

            // Clip the moves.
            if let Some(best_slide_line) = best_slide_line.as_ref() {
                self.hit_slide_line(&mut slide_move, best_slide_line);
            }

            self.momxy = slide_move;

            let endpoint = self.xy + slide_move;
            if self.p_try_move(endpoint.x(), endpoint.y(), level) {
                return;
            }

            hitcount += 1;
        }
    }

    pub fn stair_step(&mut self, level: &mut Level) {
        dbg!("STEP 1");
        // Line might have hit the middle, end-on?
        if !self.p_try_move(self.xy.x(), self.xy.y() + self.momxy.y(), level) {
            dbg!("STEP 2");
            self.p_try_move(self.xy.x() + self.momxy.x(), self.xy.y(), level);
        }
    }

    /// P_HitSlideLine
    pub fn hit_slide_line(&self, slide_move: &mut Vec2, line: &LineDef,) {
        if matches!(line.slopetype, SlopeType::Horizontal) {
            slide_move.set_y(0.0);
            return;
        }
        if matches!(line.slopetype, SlopeType::Vertical) {
            slide_move.set_x(0.0);
            return;
        }

        let side = line.point_on_side(slide_move);
        let mut line_angle = Angle::from_vector(line.delta);
        if side == 1 {
            line_angle += FRAC_2_PI;
        }

        let move_angle = Angle::from_vector(*slide_move);

        let mut delta_angle = move_angle - line_angle;
        if delta_angle.rad() >= FRAC_PI_2 {
            delta_angle += FRAC_PI_2;
        }

        let move_dist = slide_move.length();
        let new_dist = move_dist * delta_angle.cos();

        *slide_move = line_angle.unit() * new_dist;
    }
}

/// P_RadiusAttack
/// Source is the creature that caused the explosion at spot.
pub fn p_radius_attack(spot: &mut MapObject, source: &mut MapObject, damage: f32) {
    let dist = damage + MAXRADIUS;
    unimplemented!()
    // // origin of block level is bmaporgx and bmaporgy
    // let yh = (spot.xy.y() + dist - bmaporgy) >> MAPBLOCKSHIFT;
    // let yl = (spot.xy.y() - dist - bmaporgy) >> MAPBLOCKSHIFT;
    // let xh = (spot.xy.x() + dist - bmaporgx) >> MAPBLOCKSHIFT;
    // let xl = (spot.xy.x() - dist - bmaporgx) >> MAPBLOCKSHIFT;
    // bombspot = spot;
    // bombsource = source;
    // bombdamage = damage;

    // for (y = yl; y <= yh; y++)
    // for (x = xl; x <= xh; x++)
    // P_BlockThingsIterator(x, y, PIT_RadiusAttack);
}
