use crate::renderer::{
    software::defs::{SCREENHEIGHT_HALF, SCREENWIDTH},
    Renderer,
};

use super::{defs::ClipRange, segs::SegRender, RenderData};
use doom_lib::{
    log::trace, Angle, Level, MapData, MapObject, Player, Sector, Segment, SubSector, TextureData,
    IS_SSECTOR_MASK,
};
use glam::Vec2;
use sdl2::{rect::Rect, render::Canvas, surface::Surface};
use std::{
    cell::RefCell,
    f32::consts::{FRAC_PI_2, FRAC_PI_4, PI},
    rc::Rc,
};

const MAX_SEGS: usize = 32;

// Need to sort out what is shared and what is not so that a data struct
// can be organised along with method/ownsership
//
// seg_t *curline; // SHARED, PASS AS AN ARG to segs.c functions
//
// side_t *sidedef; // don't use..., get from curline/seg
//
// line_t *linedef; // In maputils as an arg to P_LineOpening, not global
//
// These can be chased through the chain of:
// seg.linedef.front_sidedef.sector.floorheight
// This block as a struct to pass round?
//
// sector_t *frontsector; // Shared in seg/bsp . c, in segs StoreWallRange +
// sector_t *backsector;

/// We store most of what is needed for rendering in various functions here to avoid
/// having to pass too many things in args through multiple function calls. This
/// is due to the Doom C relying a fair bit on global state.
///
/// `RenderData` will be passed to the sprite drawer/clipper to use `drawsegs`
/// ----------------------------------------------------------------------------
/// - R_DrawSprite, r_things.c
/// - R_DrawMasked, r_things.c
/// - R_StoreWallRange, r_segs.c, checks only for overflow of drawsegs, and uses *one* entry through ds_p
///                               it then inserts/incs pointer to next drawseg in the array when finished
/// - R_DrawPlanes, r_plane.c, checks only for overflow of drawsegs
pub struct SoftwareRenderer {
    /// index in to self.solidsegs
    new_end: usize,
    solidsegs: Vec<ClipRange>,

    pub(super) r_data: RenderData,
    pub(super) seg_renderer: SegRender,
    pub(super) texture_data: Rc<RefCell<TextureData>>,
}

impl Renderer for SoftwareRenderer {
    fn render_player_view(&mut self, player: &Player, level: &Level, canvas: &mut Canvas<Surface>) {
        let map = &level.map_data;

        self.clear();

        // TODO: remove this once flats and sky are done.
        let sub_sect = unsafe {
            level
                .map_data
                .point_in_subsector_ref(player.mobj.as_ref().unwrap().as_ref().xy)
        };
        let light_level = unsafe { (*sub_sect).sector.lightlevel };
        let scale = light_level as f32 / 255.0;
        let colour = sdl2::pixels::Color::RGBA(
            (50.0 * scale) as u8,
            (40.0 * scale) as u8,
            (40.0 * scale) as u8,
            255,
        );
        canvas.set_draw_color(colour);
        canvas
            .fill_rect(Rect::new(
                0,
                0,
                SCREENWIDTH as u32,
                SCREENHEIGHT_HALF as u32,
            ))
            .unwrap();
        let colour = sdl2::pixels::Color::RGBA(
            (40.0 * scale) as u8,
            (40.0 * scale) as u8,
            (40.0 * scale) as u8,
            255,
        );
        canvas.set_draw_color(colour);
        canvas
            .fill_rect(Rect::new(
                0,
                100,
                SCREENWIDTH as u32,
                SCREENHEIGHT_HALF as u32,
            ))
            .unwrap();

        // TODO: netupdate

        let mut count = 0;
        self.render_bsp_node(map, player, map.start_node(), canvas, &mut count);
        trace!("BSP traversals for render: {count}");

        // TODO: netupdate again
        // TODO: drawplanes
        // TODO: netupdate again
        self.draw_masked(player.viewz, canvas);
        // TODO: netupdate again
    }
}

impl SoftwareRenderer {
    pub fn new(texture_data: Rc<RefCell<TextureData>>) -> Self {
        Self {
            r_data: RenderData::new(),
            seg_renderer: SegRender::new(texture_data.clone()),
            new_end: 0,
            solidsegs: Vec::new(),
            texture_data,
        }
    }

    pub fn clear(&mut self) {
        self.clear_clip_segs();
        self.r_data.clear_data();
        self.seg_renderer = SegRender::new(self.texture_data.clone());
    }

