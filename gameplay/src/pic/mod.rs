//! All data and functions related to pictures in Doom.
//! These are:
//! - Wall textures
//! - Flat/span textures
//! - Palettes
//! - Coloumaps and light scaling
//! - Sprites (patches and frame sets)

mod animations;
pub use animations::*;
mod switches;
pub use switches::*;
mod sprites;

use std::mem::{size_of, size_of_val};

use log::{debug, warn};
use wad::types::{WadColour, WadPalette, WadPatch, WadTexture};
use wad::WadData;

use crate::doom_def::{GameMode, PowerType};
use crate::info::SPRNAMES;
use crate::pic::sprites::init_spritedefs;
use crate::Player;

use self::sprites::SpriteDef;

const MAXLIGHTZ: i32 = 128;
const LIGHTLEVELS: i32 = 16;
const NUMCOLORMAPS: i32 = 32;
const MAXLIGHTSCALE: i32 = 48;
pub const INVERSECOLORMAP: i32 = 32;
const STARTREDPALS: usize = 1;
const NUMREDPALS: usize = 8;
const STARTBONUSPALS: usize = 9;
const NUMBONUSPALS: usize = 4;
const RADIATIONPAL: usize = 13;

#[derive(Debug)]
pub struct FlatPic {
    pub name: String,
    pub data: [[usize; 64]; 64],
}

#[derive(Debug)]
pub struct WallPic {
    pub name: String,
    pub data: Vec<Vec<usize>>,
}

#[derive(Debug)]
pub struct SpritePic {
    pub name: String,
    pub left_offset: i32,
    pub top_offset: i32,
    pub data: Vec<Vec<usize>>,
}

#[derive(Debug, Default)]
pub struct PicData {
    /// Colours for pixels
    palettes: Vec<WadPalette>,
    // Usually 34 blocks of 256, each u8 being an index in to the palette
    colourmap: Vec<Vec<usize>>,
    light_scale: Vec<Vec<Vec<usize>>>,
    use_fixed_colourmap: usize,
    zlight_scale: Vec<Vec<Vec<usize>>>,
    walls: Vec<WallPic>,
    /// Used in animations
    wall_translation: Vec<usize>,
    flats: Vec<FlatPic>,
    /// Used in animations
    flat_translation: Vec<usize>,
    /// The number flats use to signify a sky should be drawn
    sky_num: usize,
    /// The index number of the texture to use for skybox
    sky_pic: usize,
    //
    sprite_patches: Vec<SpritePic>,
    sprite_defs: Vec<SpriteDef>,
    /// The pallette to be used. Can be set with `set_pallette()` or
    /// `set_player_palette()`, typically done on frame start to set effects
    /// like take-damage.
    use_pallette: usize,
    double_res: bool,
}

impl PicData {
    pub fn init(double_res: bool, wad: &WadData) -> Self {
        print!("Init image data  [");

        let colourmap = Self::init_colourmap(wad);
        let palettes = Self::init_palette(wad);
        let light_scale = Self::init_light_scales(&colourmap);
        let zlight_scale = Self::init_zlight_scales(&colourmap);

        let (walls, sky_pic) = Self::init_wall_pics(wad);
        let wall_translation = (0..walls.len()).collect();

        let (flats, sky_num) = Self::init_flat_pics(wad);
        let flat_translation = (0..flats.len()).collect();

        let sprite_patches: Vec<SpritePic> = wad
            .sprites_iter()
            .enumerate()
            .map(|(i, patch)| {
                if i % 64 == 0 {
                    print!(".");
                }

                let mut x_pos = 0;
                let mut compose =
                    vec![vec![usize::MAX; patch.height as usize]; patch.width as usize];
                for c in patch.columns.iter() {
                    if x_pos == patch.width as i32 {
                        break;
                    }
                    for (y, p) in c.pixels.iter().enumerate() {
                        let y_pos = y as i32 + c.y_offset;
                        if y_pos >= 0 && y_pos < patch.height as i32 && x_pos >= 0 {
                            compose[x_pos as usize][y_pos as usize] = *p;
                        }
                    }
                    if c.y_offset == 255 {
                        x_pos += 1;
                    }
                }

                SpritePic {
                    name: patch.name,
                    top_offset: patch.top_offset as i32,
                    left_offset: patch.left_offset as i32,
                    data: compose,
                }
            })
            .collect();
        let sprite_defs = init_spritedefs(&SPRNAMES, &sprite_patches);

        println!(".]");

        Self {
            walls,
            wall_translation,
            sky_num,
            sky_pic,
            flats,
            flat_translation,
            palettes,
            light_scale,
            zlight_scale,
            colourmap,
            use_fixed_colourmap: 0,
            sprite_patches,
            sprite_defs,
            use_pallette: 0,
            double_res,
        }
    }

