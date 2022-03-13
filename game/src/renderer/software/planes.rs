use super::defs::{Visplane, MAXOPENINGS, MAXVISPLANES, SCREENHEIGHT, SCREENWIDTH};

pub struct VisPlaneRender {
    // Here comes the obnoxious "visplane".
    pub visplanes: [Visplane; MAXVISPLANES],
    pub lastvisplane: usize,
    /// Index of current visplane in `self.visplanes` for floor
    pub floorplane: usize,
    /// Index of current visplane in `self.visplanes` for ceiling
    pub ceilingplane: usize,

    /// Stores the column number of the texture required for this opening
    pub openings: [f32; MAXOPENINGS],
    pub lastopening: i32,

    pub floorclip: [i32; SCREENWIDTH],
    pub ceilingclip: [i32; SCREENWIDTH],
    /// spanstart holds the start of a plane span
    /// initialized to 0 at start
    pub spanstart: [i32; SCREENHEIGHT],
    pub spanstop: [i32; SCREENHEIGHT],

    //lighttable_t **planezlight;
    pub planeheight: f32,

    pub yslope: [f32; SCREENHEIGHT],
    pub distscale: [f32; SCREENWIDTH],
    pub basexscale: f32,
    pub baseyscale: f32,

    pub cachedheight: [f32; SCREENHEIGHT],
    pub cacheddistance: [f32; SCREENHEIGHT],
    pub cachedxstep: [f32; SCREENHEIGHT],
    pub cachedystep: [f32; SCREENHEIGHT],
}

impl Default for VisPlaneRender {
    fn default() -> Self {
        VisPlaneRender::new()
    }
}

impl VisPlaneRender {
    pub fn new() -> Self {
        VisPlaneRender {
            visplanes: [Visplane::default(); MAXVISPLANES],
            lastvisplane: 0,
            floorplane: 0,
            ceilingplane: 0,
            openings: [f32::MAX; MAXOPENINGS],
            lastopening: 0,
            floorclip: [-1; SCREENWIDTH],
            ceilingclip: [-1; SCREENWIDTH],
            spanstart: [0; SCREENHEIGHT],
            spanstop: [0; SCREENHEIGHT],
            planeheight: 0.0,
            yslope: [0.0; SCREENHEIGHT],
            distscale: [0.0; SCREENWIDTH],
            basexscale: 0.0,
            baseyscale: 0.0,
            cachedheight: [0.0; SCREENHEIGHT],
            cacheddistance: [0.0; SCREENHEIGHT],
            cachedxstep: [0.0; SCREENHEIGHT],
            cachedystep: [0.0; SCREENHEIGHT],
        }
    }

    /// R_ClearPlanes
    /// At begining of frame.
    pub fn clear_planes(&mut self) {
        // opening / clipping determination
        for i in 0..SCREENWIDTH {
            self.floorclip[i] = SCREENHEIGHT as i32;
            self.ceilingclip[i] = -1;
        }

        self.lastvisplane = 0;
        self.lastopening = 0;

        // texture calculation
        for i in self.cachedheight.iter_mut() {
            *i = 0.0;
        }

        // left to right mapping
        // TODO: angle = (viewangle - ANG90) >> ANGLETOFINESHIFT;

        // TODO: Don't hardcode this; centerxfrac
        // scale will be unit scale at SCREENWIDTH/2 distance
        self.basexscale = (160.0f32).cos();
        self.baseyscale = -(160.0f32).sin();
    }

    pub fn find_plane<'a>(
        &'a mut self,
        mut height: f32,
        picnum: usize,
        skynum: usize,
        mut light_level: f32,
    ) -> usize {
        if picnum == skynum {
            height = 0.0;
            light_level = 0.0;
        }

        let mut check_idx = 0;
        let len = self.visplanes.len();

        for i in 0..self.lastvisplane {
            check_idx += 1;
            if height == self.visplanes[i].height
                && picnum == self.visplanes[i].picnum
                && light_level == self.visplanes[i].lightlevel
            {
                break;
            }
        }
        let mut check = &mut self.visplanes[check_idx];

        if check_idx < self.lastvisplane {
            return check_idx;
        }

        // Otherwise edit new
        if self.lastvisplane < len - 1 {
            self.lastvisplane += 1;
            check.height = height;
            check.picnum = picnum;
            check.lightlevel = light_level;
            check.minx = SCREENWIDTH as i32;
            check.maxx = -1;
            for t in &mut check.top {
                *t = 0xff;
            }
        }

        check_idx
    }

    pub fn check_plane<'a>(&'a mut self, start: i32, stop: i32, plane_idx: usize) -> usize {
        let plane = &mut self.visplanes[plane_idx];

        let (intrl, unionl) = if start < plane.minx {
            (plane.minx, start)
        } else {
            (start, plane.minx)
        };

        let (intrh, unionh) = if stop > plane.maxx {
            (plane.maxx, stop)
        } else {
            (stop, plane.maxx)
        };

        let mut x = intrl;
        for i in x..=intrh {
            x = i;
            if plane.top[i as usize] != 0xff {
                break;
            }
        }

        if x > intrh {
            plane.minx = unionl;
            plane.maxx = unionh;
            // Use the same plane
            return plane_idx;
        }

        // Otherwise make a new plane
        let height = plane.height;
        let picnum = plane.picnum;
        let lightlevel = plane.lightlevel;
        let plane = &mut self.visplanes[self.lastvisplane];
        plane.height = height;
        plane.picnum = picnum;
        plane.lightlevel = lightlevel;

        if self.lastvisplane == self.visplanes.len() - 1 {
            panic!("No more visplanes");
        }

        self.lastvisplane += 1;
        let plane = &mut self.visplanes[self.lastvisplane];
        plane.minx = start;
        plane.maxx = stop;
        for t in &mut plane.top {
            *t = 0xff;
        }

        self.lastvisplane
    }

    fn current_floor_plane(&self) -> &Visplane {
        &self.visplanes[self.floorplane]
    }

    fn current_ceiling_plane(&self) -> &Visplane {
        &self.visplanes[self.ceilingplane]
    }

    // R_CheckPlane
    //pub fn check_set_floor_plane
}