    /// R_AddLine - r_bsp
    fn add_line<'a>(
        &'a mut self,
        player: &Player,
        seg: &'a Segment,
        front_sector: &'a Sector,
        canvas: &mut Canvas<Surface>,
    ) {
        let mobj = unsafe { player.mobj.as_ref().unwrap().as_ref() };
        // reject orthogonal back sides
        let xy = mobj.xy;
        let angle = mobj.angle;

        if !seg.is_facing_point(&xy) {
            return;
        }

        let clipangle = Angle::new(FRAC_PI_4);
        // Reset to correct angles
        let mut angle1 = vertex_angle_to_object(&seg.v1, mobj);
        let mut angle2 = vertex_angle_to_object(&seg.v2, mobj);

        let span = angle1 - angle2;

        if span.rad() >= PI {
            return;
        }

        // Global angle needed by segcalc.
        self.r_data.rw_angle1 = angle1;

        angle1 -= angle;
        angle2 -= angle;

        let mut tspan = angle1 + clipangle;
        if tspan.rad() >= FRAC_PI_2 {
            tspan -= 2.0 * clipangle.rad();

            // Totally off the left edge?
            if tspan.rad() >= span.rad() {
                return;
            }
            angle1 = clipangle;
        }
        tspan = clipangle - angle2;
        if tspan.rad() > 2.0 * clipangle.rad() {
            tspan -= 2.0 * clipangle.rad();

            // Totally off the left edge?
            if tspan.rad() >= span.rad() {
                return;
            }
            angle2 = -clipangle;
        }

        angle1 += FRAC_PI_2;
        angle2 += FRAC_PI_2;
        let x1 = angle_to_screen(angle1.rad());
        let x2 = angle_to_screen(angle2.rad());

        // Does not cross a pixel?
        if x1 == x2 {
            return;
        }

        if let Some(back_sector) = &seg.backsector {
            // Doors. Block view
            if back_sector.ceilingheight <= front_sector.floorheight
                || back_sector.floorheight >= front_sector.ceilingheight
            {
                self.clip_solid_seg(x1, x2 - 1, seg, player, canvas);
                return;
            }

            // Windows usually, but also changes in heights from sectors eg: steps
            #[allow(clippy::float_cmp)]
            if back_sector.ceilingheight != front_sector.ceilingheight
                || back_sector.floorheight != front_sector.floorheight
            {
                self.clip_portal_seg(x1, x2 - 1, seg, player, canvas);
                return;
            }

            // Reject empty lines used for triggers and special events.
            // Identical floor and ceiling on both sides, identical light levels
            // on both sides, and no middle texture.
            if back_sector.ceilingpic == front_sector.ceilingpic
                && back_sector.floorpic == front_sector.floorpic
                && back_sector.lightlevel == front_sector.lightlevel
                && seg.sidedef.midtexture == usize::MAX
            {
                return;
            }
            self.clip_portal_seg(x1, x2 - 1, seg, player, canvas);
        } else {
            self.clip_solid_seg(x1, x2 - 1, seg, player, canvas);
        }
    }

    /// R_Subsector - r_bsp
    fn draw_subsector<'a>(
        &'a mut self,
        map: &MapData,
        object: &Player,
        subsect: &SubSector,
        canvas: &mut Canvas<Surface>,
    ) {
        // TODO: planes for floor & ceiling
        let front_sector = &subsect.sector;
        for i in subsect.start_seg..subsect.start_seg + subsect.seg_count {
            let seg = &map.segments()[i as usize];
            self.add_line(object, seg, front_sector, canvas);
        }
    }

    /// R_ClearClipSegs - r_bsp
    pub fn clear_clip_segs(&mut self) {
        self.solidsegs.clear();
        self.solidsegs.push(ClipRange {
            first: -0x7fffffff,
            last: -1,
        });
        for _ in 0..MAX_SEGS {
            self.solidsegs.push(ClipRange {
                first: 320,
                last: 0x7fffffff,
            });
        }
        self.new_end = 2;
    }

    /// R_ClipSolidWallSegment - r_bsp
    fn clip_solid_seg(
        &mut self,
        first: i32,
        last: i32,
        seg: &Segment,
        object: &Player,
        canvas: &mut Canvas<Surface>,
    ) {
        let mut next;

        // Find the first range that touches the range
        //  (adjacent pixels are touching).
        let mut start = 0; // first index
        while self.solidsegs[start].last < first - 1 {
            start += 1;
        }

        // We create the seg-renderer for each seg as data is not shared
        // TODO: check the above
        if first < self.solidsegs[start].first {
            if last < self.solidsegs[start].first - 1 {
                // Post is entirely visible (above start),
                // so insert a new clippost.
                self.seg_renderer.store_wall_range(
                    first,
                    last,
                    seg,
                    object,
                    &mut self.r_data,
                    canvas,
                );

                next = self.new_end;
                self.new_end += 1;

                while next != start {
                    self.solidsegs[next] = self.solidsegs[next - 1];
                    next -= 1;
                }

                self.solidsegs[next].first = first;
                self.solidsegs[next].last = last;
                return;
            }

            // There is a fragment above *start.
            // TODO: this causes a glitch?
            self.seg_renderer.store_wall_range(
                first,
                self.solidsegs[start].first - 1,
                seg,
                object,
                &mut self.r_data,
                canvas,
            );
            // Now adjust the clip size.
            self.solidsegs[start].first = first;
        }

        // Bottom contained in start?
        if last <= self.solidsegs[start].last {
            return;
        }

        next = start;
        while last >= self.solidsegs[next + 1].first - 1 {
            self.seg_renderer.store_wall_range(
                self.solidsegs[next].last + 1,
                self.solidsegs[next + 1].first - 1,
                seg,
                object,
                &mut self.r_data,
                canvas,
            );

            next += 1;

            if last <= self.solidsegs[next].last {
                self.solidsegs[start].last = self.solidsegs[next].last;
                return self.crunch(start, next);
            }
        }

        // There is a fragment after *next.
        self.seg_renderer.store_wall_range(
            self.solidsegs[next].last + 1,
            last,
            seg,
            object,
            &mut self.r_data,
            canvas,
        );
        // Adjust the clip size.
        self.solidsegs[start].last = last;

        //crunch
        self.crunch(start, next);
    }

    /// R_ClipPassWallSegment - r_bsp
    /// Clips the given range of columns, but does not includes it in the clip list.
    /// Does handle windows, e.g. LineDefs with upper and lower texture
    fn clip_portal_seg(
        &mut self,
        first: i32,
        last: i32,
        seg: &Segment,
        object: &Player,
        canvas: &mut Canvas<Surface>,
    ) {
        // Find the first range that touches the range
        //  (adjacent pixels are touching).
        let mut start = 0; // first index
        while self.solidsegs[start].last < first - 1 {
            start += 1;
        }

        if first < self.solidsegs[start].first {
            if last < self.solidsegs[start].first - 1 {
                // Post is entirely visible (above start),
                self.seg_renderer.store_wall_range(
                    first,
                    last,
                    seg,
                    object,
                    &mut self.r_data,
                    canvas,
                );
                return;
            }

            // There is a fragment above *start.
            self.seg_renderer.store_wall_range(
                first,
                self.solidsegs[start].first - 1,
                seg,
                object,
                &mut self.r_data,
                canvas,
            );
        }

        // Bottom contained in start?
        if last <= self.solidsegs[start].last {
            return;
        }

        while last >= self.solidsegs[start + 1].first - 1 {
            self.seg_renderer.store_wall_range(
                self.solidsegs[start].last + 1,
                self.solidsegs[start + 1].first - 1,
                seg,
                object,
                &mut self.r_data,
                canvas,
            );

            start += 1;

            if last <= self.solidsegs[start].last {
                return;
            }
        }

        // There is a fragment after *next.
        self.seg_renderer.store_wall_range(
            self.solidsegs[start].last + 1,
            last,
            seg,
            object,
            &mut self.r_data,
            canvas,
        );
    }

    fn crunch(&mut self, mut start: usize, mut next: usize) {
        if next == start {
            return;
        }

        while next != self.new_end && start < self.solidsegs.len() {
            next += 1;
            start += 1;
            self.solidsegs[start] = self.solidsegs[next];
        }
        self.new_end = start + 1;
    }

    /// R_RenderBSPNode - r_bsp
    pub fn render_bsp_node<'a>(
        &'a mut self,
        map: &MapData,
        player: &Player,
        node_id: u16,
        canvas: &mut Canvas<Surface>,
        count: &mut usize,
    ) {
        *count += 1;

        let mobj = unsafe { player.mobj.as_ref().unwrap().as_ref() };

        if node_id & IS_SSECTOR_MASK == IS_SSECTOR_MASK {
            // It's a leaf node and is the index to a subsector
            let subsect = &map.subsectors()[(node_id ^ IS_SSECTOR_MASK) as usize];
            // Check if it should be drawn, then draw
            self.draw_subsector(map, player, subsect, canvas);
            return;
        }

        // otherwise get node
        let node = &map.get_nodes()[node_id as usize];
        // find which side the point is on
        let side = node.point_on_side(&mobj.xy);
        // Recursively divide front space.
        self.render_bsp_node(map, player, node.child_index[side], canvas, count);

        // Possibly divide back space.
        // check if each corner of the BB is in the FOV
        //if node.point_in_bounds(&v, side ^ 1) {
        if node.bb_extents_in_fov(&mobj.xy, mobj.angle.rad(), FRAC_PI_4, side ^ 1) {
            self.render_bsp_node(map, player, node.child_index[side ^ 1], canvas, count);
        }
    }
}

