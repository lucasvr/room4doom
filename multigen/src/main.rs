pub mod parse_info;
pub mod strings;

use crate::parse_info::write_info_file;
use gumdrop::Options;
use std::collections::HashMap;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::PathBuf;

// pub struct State {
//     /// Sprite to use
//     pub sprite: SpriteNum,
//     /// The frame within this sprite to show for the state
//     pub frame: u32,
//     /// How many tics this state takes. On nightmare it is shifted >> 1
//     pub tics: i32,
//     // void (*action) (): i32,
//     /// An action callback to run on this state
//     pub action: ActionF,
//     /// The state that should come after this. Can be looped.
//     pub next_state: StateNum,
//     /// Don't know, Doom seems to set all to zero
//     pub misc1: i32,
//     /// Don't know, Doom seems to set all to zero
//     pub misc2: i32,
// }

#[derive(PartialOrd, PartialEq)]
enum LineState {
    StateType,
    InfoType(String),
    None,
}

#[derive(Debug, Clone, Options)]
struct CLIOptions {
    #[options(no_short, meta = "", help = "path to info data")]
    info: PathBuf,
    #[options(no_short, meta = "", help = "path to write generated files to")]
    out: PathBuf,
    #[options(help = "game options help")]
    help: bool,
}

pub type InfoType = HashMap<String, String>;
pub type InfoGroupType = HashMap<String, InfoType>;

fn main() -> Result<(), Box<dyn Error>> {
    let options = CLIOptions::parse_args_default_or_exit();
    let data = read_file(options.info);

    // Lines starting with:
    // - `;` are comments
    // - `$` are MapObjInfo, and may not include all possible fields
    // - `S_` are `StateNum::S_*`, and `State`
    //
    // An `S_` is unique and should accumulate in order
    // `S_` line order: statename  sprite  frame tics action nextstate [optional1] [optional2]
    //
    // SfxEnum are pre-determined?

    let data = parse_data(&data);
    write_info_file(&data.mobj_order, data.mobj_info, options.out);
    Ok(())
}

pub fn read_file(path: PathBuf) -> String {
    let mut file = OpenOptions::new()
        .read(true)
        .open(path.clone())
        .unwrap_or_else(|e| panic!("Couldn't open {:?}, {}", path, e));

    let mut buf = String::new();
    if file
        .read_to_string(&mut buf)
        .unwrap_or_else(|e| panic!("Couldn't read {:?}, {}", path, e))
        == 0
    {
        panic!("File had no data");
    }
    buf
}

pub struct Data {
    sprite_names: Vec<String>, // plain for sprnames
    sprite_enum: Vec<String>,
    states: InfoGroupType, // also convert to enum using key
    mobj_order: Vec<String>,
    mobj_info: InfoGroupType,
}

