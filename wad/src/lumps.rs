// TODO: Structures, in WAD order
//  - [X] Thing
//  - [X] LineDef
//  - [X] SideDef
//  - [X] Vertex
//  - [X] Segment   (SEGS)
//  - [X] SubSector (SSECTORS)
//  - [X] Node
//  - [X] Sector
//  - [ ] Reject
//  - [ ] Blockmap

use std::str;

use crate::{LumpInfo, WadData};

#[derive(Debug, Copy, Clone, Default)]
pub struct WadColour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl WadColour {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct WadPalette(pub [WadColour; 256]);

impl WadPalette {
    pub fn new() -> Self {
        Self([WadColour::default(); 256])
    }
}

impl Default for WadPalette {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct WadPatch {
    pub width: u16,
    pub height: u16,
    pub left_offset: i16,
    pub top_offset: i16,
    pub columns: Vec<WadPatchCol>,
}

impl WadPatch {
    pub fn from_lump(lump: &LumpInfo, wad: &WadData) -> Self {
        let file = &wad.file_data[lump.handle];
        let width = wad.read_2_bytes(lump.offset, file) as u16;

        let mut columns = Vec::new();
        for q in 0..width {
            let mut offset =
                lump.offset + wad.read_4_bytes((lump.offset + 8) + 4 * q as usize, file) as usize;
            loop {
                let y_offset = wad.read_byte(offset, file) as u32;
                if y_offset == 255 {
                    columns.push(WadPatchCol {
                        y_offset,
                        pixels: Vec::new(),
                    });
                    break;
                }

                offset += 1;
                let len = wad.read_byte(offset, file) as u32;
                offset += 1;
                columns.push(WadPatchCol {
                    y_offset,
                    pixels: (0..len)
                        .map(|_| {
                            offset += 1;
                            wad.read_byte(offset as usize, file) as usize
                        })
                        .collect(),
                });

                offset += 2;
            }
        }

        WadPatch {
            width,
            height: wad.read_2_bytes(lump.offset + 2, file) as u16,
            left_offset: wad.read_2_bytes(lump.offset + 4, file),
            top_offset: wad.read_2_bytes(lump.offset + 6, file),
            columns,
        }
    }
}

/// A column of pixels. Each `pixel` is an index in to the palette to fetch colour.
/// There can be multiple of `WadPatchCol` in a column, and the column itself is
/// ended only when `y_offset` is `0xFF`.
#[derive(Debug, Clone)]
pub struct WadPatchCol {
    /// Determines where on the column the pixel stream starts.
    /// An 0xFF terminates the patch data.
    pub y_offset: u32,
    /// Every `usize` here is an index in to the play palette
    pub pixels: Vec<usize>,
}

/// Contains all the data required to compose a full texture from a series of
/// patches.
#[derive(Debug, Clone)]
pub struct WadTexture {
    /// Texture name
    pub name: String,
    /// Full width of the composed texture
    pub width: u32,
    /// Full height of the composed texture
    pub height: u32,
    /// Collection of `WadTexPatch` which determine where a patch is positioned
    /// in the texture.
    pub patches: Vec<WadTexPatch>,
}

/// Position of a patch, and which patch (via indexto PNAMES) to use.
#[derive(Debug, Clone)]
pub struct WadTexPatch {
    /// Left start position
    pub origin_x: i32,
    /// Top start position
    pub origin_y: i32,
    pub patch_index: usize,
}

/// A `Thing` describes only the position, type, and angle + spawn flags
///
/// The data in the WAD lump is structured as follows:
///
/// | Field Size | Data Type | Content    |
/// |------------|-----------|------------|
/// |  0x00-0x01 |    i16    | X Position |
/// |  0x02-0x03 |    i16    | Y Position |
/// |  0x04-0x05 |    i16    | Angle      |
/// |  0x06-0x07 |    i16    | Type       |
/// |  0x08-0x09 |    i16    | Flags      |
///
/// Each `Thing` record is 10 bytes
// TODO: A `Thing` type will need to be mapped against an enum
#[derive(Debug, Copy, Clone)]
pub struct WadThing {
    pub x: i16,
    pub y: i16,
    pub angle: i16,
    pub kind: i16,
    pub flags: i16,
}

impl WadThing {
    pub fn new(x: i16, y: i16, angle: i16, kind: i16, flags: i16) -> WadThing {
        WadThing {
            x,
            y,
            angle,
            kind,
            flags,
        }
    }
}

/// A `Vertex` is the basic struct used for any type of coordinate
/// in the game
///
/// The data in the WAD lump is structured as follows:
///
/// | Field Size | Data Type | Content      |
/// |------------|-----------|--------------|
/// |  0x00-0x01 |    i16    | X Coordinate |
/// |  0x02-0x03 |    i16    | Y Coordinate |
#[derive(Debug, Default, Clone)]
pub struct WadVertex {
    pub x: i16,
    pub y: i16,
}

impl WadVertex {
    pub fn new(x: i16, y: i16) -> WadVertex {
        WadVertex { x, y }
    }
}

/// Each linedef represents a line from one of the VERTEXES to another.
///
/// The data in the WAD lump is structured as follows:
///
///| Field Size | Data Type      | Content                                   |
///|------------|----------------|-------------------------------------------|
///|  0x00-0x01 | Unsigned short | Start vertex                              |
///|  0x02-0x03 | Unsigned short | End vertex                                |
///|  0x04-0x05 | Unsigned short | Flags (details below)                     |
///|  0x06-0x07 | Unsigned short | Line type / Action                        |
///|  0x08-0x09 | Unsigned short | Sector tag                                |
///|  0x10-0x11 | Unsigned short | Front sidedef ( 0xFFFF side not present ) |
///|  0x12-0x13 | Unsigned short | Back sidedef  ( 0xFFFF side not present ) |
///
/// Each linedef's record is 14 bytes, and is made up of 7 16-bit
/// fields
///
/// A Linedef will always have at least one side. This first side is referred to
/// as either front or right. If you imagine a linedef starting from the bottom
/// of the screen travelling upwards then the right side of this line is the first
/// valid side (and is the front).
#[derive(Debug, Clone)]
pub struct WadLineDef {
    /// The line starts from this point
    pub start_vertex: i16,
    /// The line ends at this point
    pub end_vertex: i16,
    /// The line attributes, see `LineDefFlags`
    pub flags: i16,
    pub special: i16,
    /// This is a number which ties this line's effect type
    /// to all SECTORS that have the same tag number (in their last
    /// field)
    pub sector_tag: i16,
    /// Pointer to the front (right) `SideDef` for this line
    pub front_sidedef: i16,
    /// Pointer to the (left) `SideDef` for this line
    /// If the parsed value == `0xFFFF` means there is no sidedef
    pub back_sidedef: Option<i16>,
}

impl WadLineDef {
    pub fn new(
        start_vertex: i16,
        end_vertex: i16,
        flags: i16,
        line_type: i16,
        sector_tag: i16,
        front_sidedef: i16,
        back_sidedef: Option<i16>,
    ) -> WadLineDef {
        WadLineDef {
            start_vertex,
            end_vertex,
            flags,
            special: line_type,
            sector_tag,
            front_sidedef,
            back_sidedef,
        }
    }
}

/// The Segments (SEGS) are in a sequential order determined by the `SubSector`
/// (SSECTOR), which are part of the NODES recursive tree
///
/// The data in the WAD lump is structured as follows:
///
/// | Field Size | Data Type | Content                              |
/// |------------|-----------|--------------------------------------|
/// |  0x00-0x01 |    i16    | Index to vertex the line starts from |
/// |  0x02-0x03 |    i16    | Index to vertex the line ends with   |
/// |  0x04-0x05 |    i16    | Angle in Binary Angle Measurement (BAMS) |
/// |  0x06-0x07 |    i16    | Index to the linedef this seg travels along|
/// |  0x08-0x09 |    i16    | Direction along line. 0 == SEG is on the right and follows the line, 1 == SEG travels in opposite direction |
/// |  0x10-0x11 |    i16    | Offset: this is the distance along the linedef this seg starts at |
///
/// Each `Segment` record is 12 bytes
#[derive(Debug, Clone)]
pub struct WadSegment {
    /// The line starts from this point
    pub start_vertex: i16,
    /// The line ends at this point
    pub end_vertex: i16,
    /// Binary Angle Measurement
    ///
    /// Degrees(0-360) = angle * 0.005493164
    pub angle: i16,
    /// The Linedef this segment travels along
    pub linedef: i16,
    /// The `side`, 0 = front/right, 1 = back/left
    pub direction: i16,
    /// Offset distance along the linedef (from `start_vertex`) to the start
    /// of this `Segment`
    ///
    /// For diagonal `Segment` offset can be found with:
    /// `DISTANCE = SQR((x2 - x1)^2 + (y2 - y1)^2)`
    pub offset: i16,
}

impl WadSegment {
    pub fn new(
        start_vertex: i16,
        end_vertex: i16,
        angle: i16,
        linedef: i16,
        direction: i16,
        offset: i16,
    ) -> WadSegment {
        WadSegment {
            start_vertex,
            end_vertex,
            angle,
            linedef,
            direction,
            offset,
        }
    }

