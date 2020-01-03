use std::str;

/// The flags control some attributes of the line
pub enum LineDefFlags {
    /// Players and monsters cannot cross this line. Note that
    /// if there is no sector on the other side, they can't go through the line
    /// anyway, regardless of the flags
    Blocking = 1,
    /// Monsters cannot cross this line
    BlockMonsters = 1 << 1,
    /// The linedef's two sidedefs can have "-" as a texture,
    /// which in this case means "transparent". If this flag is not set, the
    /// sidedefs can't be transparent. A side effect of this flag is that if
    /// it is set, then gunfire (pistol, shotgun, chaingun) can go through it
    TwoSided = 1 << 2,
    /// The upper texture is pasted onto the wall from
    /// the top down instead of from the bottom up like usual.
    /// The effect is if a wall moves down, it looks like the
    /// texture is stationary and is appended to as the wall moves
    UnpegTop = 1 << 3,
    /// Lower and middle textures are drawn from the
    /// bottom up, instead of from the top down like usual
    /// The effect is if a wall moves up, it looks like the
    /// texture is stationary and is appended to as the wall moves
    UnpegBottom = 1 << 4,
    /// On the automap, this line appears in red like a normal
    /// solid wall that has nothing on the other side. This is useful in
    /// protecting secret doors and such. Note that if the sector on the other
    /// side of this "secret" line has its floor height HIGHER than the sector
    /// on the facing side of the secret line, then the map will show the lines
    /// beyond and thus give up the secret
    Secret = 1 << 5,
    /// For purposes of monsters hearing sounds and thus
    /// becoming alerted. Every time a player fires a weapon, the "sound" of
    /// it travels from sector to sector, alerting all non-deaf monsters in
    /// each new sector. This flag blocks sound traveling out of this sector
    /// through this line to adjacent sector
    BlockSound = 1 << 6,
    /// Not on AutoMap
    DontDraw = 1 << 7,
    /// Already on AutoMap
    Draw = 1 << 8,
}

#[derive(Debug)]
pub struct Vertex {
    pub x: i16,
    pub y: i16,
}

impl Vertex {
    pub fn new(x: i16, y: i16) -> Vertex {
        Vertex { x: x, y: y }
    }
}

/// Each linedef represents a line from one of the VERTEXES to another,
/// and each linedef's record is 14 bytes, and is made up of 7 16-bit
/// fields
#[derive(Debug)]
pub struct LineDef {
    /// The line starts from this point
    pub start_vertex: i16,
    /// The line ends at this point
    pub end_vertex: i16,
    /// The line attributes, see `LineDefFlags`
    pub flags: u16,
    pub line_type: u16,
    /// This is a number which ties this line's effect type
    /// to all SECTORS that have the same tag number (in their last
    /// field)
    pub sector_tag: u16,
    /// Index number of the front `SideDef` for this line
    pub front_sidedef: u16, //0xFFFF means there is no sidedef
    /// Index number of the back `SideDef` for this line
    pub back_sidedef: u16, //0xFFFF means there is no sidedef
}

impl LineDef {
    pub fn new(
        start_vertex: i16,
        end_vertex: i16,
        flags: u16,
        line_type: u16,
        sector_tag: u16,
        front_sidedef: u16,
        back_sidedef: u16,
    ) -> LineDef {
        LineDef {
            start_vertex,
            end_vertex,
            flags,
            line_type,
            sector_tag,
            front_sidedef,
            back_sidedef,
        }
    }
}

/// A `Sector` is a horizontal (east-west and north-south) area of the map
/// where a floor height and ceiling height is defined.
/// Any change in floor or ceiling height or texture requires a
/// new sector (and therefore separating linedefs and sidedefs).
///
/// Each `Sector` record is 26 bytes
#[derive(Debug)]
pub struct Sector {
    floor_height: i16,
    ceil_height: i16,
    /// Floor texture name
    floor_tex: String,
    /// Ceiling texture name
    ceil_tex: String,
    /// Light level from 0-255. There are actually only 32 brightnesses
    /// possible so blocks of 8 are the same bright
    light_level: u16,
    /// This determines some area-effects called special sectors
    typ: u16,
    /// a "tag" number corresponding to LINEDEF(s) with the same tag
    /// number. When that linedef is activated, something will usually
    /// happen to this sector - its floor will rise, the lights will
    /// go out, etc
    tag: u16,
}

