// include readme in docs
#![doc = include_str!("../README.md")]

pub mod emit;
mod file;
mod parsers;
mod tests;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use microns::Microns;
use std::{io::Write, path::Path};
/// Default basic annotations for G1 moves, generated automatically
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Tag {
    Retraction,
    DeRetraction,
    Travel,
    RaiseZ,
    LowerZ,
    Wipe,
    Extrusion,
    Feedrate,
    #[default]
    Uninitialized,
}

/// Struct to store G1 params as optional strings
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct G1 {
    pub x: Option<Microns>,
    pub y: Option<Microns>,
    pub z: Option<Microns>,
    pub e: Option<Microns>,
    pub f: Option<Microns>,
    pub tag: Tag,
}

/// Enum to represent all possible gcode commands that we would
/// like to handle, leaving any unknown commands as raw strings.
/// Specific structs to store information for each command can
/// be added as needed.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    G1(G1),
    G90,
    G91,
    M82,
    M83,
    Raw(String),
}

impl Command {
    pub fn tag(&self) -> Tag {
        match self {
            Command::G1(g1) => g1.tag,
            _ => Tag::Uninitialized,
        }
    }
}

/// Struct to store a single line of gcode, with an id, command,
/// and comments
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GCodeLine {
    pub id: Id,
    pub command: Command,
    pub comments: String,
}

/// Struct to store all information for a .gcode file,
/// specifically calling out relative vs absolute positioning
/// and extrusion and with a counter to generate line ids
///
//~ NOTE: this struct is generated through the FromStr trait
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GCodeModel {
    pub lines: Vec<GCodeLine>, // keep track of line order
    pub rel_xyz: bool,
    pub rel_e: bool,
    pub id_counter: Counter,
}

impl std::str::FromStr for GCodeModel {
    type Err = parsers::GCodeParseError;
    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        let gcode = parsers::gcode_parser(&mut s);
        match gcode {
            Ok(gcode) => Ok(gcode),
            Err(e) => Err(e),
        }
    }
}

impl GCodeModel {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(file::open_gcode_file(path)?.parse()?)
    }
    pub fn write_to_file(&self, path: &Path) -> Result<(), std::io::Error> {
        use emit::Emit;
        use std::fs::File;
        let out = self.emit(false);
        let mut f = File::create(path)?;
        f.write_all(out.as_bytes())?;
        println!("save successful");
        Ok(())
    }
    pub fn tag_g1(&mut self) {
        let mut prev = [
            Microns::ZERO,
            Microns::ZERO,
            Microns::ZERO,
        ];
        for line in self.lines.iter_mut() {
            if let Command::G1(G1 { x, y, z, e, f, tag }) = &mut line.command {
                let curr = [
                    prev[0] + x.unwrap_or(Microns::ZERO),
                    prev[1] + y.unwrap_or(Microns::ZERO),
                    prev[2] + z.unwrap_or(Microns::ZERO),
                ];

                let dx = curr[0] - prev[0];
                let dy = curr[1] - prev[1];
                let dz = curr[2] - prev[2];
                let de = e.unwrap_or(Microns::ZERO);
                let f = f.unwrap_or(Microns::ZERO);

                *tag = {
                    if de > Microns::ZERO {
                        if dx.abs() > Microns::ZERO || dy.abs() > Microns::ZERO {
                            Tag::Extrusion
                        } else { Tag::DeRetraction }
                    } else if de == Microns::ZERO {
                        if dx.abs() > Microns::ZERO || dy.abs() > Microns::ZERO {
                            Tag::Travel
                        } else if dz > Microns::ZERO {
                            Tag::RaiseZ
                        } else if dz < Microns::ZERO {
                            Tag::LowerZ
                        } else if f > Microns::ZERO {
                            Tag::Feedrate
                        } else { Tag::Uninitialized }
                    } else if dx.abs() > Microns::ZERO || dy.abs() > Microns::ZERO {
                            Tag::Wipe
                    } else {
                        Tag::Retraction
                    }
                };
                prev = curr;
            }
        }
    }
}

#[test]
fn tag_test() {
    let mut gcode = GCodeModel::default();
    gcode.lines.push(GCodeLine {
        id: gcode.id_counter.get(),
        command: Command::G1(G1 {
            x: Some(Microns::from(10.0)),
            y: Some(Microns::from(10.0)),
            z: Some(Microns::from(10.0)),
            e: Some(Microns::from(10.0)),
            f: Some(Microns::from(10.0)),
            tag: Tag::Uninitialized,
        }),
        comments: String::new(),
    });
    gcode.tag_g1();
    assert_eq!(gcode.lines[0].command.tag(), Tag::Extrusion);
    gcode.lines.push(GCodeLine {
        id: gcode.id_counter.get(),
        command: Command::G1(G1::default()),
        comments: String::new(),
    });
    gcode.tag_g1();
    assert_eq!(gcode.lines[1].command.tag(), Tag::Uninitialized);
    gcode.lines.push(GCodeLine {
        id: gcode.id_counter.get(),
        command: Command::G1(G1 {
            e: Some(Microns::from(-10.0)),
            ..Default::default()
        }),
        comments: String::new(),
    });
    gcode.tag_g1();
    assert_eq!(gcode.lines[2].command.tag(), Tag::Retraction);
    gcode.lines.push(GCodeLine {
        id: gcode.id_counter.get(),
        command: Command::G1(G1 {
            e: Some(Microns::from(-10.0)),
            x: Some(Microns::from(10.0)),
            y: Some(Microns::from(10.0)),
            ..Default::default()
        }),
        comments: String::new(),
    });
    gcode.tag_g1();
    assert_eq!(gcode.lines[3].command.tag(), Tag::Wipe);
    gcode.lines.push(GCodeLine {
        id: gcode.id_counter.get(),
        command: Command::G1(G1 {
            e: Some(Microns::from(-10.0)),
            z: Some(Microns::from(10.0)),
            ..Default::default()
        }),
        comments: String::new(),
    });
    gcode.tag_g1();
    assert_eq!(gcode.lines[4].command.tag(), Tag::Retraction);
    gcode.lines.push(GCodeLine {
        id: gcode.id_counter.get(),
        command: Command::G1(G1 {
            e: Some(Microns::from(-10.0)),
            z: Some(Microns::from(-10.0)),
            ..Default::default()
        }),
        comments: String::new(),
    });
    gcode.tag_g1();
    assert_eq!(gcode.lines[5].command.tag(), Tag::Retraction);
    gcode.lines.push(GCodeLine {
        id: gcode.id_counter.get(),
        command: Command::G1(G1 {
            f: Some(Microns::from(10.0)),
            ..Default::default()
        }),
        comments: String::new(),
    });
    gcode.tag_g1();
    assert_eq!(gcode.lines[6].command.tag(), Tag::Feedrate);
    gcode.lines.push(GCodeLine {
        id: gcode.id_counter.get(),
        command: Command::G1(G1 {
            e: Some(Microns::from(10.0)),
            ..Default::default()
        }),
        comments: String::new(),
    });
    gcode.tag_g1();
    assert_eq!(gcode.lines[7].command.tag(), Tag::DeRetraction);
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counter {
    count: u32,
}

impl Counter {
    fn get(&mut self) -> Id {
        let out = self.count;
        self.count += 1;
        Id(out)
    }
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Id(u32);

impl Id {
    pub fn get(&self) -> u32 {
        self.0
    }
}
