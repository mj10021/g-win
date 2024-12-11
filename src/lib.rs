// include readme in docs
#![doc = include_str!("../README.md")]

pub mod analyzer;
mod display;
mod parsers;
pub mod state;
mod tests;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::{
    io::BufReader,
    path::{Path, PathBuf},
};

use microns::*;

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
    /// micrometers (first layer, rest of layers((0, 0) if nonplanar))
    layer_height: (Microns, Microns),
}

impl Default for PrintMetadata {
    fn default() -> Self {
        PrintMetadata {
            relative_e: false,
            relative_xyz: false,
            layer_height: (Microns::ZERO, Microns::ZERO),
        }
    }
}

impl From<&GCodeModel> for PrintMetadata {
    fn from(gcode: &GCodeModel) -> Self {
        let mut cursor = analyzer::Cursor::from(gcode);
        PrintMetadata {
            relative_e: true,
            relative_xyz: false,
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
    pub metadata: PrintMetadata,
}

impl GCodeModel {
    pub fn get(&self, idx: usize) -> Option<&GCodeLine> {
        self.lines.get(idx)
    }
}