impl Sector {
    pub fn new(
        floor_height: i16,
        ceil_height: i16,
        floor_tex: &[u8],
        ceil_tex: &[u8],
        light_level: u16,
        typ: u16,
        tag: u16,
    ) -> Sector {
        if floor_tex.len() != 8 {
            panic!(
                "sector floor_tex name incorrect length, expected 8, got {}",
                floor_tex.len()
            )
        }
        if ceil_tex.len() != 8 {
            panic!(
                "sector ceil_tex name incorrect length, expected 8, got {}",
                ceil_tex.len()
            )
        }
        Sector {
            floor_height,
            ceil_height,
            floor_tex: str::from_utf8(floor_tex)
                .expect("Invalid floor tex name")
                .trim_end_matches("\u{0}") // better to address this early to avoid many casts later
                .to_owned(),
            ceil_tex: str::from_utf8(ceil_tex)
                .expect("Invalid ceiling tex name")
                .trim_end_matches("\u{0}") // better to address this early to avoid many casts later
                .to_owned(),
            light_level,
            typ,
            tag,
        }
    }
}

/// A sidedef is a definition of what wall texture(s) to draw along a
/// `LineDef`, and a group of sidedefs outline the space of a `Sector`
///
/// Each `SideDef` record is 30 bytes
#[derive(Debug)]
pub struct SideDef {
    x_offset: i16,
    y_offset: i16,
    /// Name of upper texture used for example in the upper of a window
    upper_tex: String,
    /// Name of lower texture used for example in the front of a step
    lower_tex: String,
    /// The regular part of a wall
    middle_tex: String,
    /// Sector that this sidedef faces or helps to surround
    sector_id: u16,
}

impl SideDef {
    pub fn new(
        x_offset: i16,
        y_offset: i16,
        upper_tex: &[u8],
        lower_tex: &[u8],
        middle_tex: &[u8],
        sector_id: u16,
    ) -> SideDef {
        if upper_tex.len() != 8 {
            panic!(
                "sidedef upper_tex name incorrect length, expected 8, got {}",
                upper_tex.len()
            )
        }
        if lower_tex.len() != 8 {
            panic!(
                "sidedef lower_tex name incorrect length, expected 8, got {}",
                lower_tex.len()
            )
        }
        if middle_tex.len() != 8 {
            panic!(
                "sidedef middle_tex name incorrect length, expected 8, got {}",
                middle_tex.len()
            )
        }
        SideDef {
            x_offset,
            y_offset,
            upper_tex: str::from_utf8(upper_tex)
                .expect("Invalid upper_tex name")
                .trim_end_matches("\u{0}") // better to address this early to avoid many casts later
                .to_owned(),
            lower_tex: str::from_utf8(lower_tex)
                .expect("Invalid lower_tex name")
                .trim_end_matches("\u{0}") // better to address this early to avoid many casts later
                .to_owned(),
            middle_tex: str::from_utf8(middle_tex)
                .expect("Invalid middle_tex name")
                .trim_end_matches("\u{0}") // better to address this early to avoid many casts later
                .to_owned(),
            sector_id,
        }
    }
}

#[derive(Debug, Default)]
pub struct MapExtents {
    pub min_x: i16,
    pub max_x: i16,
    pub min_y: i16,
    pub max_y: i16,
    pub automap_scale: i16,
}

#[derive(Debug)]
pub struct Map {
    name: String,
    vertexes: Vec<Vertex>,
    linedefs: Vec<LineDef>,
    sectors: Vec<Sector>,
    sidedefs: Vec<SideDef>,
    extents: MapExtents,
}