pub fn parse_data(input: &str) -> Data {
    // K/V = key/mobj name, <K= field, (data, comment)>
    let mut mobj_info: InfoGroupType = HashMap::new();
    let mut states: InfoGroupType = HashMap::new(); // Also used to build StateEnum

    let mut sprite_names = Vec::new();
    let mut sprite_enum = Vec::new();
    let mut mobj_order = Vec::new();
    let mut info_misc_count = 0;
    let mut line_state = LineState::None;

    for line in input.lines() {
        if line.starts_with("S_") {
            let split: Vec<String> = line.split_whitespace().map(|s| s.to_string()).collect();
            if split.len() > 1 {
                states.insert(validate_field(&split[0]), HashMap::new());
                // Sprite enum
                if !sprite_names.contains(&split[1]) {
                    sprite_names.push(split[1].to_uppercase().to_string());
                }
                let en = format!("SpriteNum::SPR_{},", split[1].to_uppercase());
                if !sprite_enum.contains(&en) {
                    sprite_enum.push(en.clone());
                }
                // State data
                if let Some(map) = states.get_mut(&split[0]) {
                    map.insert("sprite".to_string(), en);
                    map.insert("frame".to_string(), split[2].to_string());
                    map.insert("tics".to_string(), split[3].to_string());
                    map.insert("action".to_string(), validate_field(&split[4]));
                    map.insert("next_state".to_string(), validate_field(&split[5]));
                }
            }
            line_state = LineState::StateType;
        }
        if line.starts_with('$') {
            let split: Vec<String> = line.split_whitespace().map(|s| s.to_string()).collect();
            if split[1].contains("DEFAULT") {
                // ignore this one
                continue;
            }
            if split.len() == 2 {
                // A full def
                line_state = LineState::InfoType(split[1].clone());
                mobj_info.insert(split[1].clone(), HashMap::new());
                mobj_order.push(split[1].clone());
            } else {
                // Or one of:
                // if split[1] == "+" {
                // A misc object:
                // $ + doomednum 2023 spawnstate S_PSTR 	flags 	MF_SPECIAL|MF_COUNTITEM
                let mut map = HashMap::new();
                for chunk in split.chunks(2).skip(1) {
                    if chunk[0].starts_with(';') {
                        break;
                    }
                    map.insert(chunk[0].to_string(), validate_field(&chunk[1]));
                }
                let name = if split[1] == "+" {
                    let tmp = format!("MT_MISC{info_misc_count}");
                    info_misc_count += 1;
                    tmp
                } else {
                    split[1].to_string()
                };
                mobj_info.insert(name.clone(), map);
                mobj_order.push(name.clone());
                line_state = LineState::InfoType(name);
            }
        }

        // Multiline info
        if let LineState::InfoType(name) = &mut line_state {
            if line.is_empty() {
                // reset
                line_state = LineState::None;
                continue;
            }
            let split: Vec<String> = line.split_whitespace().map(|s| s.to_string()).collect();
            for chunk in split.chunks(2) {
                if chunk[0].starts_with(';') {
                    break;
                }
                if let Some(entry) = mobj_info.get_mut(name) {
                    entry.insert(chunk[0].clone(), validate_field(&chunk[1]));
                }
            }
        }
    }

    Data {
        sprite_names,
        sprite_enum,
        states,
        mobj_order,
        mobj_info,
    }
}

pub fn validate_field(input: &str) -> String {
    if input.contains("*FRACUNIT") {
        // Convert to something we can parse with f32
        let mut tmp = input.trim_end_matches("*FRACUNIT").to_string();
        tmp.push_str(".0");
        tmp
    } else if input.starts_with("S_") {
        // Stat number
        let mut tmp = "StateNum::".to_string();
        tmp.push_str(input);
        tmp
    } else if input.starts_with("sfx_") {
        // Sound
        let mut tmp = "SfxEnum::".to_string();
        tmp.push_str(capitalize(input.trim_start_matches("sfx_")).as_str());
        tmp
    } else if input.starts_with("MF_") {
        // Flag
        let mut tmp = String::new();
        if input.split('|').count() == 0 {
            let append = input.trim_start_matches("MF_").to_ascii_lowercase();
            tmp.push_str(format!("MapObjFlag::{} as u32", capitalize(&append)).as_str());
        } else {
            for mf in input.split('|') {
                let append = mf.trim_start_matches("MF_").to_ascii_lowercase();
                tmp.push_str(format!("MapObjFlag::{} as u32 |", capitalize(&append)).as_str());
            }
            tmp = tmp.trim_end_matches('|').to_string();
        }
        tmp
    } else if input.starts_with("A_") {
        // Action function
        let lower = input.to_lowercase();
        return if PLAYER_FUNCS.contains(&lower.as_str()) {
            format!("ActionF::Player({lower}),")
        } else {
            format!("ActionF::Actor({lower}),")
        };
    } else {
        input.to_string()
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

const PLAYER_FUNCS: [&str; 22] = [
    "a_bfgsound",
    "a_checkreload",
    "a_closeshotgun2",
    "a_firebfg",
    "a_firecgun",
    "a_firemissile",
    "a_firepistol",
    "a_fireplasma",
    "a_fireshotgun",
    "a_fireshotgun2",
    "a_gunflash",
    "a_light0",
    "a_light1",
    "a_light2",
    "a_loadshotgun2",
    "a_lower",
    "a_openshotgun2",
    "a_punch",
    "a_raise",
    "a_refire",
    "a_saw",
    "a_weaponready",
];