    // /// True if the right side of the segment faces the point
    // pub fn is_facing_point(&self, point: &WadVertex) -> bool {
    //     let start = &self.start_vertex;
    //     let end = &self.end_vertex;
    //
    //     let d = (end.y - start.y) * (start.x - point.x)
    //         - (end.x - start.x) * (start.y - point.y);
    //     if d <= EPSILON {
    //         return true;
    //     }
    //     false
    // }
}

/// A `SubSector` divides up all the SECTORS into convex polygons. They are then
/// referenced through the NODES resources. There will be (number of nodes) + 1.
///
/// The data in the WAD lump is structured as follows:
///
/// | Field Size | Data Type | Content                            |
/// |------------|-----------|------------------------------------|
/// |  0x00-0x01 |    i16    | How many segments line this sector |
/// |  0x02-0x03 |    i16    | Index to the starting segment      |
///
/// Each `SubSector` record is 4 bytes
#[derive(Debug, Clone)]
pub struct WadSubSector {
    /// How many `Segment`s line this `SubSector`
    pub seg_count: i16,
    /// The `Segment` to start with
    pub start_seg: i16,
}

impl WadSubSector {
    pub fn new(seg_count: i16, start_seg: i16) -> WadSubSector {
        WadSubSector {
            seg_count,
            start_seg,
        }
    }
}

/// A `Sector` is a horizontal (east-west and north-south) area of the level
/// where a floor height and ceiling height is defined.
/// Any change in floor or ceiling height or texture requires a
/// new sector (and therefore separating linedefs and sidedefs).
///
/// Each `Sector` record is 26 bytes
#[derive(Debug, Clone)]
pub struct WadSector {
    pub floor_height: i16,
    pub ceil_height: i16,
    /// Floor texture name
    pub floor_tex: String,
    /// Ceiling texture name
    pub ceil_tex: String,
    /// Light level from 0-255. There are actually only 32 brightnesses
    /// possible so blocks of 8 are the same bright
    pub light_level: i16,
    /// This determines some area-effects called special sectors
    pub kind: i16,
    /// a "tag" number corresponding to LINEDEF(s) with the same tag
    /// number. When that linedef is activated, something will usually
    /// happen to this sector - its floor will rise, the lights will
    /// go out, etc
    pub tag: i16,
}

impl WadSector {
    pub fn new(
        floor_height: i16,
        ceil_height: i16,
        floor_tex: &[u8],
        ceil_tex: &[u8],
        light_level: i16,
        kind: i16,
        tag: i16,
    ) -> WadSector {
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

        WadSector {
            floor_height,
            ceil_height,
            floor_tex: str::from_utf8(floor_tex)
                .unwrap_or_else(|_| panic!("Invalid floor tex name: {:?}", floor_tex))
                .trim_end_matches('\u{0}') // better to address this early to avoid many casts later
                .to_owned(),
            ceil_tex: str::from_utf8(ceil_tex)
                .expect("Invalid ceiling tex name")
                .trim_end_matches('\u{0}') // better to address this early to avoid many casts later
                .to_owned(),
            light_level,
            kind,
            tag,
        }
    }
}

/// A sidedef is a definition of what wall texture(s) to draw along a
/// `LineDef`, and a group of sidedefs outline the space of a `Sector`
///
/// Each `SideDef` record is 30 bytes
#[derive(Debug, Clone)]
pub struct WadSideDef {
    pub x_offset: i16,
    pub y_offset: i16,
    /// Name of upper texture used for example in the upper of a window
    pub upper_tex: String,
    /// Name of lower texture used for example in the front of a step
    pub lower_tex: String,
    /// The regular part of a wall
    pub middle_tex: String,
    /// Sector that this sidedef faces or helps to surround
    pub sector: i16,
}

impl WadSideDef {
    pub fn new(
        x_offset: i16,
        y_offset: i16,
        upper_tex: &[u8],
        lower_tex: &[u8],
        middle_tex: &[u8],
        sector: i16,
    ) -> WadSideDef {
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
        WadSideDef {
            x_offset,
            y_offset,
            upper_tex: str::from_utf8(upper_tex)
                .expect("Invalid upper_tex name")
                .trim_end_matches('\u{0}') // better to address this early to avoid many casts later
                .to_owned(),
            lower_tex: str::from_utf8(lower_tex)
                .expect("Invalid lower_tex name")
                .trim_end_matches('\u{0}') // better to address this early to avoid many casts later
                .to_owned(),
            middle_tex: str::from_utf8(middle_tex)
                .expect("Invalid middle_tex name")
                .trim_end_matches('\u{0}') // better to address this early to avoid many casts later
                .to_owned(),
            sector,
        }
    }
}

/// The base node structure as parsed from the WAD records. What is stored in the WAD
/// is the splitting line used for splitting the level/node (starts with the level then
/// consecutive nodes, aiming for an even split if possible), a box which encapsulates
/// the left and right regions of the split, and the index numbers for left and right
/// children of the node; the index is in to the array built from this lump.
///
/// **The last node is the root node**
///
/// The data in the WAD lump is structured as follows:
///
/// | Field Size | Data Type                            | Content                                          |
/// |------------|--------------------------------------|--------------------------------------------------|
/// | 0x00-0x03  | Partition line x coordinate          | X coordinate of the splitter                     |
/// | 0x02-0x03  | Partition line y coordinate          | Y coordinate of the splitter                     |
/// | 0x04-0x05  | Change in x to end of partition line | The amount to move in X to reach end of splitter |
/// | 0x06-0x07  | Change in y to end of partition line | The amount to move in Y to reach end of splitter |
/// | 0x08-0x09  | Right (Front) box top                | First corner of front box (Y coordinate)         |
/// | 0x0A-0x0B  | Right (Front)  box bottom            | Second corner of front box (Y coordinate)        |
/// | 0x0C-0x0D  | Right (Front)  box left              | First corner of front box (X coordinate)         |
/// | 0x0E-0x0F  | Right (Front)  box right             | Second corner of front box (X coordinate)        |
/// | 0x10-0x11  | Left (Back) box top                  | First corner of back box (Y coordinate)          |
/// | 0x12-0x13  | Left (Back)  box bottom              | Second corner of back box (Y coordinate)         |
/// | 0x14-0x15  | Left (Back)  box left                | First corner of back box (X coordinate)          |
/// | 0x16-0x17  | Left (Back)  box right               | Second corner of back box (X coordinate)         |
/// | 0x18-0x19  | Right (Front) child index            | Index of the front child + sub-sector indicator  |
/// | 0x1A-0x1B  | Left (Back)  child index             | Index of the back child + sub-sector indicator   |
#[derive(Debug, Clone)]
pub struct WadNode {
    /// Where the line used for splitting the level starts
    pub x: i16,
    pub y: i16,
    /// Where the line used for splitting the level ends
    pub dx: i16,
    pub dy: i16,
    /// Coordinates of the bounding boxes:
    pub bounding_boxes: [[i16; 4]; 2],
    /// The node children. Doom uses a clever trick where if one node is selected
    /// then the other can also be checked with the same/minimal code by inverting
    /// the last bit
    pub child_index: [u16; 2],
}

impl WadNode {
    pub fn new(
        x: i16,
        y: i16,
        dx: i16,
        dy: i16,
        bounding_boxes: [[i16; 4]; 2],
        right_child_id: u16,
        left_child_id: u16,
    ) -> WadNode {
        WadNode {
            x,
            y,
            dx,
            dy,
            bounding_boxes,
            child_index: [right_child_id, left_child_id],
        }
    }
}

/// The `BLOCKMAP` is a pre-calculated structure that the game engine uses to simplify
/// collision-detection between moving things and walls.
///
/// Each "block" is 128 square.
#[derive(Debug, Clone)]
pub struct WadBlockMap {
    /// Leftmost X coord, this is 16.16 fixed point, doing an `((i as i32)<<16) as f32` will convert
    pub x_origin: i16,
    /// Bottommost Y coord, this is 16.16 fixed point, doing an `((i as i32)<<16) as f32` will convert
    pub y_origin: i16,
    /// Width
    pub width: i16,
    /// Height
    pub height: i16,
    /// The line index is used by converting a local X.Y coordinate in to an offset in to this array.
    /// The number at that location is then the index number in to the linedefs array.
    pub line_indexes: Vec<i16>,
    /// Blockmap Index start
    pub blockmap_offset: usize,
}

impl WadBlockMap {
    pub fn new(
        x_origin: i16,
        y_origin: i16,
        width: i16,
        height: i16,
        lines: Vec<i16>,
        blockmap_idx: usize,
    ) -> WadBlockMap {
        WadBlockMap {
            x_origin,
            y_origin,
            width,
            height,
            line_indexes: lines,
            blockmap_offset: blockmap_idx,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::WadData;

    #[test]
    fn texture1_header_0() {
        let wad = WadData::new("../doom1.wad".into());
        let lump = wad.find_lump_or_panic("TEXTURE1");
        assert_eq!(lump.name, "TEXTURE1");
        assert_eq!(lump.size, 9234);
        assert_eq!(lump.offset, 977748);

        let file = &wad.file_data[lump.handle];
        let tex_count = wad.read_4_bytes(lump.offset, file);
        assert_eq!(tex_count, 125);

        let mut tex_offsets = Vec::new();
        for i in 0..tex_count as usize {
            tex_offsets
                .push(lump.offset + wad.read_4_bytes(lump.offset + 4 + 4 * i, file) as usize);
        }

        // Read texture name
        let mut n = [0u8; 8]; // length is 8 slots total
        for (i, slot) in n.iter_mut().enumerate() {
            *slot = file[tex_offsets[0] + i];
        }
        let name = std::str::from_utf8(&n)
            .expect("Invalid lump name")
            .trim_end_matches('\u{0}')
            .to_owned();
        assert_eq!(name.as_str(), "AASTINKY");

        // offset + 4, ignored
        let tex0_width = wad.read_2_bytes(tex_offsets[0] + 12, file);
        let tex0_height = wad.read_2_bytes(tex_offsets[0] + 14, file);
        assert_eq!(tex0_width, 24);
        assert_eq!(tex0_height, 72);
        // offset + 4, ignored
        // Patch count tells how many blocks of 10 bytes to read
        let tex0_patch_count = wad.read_2_bytes(tex_offsets[0] + 20, file);
        assert_eq!(tex0_patch_count, 2);

        // Multiple blocks (n = patch_count)
        // And then patch_count * block of 10 bytes
        // Each block is a patch layout to form the texture
        let tex0_h_offset = wad.read_2_bytes(tex_offsets[0] + 22, file);
        let tex0_v_offset = wad.read_2_bytes(tex_offsets[0] + 24, file);
        // Patch from PNAMES index
        let tex0_p_index = wad.read_2_bytes(tex_offsets[0] + 26, file);
        assert_eq!(tex0_h_offset, 0);
        assert_eq!(tex0_v_offset, 0);
        assert_eq!(tex0_p_index, 0);

        let tex0_h_offset = wad.read_2_bytes(tex_offsets[0] + 22 + 10, file);
        let tex0_v_offset = wad.read_2_bytes(tex_offsets[0] + 24 + 10, file);
        let tex0_p_index = wad.read_2_bytes(tex_offsets[0] + 26 + 10, file);
        assert_eq!(tex0_h_offset, 12);
        assert_eq!(tex0_v_offset, -6);
        assert_eq!(tex0_p_index, 0);

        // The last in the list
        let mut n = [0u8; 8]; // length is 8 slots total
        for (i, slot) in n.iter_mut().enumerate() {
            *slot = file[tex_offsets[tex_count as usize - 1] + i];
        }
        let name = std::str::from_utf8(&n)
            .expect("Invalid lump name")
            .trim_end_matches('\u{0}')
            .to_owned();
        assert_eq!(name.as_str(), "TEKWALL5");
    }

    #[test]
    fn pnames_array() {
        let wad = WadData::new("../doom1.wad".into());
        let lump = wad.find_lump_or_panic("PNAMES");
        assert_eq!(lump.name, "PNAMES");
        assert_eq!(lump.size, 2804);
        assert_eq!(lump.offset, 986984);

        let file = &wad.file_data[lump.handle];

        let patch_count = wad.read_4_bytes(lump.offset, file);
        assert_eq!(patch_count, 350);

        let mut n = [0u8; 8]; // length is 8 slots total
        for (i, slot) in n.iter_mut().enumerate() {
            *slot = file[lump.offset + 4 + i];
        }
        let name = std::str::from_utf8(&n)
            .expect("Invalid lump name")
            .trim_end_matches('\u{0}')
            .to_owned();
        assert_eq!(name.as_str(), "WALL00_3");

        let mut n = [0u8; 8]; // length is 8 slots total
        for (i, slot) in n.iter_mut().enumerate() {
            *slot = file[lump.offset + 4 + 8 + i];
        }
        let name = std::str::from_utf8(&n)
            .expect("Invalid lump name")
            .trim_end_matches('\u{0}')
            .to_owned();
        assert_eq!(name.as_str(), "W13_1");

        let mut n = [0u8; 8]; // length is 8 slots total
        for (i, slot) in n.iter_mut().enumerate() {
            *slot = file[lump.offset + 4 + 16 + i];
        }
        let name = std::str::from_utf8(&n)
            .expect("Invalid lump name")
            .trim_end_matches('\u{0}')
            .to_owned();
        assert_eq!(name.as_str(), "DOOR2_1");
    }

    #[test]
    #[ignore = "Registered Doom only"]
    fn texture2_header() {
        let wad = WadData::new("../doom.wad".into());
        let lump = wad.find_lump_or_panic("TEXTURE2");
        assert_eq!(lump.name, "TEXTURE2");
        assert_eq!(lump.size, 8036);
        assert_eq!(lump.offset, 3690828);

        let file = &wad.file_data[lump.handle];
        let tex_count = wad.read_4_bytes(lump.offset, file);
        assert_eq!(tex_count, 162);

        let mut tex_offsets = Vec::new();
        for i in 0..tex_count as usize {
            tex_offsets
                .push(lump.offset + wad.read_4_bytes(lump.offset + 4 + 4 * i, file) as usize);
        }

        let mut n = [0u8; 8]; // length is 8 slots total
        for (i, slot) in n.iter_mut().enumerate() {
            *slot = file[tex_offsets[0] + i];
        }
        let name = std::str::from_utf8(&n)
            .expect("Invalid lump name")
            .trim_end_matches('\u{0}')
            .to_owned();
        assert_eq!(name.as_str(), "ASHWALL");

        let mut n = [0u8; 8]; // length is 8 slots total
        for (i, slot) in n.iter_mut().enumerate() {
            *slot = file[tex_offsets[tex_count as usize - 1] + i];
        }
        let name = std::str::from_utf8(&n)
            .expect("Invalid lump name")
            .trim_end_matches('\u{0}')
            .to_owned();
        assert_eq!(name.as_str(), "WOODSKUL");

        //assert_eq!(lump.offset as i32 + tex_offsets[0], lump.offset as i32 + 4);
    }
}
