// include readme in doctests
#![doc = include_str!("../README.md")]

mod emit;
mod file;
mod parsers;
mod tests;

use std::{io::Write, path::Path};

/// Struct to store G1 params as optional strings
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct G1 {
    pub x: Option<String>,
    pub y: Option<String>,
    pub z: Option<String>,
    pub e: Option<String>,
    pub f: Option<String>,
}

/// Enum to represent all possible gcode commands that we would
/// like to handle, leaving any unknown commands as raw strings.
/// Specific structs to store information for each command can
/// be added as needed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    G1(G1),
    G90,
    G91,
    M82,
    M83,
    Raw(String),
}

/// Struct to store a single line of gcode, with an id, command,
/// and comments
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
}

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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Id(u32);