impl Map {
    pub fn new(name: String) -> Map {
        Map {
            name,
            vertexes: Vec::new(),
            linedefs: Vec::new(),
            sectors: Vec::new(),
            sidedefs: Vec::new(),
            extents: MapExtents::default(),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn add_vertex(&mut self, v: Vertex) {
        if self.extents.min_x > v.x {
            self.extents.min_x = v.x;
        } else if self.extents.max_x < v.x {
            self.extents.max_x = v.x;
        }

        if self.extents.min_y > v.y {
            self.extents.min_y = v.y;
        } else if self.extents.max_y < v.y {
            self.extents.max_y = v.y;
        }

        self.vertexes.push(v);
    }

    pub fn get_vertexes(&self) -> &[Vertex] {
        &self.vertexes
    }

    pub fn add_linedef(&mut self, l: LineDef) {
        self.linedefs.push(l);
    }

    pub fn get_linedefs(&self) -> &[LineDef] {
        &self.linedefs
    }

    pub fn add_sector(&mut self, s: Sector) {
        self.sectors.push(s);
    }

    pub fn get_sectors(&self) -> &[Sector] {
        &self.sectors
    }

    pub fn add_sidedef(&mut self, s: SideDef) {
        self.sidedefs.push(s);
    }

    pub fn get_sidedefs(&self) -> &[SideDef] {
        &self.sidedefs
    }

    pub fn get_extents(&self) -> &MapExtents {
        &self.extents
    }
}

#[cfg(test)]
mod tests {
    use crate::map;
    use crate::map::LineDefFlags;
    use crate::wad::Wad;

    #[test]
    fn check_flags_enum() {
        let flag = 28; // upper and lower unpegged, twosided
        println!("Blocking, two-sided, unpeg top and bottom\n{:#018b}", 29);
        println!("Flag: Blocking\n{:#018b}", LineDefFlags::Blocking as u16);
        println!(
            "Flag: Block Monsters\n{:#018b}",
            LineDefFlags::BlockMonsters as u16
        );
        println!("Flag: Two-sided\n{:#018b}", LineDefFlags::TwoSided as u16);
        println!("Flag: Unpeg upper\n{:#018b}", LineDefFlags::UnpegTop as u16);
        println!(
            "Flag: Unpeg lower\n{:#018b}",
            LineDefFlags::UnpegBottom as u16
        );
        println!("Flag: Secret\n{:#018b}", LineDefFlags::Secret as u16);
        println!(
            "Flag: Block sound\n{:#018b}",
            LineDefFlags::BlockSound as u16
        );
        println!(
            "Flag: Not on AutoMap yet\n{:#018b}",
            LineDefFlags::DontDraw as u16
        );
        println!(
            "Flag: Already on AutoMap\n{:#018b}",
            LineDefFlags::Draw as u16
        );
        let compare = LineDefFlags::TwoSided as u16
            | LineDefFlags::UnpegTop as u16
            | LineDefFlags::UnpegBottom as u16;
        assert_eq!(compare, flag);
    }

    #[test]
    fn load_e1m1() {
        let mut wad = Wad::new("../doom1.wad");
        wad.read_directories();

        let mut map = map::Map::new("E1M1".to_owned());
        wad.load_map(&mut map);

        let vertexes = map.get_vertexes();
        assert_eq!(vertexes[0].x, 1088);
        assert_eq!(vertexes[0].y, -3680);
        assert_eq!(vertexes[466].x, 2912);
        assert_eq!(vertexes[466].y, -4848);

        let linedefs = map.get_linedefs();
        assert_eq!(linedefs[0].start_vertex, 0);
        assert_eq!(linedefs[0].end_vertex, 1);
        assert_eq!(linedefs[2].start_vertex, 3);
        assert_eq!(linedefs[2].end_vertex, 0);
        assert_eq!(linedefs[2].front_sidedef, 2);
        assert_eq!(linedefs[2].back_sidedef, 65535);
        assert_eq!(linedefs[474].start_vertex, 384);
        assert_eq!(linedefs[474].end_vertex, 348);
        assert_eq!(linedefs[474].flags, 1);
        assert_eq!(linedefs[474].front_sidedef, 647);
        assert_eq!(linedefs[474].back_sidedef, 65535);

        // Flag check
        assert_eq!(linedefs[26].flags, 29);
        let compare = LineDefFlags::Blocking as u16
            | LineDefFlags::TwoSided as u16
            | LineDefFlags::UnpegTop as u16
            | LineDefFlags::UnpegBottom as u16;
        assert_eq!(linedefs[26].flags, compare);

        let sectors = map.get_sectors();
        assert_eq!(sectors[0].floor_height, 0);
        assert_eq!(sectors[0].ceil_height, 72);
        assert_eq!(sectors[0].floor_tex, "FLOOR4_8");
        assert_eq!(sectors[0].ceil_tex, "CEIL3_5");
        assert_eq!(sectors[0].light_level, 160);
        assert_eq!(sectors[0].typ, 0);
        assert_eq!(sectors[0].tag, 0);
        assert_eq!(sectors[84].floor_height, -24);
        assert_eq!(sectors[84].ceil_height, 48);
        assert_eq!(sectors[84].floor_tex, "FLOOR5_2");
        assert_eq!(sectors[84].ceil_tex, "CEIL3_5");
        assert_eq!(sectors[84].light_level, 255);
        assert_eq!(sectors[84].typ, 0);
        assert_eq!(sectors[84].tag, 0);

        let sidedefs = map.get_sidedefs();
        assert_eq!(sidedefs[0].x_offset, 0);
        assert_eq!(sidedefs[0].y_offset, 0);
        assert_eq!(sidedefs[0].middle_tex, "DOOR3");
        assert_eq!(sidedefs[0].sector_id, 40);
        assert_eq!(sidedefs[9].x_offset, 0);
        assert_eq!(sidedefs[9].y_offset, 48);
        assert_eq!(sidedefs[9].middle_tex, "BROWN1");
        assert_eq!(sidedefs[9].sector_id, 38);
        assert_eq!(sidedefs[647].x_offset, 4);
        assert_eq!(sidedefs[647].y_offset, 0);
        assert_eq!(sidedefs[647].middle_tex, "SUPPORT2");
        assert_eq!(sidedefs[647].sector_id, 70);
    }
}
