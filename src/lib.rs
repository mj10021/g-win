// include readme in docs
#![doc = include_str!("../README.md")]

pub mod analyzer;
mod display;
pub mod microns;
mod parsers;
mod tests;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::{io::Write, path::Path};

pub use microns::Microns;

// check that path is to a file with the correct extension and read to String
fn open_gcode_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    // check path extension
    if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
        match extension {
            "gcode" => return Ok(String::from_utf8(std::fs::read(path)?)?),
            _ => return Err(Box::from(format!("invalid file extension: {}", extension))),
        }
    }
    Err(Box::from("unable to parse file extension"))
}

#[test]
fn open_gcode_file_test() {
    let path = Path::new("src/tests/test.gcode");
    let _ = open_gcode_file(path).unwrap();
}

/// Represent all possible gcode commands that we would
/// like to handle, leaving any unknown commands as raw strings.
/// Specific structs to store information for each command can
/// be added as needed.
///
/// G1 params are parsed into floats later on so that the enum can
/// implement Eq and Hash
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    G1 {
        x: Option<Microns>,
        y: Option<Microns>,
        z: Option<Microns>,
        e: Option<Microns>,
        f: Option<Microns>,
    },
    G90,
    G91,
    M82,
    M83,
    Home(String),
    Raw(String),
}

/// Store a single line of gcode, with an id, command,
/// and comments
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GCodeLine {
    pub command: Command,
    pub comments: String,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrintMetadata {
    relative_e: bool,
    relative_xyz: bool,
    preprint: std::ops::RangeInclusive<usize>,
    postprint: std::ops::RangeInclusive<usize>,
    /// micrometers (first layer, rest of layers((0, 0) if nonplanar))
    layer_height: (Microns, Microns),
}

impl Default for PrintMetadata {
    fn default() -> Self {
        PrintMetadata {
            relative_e: false,
            relative_xyz: false,
            preprint: 0..=0,
            postprint: 0..=0,
            layer_height: (Microns::ZERO, Microns::ZERO),
        }
    }
}

impl From<&GCodeModel> for PrintMetadata {
    fn from(gcode: &GCodeModel) -> Self {
        let mut cursor = analyzer::Cursor::from(gcode);
        PrintMetadata {
            preprint: cursor.pre_print().unwrap_or(0..=0),
            postprint: cursor.post_print().unwrap_or(0..=0),
            relative_e: gcode.rel_e,
            relative_xyz: gcode.rel_xyz,
            layer_height: cursor.layer_height(),
        }
    }
}
/// Store all information for a .gcode file,
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
    pub metadata: PrintMetadata,
}

impl std::str::FromStr for GCodeModel {
    type Err = parsers::GCodeParseError;
    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        let gcode = parsers::gcode_parser(&mut s);
        match gcode {
            Ok(mut gcode) => {
                let metadata = PrintMetadata::from(&gcode);
                gcode.metadata = metadata;
                Ok(gcode)
            }
            Err(e) => Err(e),
        }
    }
}

impl GCodeModel {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(open_gcode_file(path)?.parse()?)
    }
    pub fn write_to_file(&self, path: &Path) -> Result<(), std::io::Error> {
        use std::fs::File;
        let out = self.to_string();
        let mut f = File::create(path)?;
        f.write_all(out.as_bytes())?;
        println!("save successful");
        Ok(())
    }
}