    fn init_palette(wad: &WadData) -> Vec<WadPalette> {
        print!(".");
        wad.playpal_iter().collect()
    }

    fn init_colourmap(wad: &WadData) -> Vec<Vec<usize>> {
        print!(".");
        wad.colourmap_iter()
            .map(|i| i as usize)
            .collect::<Vec<usize>>()
            .chunks(256)
            .map(|v| v.to_owned())
            .collect()
    }

    fn init_light_scales(colourmap: &[Vec<usize>]) -> Vec<Vec<Vec<usize>>> {
        print!(".");
        (0..LIGHTLEVELS)
            .map(|i| {
                let startmap = ((LIGHTLEVELS - 1 - i) * 2) * NUMCOLORMAPS / LIGHTLEVELS;
                (0..MAXLIGHTSCALE)
                    .map(|j| {
                        // let j = MAXLIGHTSCALE - j;
                        let mut level = startmap - j / 2;
                        // let scale = (160 / (j + 1)) as f32;
                        // let mut level = startmap - (scale / 2.0) as
                        // i32;
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
            .collect()
    }

    /// A non-zero value is the the colourmap number forced to use for all
    /// light-levels
    pub fn set_fixed_lightscale(&mut self, colourmap: usize) {
        self.use_fixed_colourmap = colourmap
    }

    fn init_zlight_scales(colourmap: &[Vec<usize>]) -> Vec<Vec<Vec<usize>>> {
        print!(".");
        (0..LIGHTLEVELS)
            .map(|i| {
                let startmap = ((LIGHTLEVELS - 1 - i) * 2) * NUMCOLORMAPS / LIGHTLEVELS;
                (0..MAXLIGHTZ)
                    .map(|j| {
                        let scale = (160 / (j + 1)) as f32;
                        let mut level = startmap - (scale / 2.0) as i32;
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
            .collect()
    }

    fn init_wall_pics(wad: &WadData) -> (Vec<WallPic>, usize) {
        print!(".");
        let patches: Vec<WadPatch> = wad.patches_iter().collect();
        // Need to include flats
        let pnames: Vec<String> = wad.pnames_iter().collect();
        let mut sorted: Vec<WadPatch> = Vec::with_capacity(pnames.len());
        for name in &pnames {
            let mut log = true;
            for patch in &patches {
                if &patch.name == name {
                    sorted.push(patch.clone());
                    log = false;
                    break;
                }
            }
            if log {
                warn!("Mising: {name}");
            }
        }
        print!(".");
        // info!("Init wall textures.");
        let mut skytexture = 0;
        let mut texture_alloc_size = 0;

        let mut pic_func = |(i, tex)| {
            let pic = Self::build_wall_pic(tex, &sorted);
            if pic.name == "SKY1" {
                print!(".");
                skytexture = i;
            }
            texture_alloc_size += size_of_val(&pic.name);
            for y in &pic.data {
                texture_alloc_size += size_of::<usize>() * y.len();
            }
            if i % 64 == 0 {
                print!(".");
            }
            pic
        };

        let mut wall_pic: Vec<WallPic> = wad
            .texture_iter("TEXTURE1")
            .enumerate()
            .map(&mut pic_func)
            .collect();

        if wad.lump_exists("TEXTURE2") {
            let mut textures2: Vec<WallPic> = wad
                .texture_iter("TEXTURE2")
                .enumerate()
                .map(&mut pic_func)
                .collect();
            wall_pic.append(&mut textures2);
        };

        let tmp = (texture_alloc_size / 1024).to_string();
        let size = tmp.split_at(2);
        debug!("Total memory used for textures: {},{} KiB", size.0, size.1);

        (wall_pic, skytexture)
    }

    fn init_flat_pics(wad: &WadData) -> (Vec<FlatPic>, usize) {
        print!(".");
        let mut skynum = 256;
        // info!("Init flats.");
        let mut flats = Vec::with_capacity(wad.flats_iter().count());
        print!(".");

        let mut flat_alloc_size = 0;
        for (i, wf) in wad.flats_iter().enumerate() {
            let mut flat = FlatPic {
                name: wf.name.clone(),
                data: [[0; 64]; 64],
            };
            let mut outofbounds = false;
            for (y, col) in wf.data.chunks(64).enumerate() {
                if y >= 64 || outofbounds {
                    outofbounds = true;
                    break;
                }
                for (x, px) in col.iter().enumerate() {
                    if x >= 64 || outofbounds {
                        outofbounds = true;
                        break;
                    }
                    flat.data[x][y] = *px as usize;
                }
            }
            if outofbounds {
                warn!("Flat {} was not 64x64 in size", wf.name);
            }
            if flat.name == "F_SKY1" {
                skynum = flats.len();
            }

            flat_alloc_size += size_of_val(&flat.name);
            flat_alloc_size += flat.data.len() * flat.data[0].len() * size_of::<u8>();
            if i % 32 == 0 {
                print!(".");
            }

            flats.push(flat);
        }

        debug!(
            "Total memory used for flats: {} KiB",
            flat_alloc_size / 1024
        );

        (flats, skynum)
    }

    /// Build a texture out of patches and return it
    fn build_wall_pic(texture: WadTexture, patches: &[WadPatch]) -> WallPic {
        let mut compose = vec![vec![usize::MAX; texture.height as usize]; texture.width as usize];
        for wad_tex_patch in texture.patches.iter() {
            let wad_patch = &patches[wad_tex_patch.patch_index];
            // draw patch
            let mut x_pos = wad_tex_patch.origin_x;
            if x_pos.is_negative() {
                // OG Doom sets the start to 0 if less than 0
                // skip = x_pos.abs() as usize;
                x_pos = 0;
            }
            for c in wad_patch.columns.iter() {
                if c.y_offset == 255 {
                    x_pos += 1;
                    continue;
                }
                if x_pos == texture.width as i32 {
                    break;
                }

                for (y, p) in c.pixels.iter().enumerate() {
                    let y_pos = y as i32 + wad_tex_patch.origin_y + c.y_offset;
                    if y_pos >= 0 && y_pos < texture.height as i32 {
                        compose[x_pos as usize][y_pos as usize] = *p;
                    }
                }
            }
        }

        debug!("Built texture: {}", &texture.name);
        WallPic {
            name: texture.name,
            data: compose,
        }
    }

    pub fn palette(&self) -> &[WadColour] {
        &self.palettes[self.use_pallette].0
    }

    pub fn set_palette(&mut self, mut num: usize) {
        if num >= self.palettes.len() {
            num = self.palettes.len() - 1;
        }
        self.use_pallette = num;
    }

    /// Used to set effects for the player visually, such as damage
    pub fn set_player_palette(&mut self, player: &Player) {
        let mut damagecount = player.status.damagecount;
        let berkers;

        if player.status.powers[PowerType::Strength as usize] != 0 {
            // slowly fade the berzerk out
            berkers = 12 - (player.status.powers[PowerType::Strength as usize] >> 6);

            if berkers > damagecount {
                damagecount = berkers;
            }
        }

        if damagecount != 0 {
            self.use_pallette = ((damagecount + 7) >> 3) as usize;
            if self.use_pallette >= NUMREDPALS {
                self.use_pallette = NUMREDPALS - 1;
            }
            self.use_pallette += STARTREDPALS;
        } else if player.status.bonuscount != 0 {
            self.use_pallette = ((player.status.bonuscount + 7) >> 3) as usize;
            if self.use_pallette >= NUMBONUSPALS {
                self.use_pallette = NUMBONUSPALS - 1;
            }
            self.use_pallette += STARTBONUSPALS;
        } else if player.status.powers[PowerType::IronFeet as usize] > 4 * 32
            || player.status.powers[PowerType::IronFeet as usize] & 8 != 0
        {
            self.use_pallette = RADIATIONPAL;
        } else {
            self.use_pallette = 0;
        }

        if self.use_pallette >= self.palettes.len() {
            self.use_pallette = self.palettes.len() - 1;
        }
    }

    /// Get the number of the flat used for the sky texture. Sectors using this
    /// number for the flat will be rendered with the skybox.
    pub fn sky_num(&self) -> usize {
        self.sky_num
    }

    /// Get the index used by `get_texture()` to return a texture.
    pub fn sky_pic(&self) -> usize {
        self.sky_pic
    }

    /// Set the correct skybox for the map/episode currently playing
    pub fn set_sky_pic(&mut self, mode: GameMode, episode: usize, map: usize) {
        if mode == GameMode::Commercial {
            self.sky_pic = self.wallpic_num_for_name("SKY3").expect("SKY3 is missing");
            if map < 12 {
                self.sky_pic = self.wallpic_num_for_name("SKY1").expect("SKY1 is missing");
            } else if map < 21 {
                self.sky_pic = self.wallpic_num_for_name("SKY2").expect("SKY2 is missing");
            }
        } else {
            match episode {
                2 => {
                    self.sky_pic = self.wallpic_num_for_name("SKY2").expect("SKY2 is missing");
                }
                3 => {
                    self.sky_pic = self.wallpic_num_for_name("SKY3").expect("SKY3 is missing");
                }
                4 => {
                    self.sky_pic = self.wallpic_num_for_name("SKY4").expect("SKY4 is missing");
                }
                _ => {
                    self.sky_pic = self.wallpic_num_for_name("SKY1").expect("SKY1 is missing");
                }
            }
        }
    }

    pub fn colourmap(&self, index: usize) -> &[usize] {
        &self.colourmap[index]
    }

    fn colourmap_for_scale(&self, scale: f32) -> usize {
        let mut colourmap = if self.double_res {
            (scale * 15.8 / 2.0) as u32
        } else {
            (scale * 15.8) as u32
        };
        if colourmap >= MAXLIGHTSCALE as u32 - 1 {
            colourmap = MAXLIGHTSCALE as u32 - 1;
        }
        colourmap as usize
    }

    /// Get the correct colourmapping for a light level. The colourmap is
    /// indexed by the Y coordinate of a texture column.
    pub fn wall_light_colourmap(&self, light_level: i32, wall_scale: f32) -> &[usize] {
        if self.use_fixed_colourmap != 0 {
            return &self.colourmap[self.use_fixed_colourmap];
        }

        let mut light_level = light_level;
        if light_level >= self.light_scale.len() as i32 {
            light_level = self.light_scale.len() as i32 - 1;
        }

        let colourmap = self.colourmap_for_scale(wall_scale);
        &self.light_scale[light_level as usize][colourmap]
    }

    /// Light may need right-shifting by 4
    pub fn sprite_light_colourmap(&self, light_level: usize, scale: f32) -> &[usize] {
        if self.use_fixed_colourmap != 0 {
            return &self.colourmap[self.use_fixed_colourmap];
        }

        let mut light_level = light_level;
        if light_level >= self.light_scale.len() {
            light_level = self.light_scale.len() - 1;
        }

        let colourmap = self.colourmap_for_scale(scale);
        &self.light_scale[light_level][colourmap]
    }

    // pub fn light_colourmap(&self, light_level: usize, colourmap: usize) ->
    // &[usize] {     &self.light_scale[light_level][colourmap]
    // }

    pub fn flat_light_colourmap(&self, mut light_level: i32, wall_scale: u32) -> &[usize] {
        if self.use_fixed_colourmap != 0 {
            return &self.colourmap[self.use_fixed_colourmap];
        }

        let mut dist = wall_scale >> 4;

        if dist >= MAXLIGHTZ as u32 - 1 {
            dist = MAXLIGHTZ as u32 - 1;
        }

        if light_level >= self.zlight_scale.len() as i32 {
            light_level = self.zlight_scale.len() as i32 - 1;
        }

        &self.zlight_scale[light_level as usize][dist as usize]
    }

    pub fn get_texture(&self, num: usize) -> &WallPic {
        let num = self.wall_translation[num];
        &self.walls[num]
    }

    pub fn get_flat(&self, num: usize) -> &FlatPic {
        let num = self.flat_translation[num];
        &self.flats[num]
    }

    pub fn wallpic_num_for_name(&self, name: &str) -> Option<usize> {
        for (i, tex) in self.walls.iter().enumerate() {
            if tex.name == name {
                return Some(i);
            }
        }
        None
    }

    pub fn flat_num_for_name(&self, name: &str) -> Option<usize> {
        for (i, tex) in self.flats.iter().enumerate() {
            if tex.name == name {
                return Some(i);
            }
        }
        None
    }

    /// Return a ref to the specified column of the requested texture
    pub fn wall_pic_column(&self, texture: usize, mut texture_column: usize) -> &[usize] {
        let texture = &self.walls[self.wall_translation[texture]];

        if texture_column >= texture.data.len() {
            texture_column &= texture.data.len() - 1;
        }

        &texture.data[texture_column]
    }

    pub fn num_textures(&self) -> usize {
        self.walls.len()
    }

    pub fn sprite_def(&self, sprite_num: usize) -> &SpriteDef {
        &self.sprite_defs[sprite_num]
    }

    pub fn sprite_patch(&self, patch_num: usize) -> &SpritePic {
        &self.sprite_patches[patch_num]
    }
}