pub fn angle_to_screen(mut radian: f32) -> i32 {
    let mut x;

    // Left side
    let p = 160.9 / (FRAC_PI_4).tan();
    if radian > FRAC_PI_2 {
        radian -= FRAC_PI_2;
        let t = radian.tan();
        x = t * p;
        x = p - x;
    } else {
        // Right side
        radian = FRAC_PI_2 - radian;
        let t = radian.tan();
        x = t * p;
        x += p;
    }
    x as i32
}

/// R_PointToAngle
// To get a global angle from cartesian coordinates,
//  the coordinates are flipped until they are in
//  the first octant of the coordinate system, then
//  the y (<=x) is scaled and divided by x to get a
//  tangent (slope) value which is looked up in the
//  tantoangle[] table.
///
/// The flipping isn't done here...
pub fn vertex_angle_to_object(vertex: &Vec2, mobj: &MapObject) -> Angle {
    let x = vertex.x() - mobj.xy.x();
    let y = vertex.y() - mobj.xy.y();
    Angle::new(y.atan2(x))
}

// pub fn point_to_angle_2(point1: &Vec2, point2: &Vec2) -> Angle {
//     let x = point1.x() - point2.x();
//     let y = point1.y() - point2.y();
//     Angle::new(y.atan2(x))
// }

