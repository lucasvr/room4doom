use doom_lib::{Angle, Texture};
use wad::{
    lumps::{WadColour, WadPalette, WadPatch, WadTexture},
    WadData,
};

use self::{defs::DrawSeg, portals::PortalClip};

pub mod bsp;
pub mod defs;
pub mod plane;
pub mod portals;
pub mod segs;
pub mod things;

const LIGHTLEVELS: i32 = 16;
const NUMCOLORMAPS: i32 = 32;
const MAXLIGHTSCALE: i32 = 48;

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
#[derive(Default)]
pub struct RenderData {
    pub rw_angle1: Angle,
    // DrawSeg used, which is inserted in drawsegs at end of r_segs
    pub drawsegs: Vec<DrawSeg>,
    pub portal_clip: PortalClip,
    /// index to drawsegs
    /// Used in r_segs and r_things
    pub ds_p: usize, // Or, depending on place in code this can be skipped and a new

    /// Colours for pixels
    palettes: Vec<WadPalette>,
    // Usually 34 blocks of 256, each u8 being an index in to the palette
    colourmap: Vec<Vec<usize>>,
    lightscale: Vec<Vec<Vec<usize>>>,
    /// Indexing is [texture num][x][y]
    textures: Vec<Texture>,
}

impl RenderData {
    pub fn new(wad: &WadData) -> Self {
        let palettes = wad.playpal_iter().collect();
        let colourmap: Vec<Vec<usize>> = wad
            .colourmap_iter()
            .map(|i| i as usize)
            .collect::<Vec<usize>>()
            .chunks(256)
            .map(|v| v.to_owned())
            .collect();

        let lightscale = (0..LIGHTLEVELS)
            .map(|i| {
                let startmap = ((LIGHTLEVELS - 1 - i) * 2) * NUMCOLORMAPS / LIGHTLEVELS;
                (0..MAXLIGHTSCALE)
                    .map(|j| {
                        let mut level = startmap - j / 2;
                        if level < 0 {
                            level = 0;
                        }
                        if level >= NUMCOLORMAPS {
                            level = NUMCOLORMAPS - 1;
                        }
                        colourmap[level as usize].to_owned()
                    })
                    .collect()
            })
            .collect();

        for i in 0..LIGHTLEVELS {
            // TODO: const LIGHTLEVELS
            let startmap = ((LIGHTLEVELS - 1 - i) * 2) * NUMCOLORMAPS / LIGHTLEVELS;
            for j in 0..MAXLIGHTSCALE {
                let mut level = startmap - j / 2;
                if level < 0 {
                    level = 0;
                }
                if level >= NUMCOLORMAPS {
                    level = NUMCOLORMAPS - 1;
                }
            }
        }

        let patches: Vec<WadPatch> = wad.patches_iter().collect();
        let mut textures: Vec<Texture> = wad
            .texture_iter("TEXTURE1")
            .map(|tex| Self::compose_texture(tex, &patches))
            .collect();
        if wad.lump_exists("TEXTURE2") {
            let mut textures2: Vec<Texture> = wad
                .texture_iter("TEXTURE2")
                .map(|tex| Self::compose_texture(tex, &patches))
                .collect();
            textures.append(&mut textures2);
        }

        Self {
            rw_angle1: Angle::default(),
            drawsegs: Vec::new(),
            portal_clip: PortalClip::default(),
            ds_p: 0,
            palettes,
            colourmap,
            lightscale,
            textures,
        }
    }

    fn compose_texture(texture: WadTexture, patches: &[WadPatch]) -> Texture {
        let mut compose = vec![vec![usize::MAX; texture.height as usize]; texture.width as usize];

        for patch_pos in &texture.patches {
            let patch = &patches[patch_pos.patch_index];
            // draw patch
            let mut x_pos = patch_pos.origin_x;
            for c in patch.columns.iter() {
                if x_pos == texture.width as i32 {
                    break;
                }
                for (y, p) in c.pixels.iter().enumerate() {
                    let y_pos = y as i32 + patch_pos.origin_y + c.y_offset as i32;
                    if y_pos >= 0 && y_pos < texture.height as i32 && x_pos >= 0 {
                        compose[x_pos as usize][y_pos as usize] = *p;
                    }
                }
                if c.y_offset == 255 {
                    x_pos += 1;
                }
            }
        }
        compose
    }

    pub fn get_palette(&self, num: usize) -> &[WadColour] {
        &self.palettes[num].0
    }

    pub fn get_colourmap(&self, index: usize) -> &[usize] {
        &self.colourmap[index]
    }

    pub fn get_lightscale(&self, index: usize) -> &Vec<Vec<usize>> {
        &self.lightscale[index]
    }

    pub fn get_texture(&self, num: usize) -> &Texture {
        &self.textures[num]
    }

    pub fn num_textures(&self) -> usize {
        self.textures.len()
    }

    pub fn clear_data(&mut self) {
        self.portal_clip.clear();
        self.drawsegs.clear();
        self.ds_p = 0;
        self.rw_angle1 = Angle::default();
    }
}