#[cfg(test)]
mod tests {
    use doom_lib::{MapData, IS_SSECTOR_MASK};
    use wad::WadData;

    #[test]
    fn check_nodes_of_e1m1() {
        let wad = WadData::new("../doom1.wad".into());
        let mut map = MapData::new("E1M1".to_owned());
        map.load(&wad);

        let nodes = map.get_nodes();
        assert_eq!(nodes[0].xy.x() as i32, 1552);
        assert_eq!(nodes[0].xy.y() as i32, -2432);
        assert_eq!(nodes[0].delta.x() as i32, 112);
        assert_eq!(nodes[0].delta.y() as i32, 0);

        assert_eq!(nodes[0].bounding_boxes[0][0].x() as i32, 1552); //left
        assert_eq!(nodes[0].bounding_boxes[0][0].y() as i32, -2432); //top
        assert_eq!(nodes[0].bounding_boxes[0][1].x() as i32, 1664); //right
        assert_eq!(nodes[0].bounding_boxes[0][1].y() as i32, -2560); //bottom

        assert_eq!(nodes[0].bounding_boxes[1][0].x() as i32, 1600);
        assert_eq!(nodes[0].bounding_boxes[1][0].y() as i32, -2048);

        assert_eq!(nodes[0].child_index[0], 32768);
        assert_eq!(nodes[0].child_index[1], 32769);
        assert_eq!(IS_SSECTOR_MASK, 0x8000);

        assert_eq!(nodes[235].xy.x() as i32, 2176);
        assert_eq!(nodes[235].xy.y() as i32, -3776);
        assert_eq!(nodes[235].delta.x() as i32, 0);
        assert_eq!(nodes[235].delta.y() as i32, -32);
        assert_eq!(nodes[235].child_index[0], 128);
        assert_eq!(nodes[235].child_index[1], 234);

        println!("{:#018b}", IS_SSECTOR_MASK);

        println!("00: {:#018b}", nodes[0].child_index[0]);
        println!("00: {:#018b}", nodes[0].child_index[1]);

        println!("01: {:#018b}", nodes[1].child_index[0]);
        println!("01: {:#018b}", nodes[1].child_index[1]);

        println!("02: {:#018b}", nodes[2].child_index[0]);
        println!("02: {:#018b}", nodes[2].child_index[1]);

        println!("03: {:#018b}", nodes[3].child_index[0]);
        println!("03: {:#018b}", nodes[3].child_index[1]);
    }
}
