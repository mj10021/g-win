pub mod emit;
pub mod file;
pub mod parsers;
mod tests;
use std::{io::Write, path::Path};

#[derive(Clone, Debug, Default, PartialEq)]
struct Counter {
    count: u32,
}

impl Counter {
    fn get(&mut self) -> Id {
        let out = self.count;
        self.count += 1;
        Id(out)
    }
}

#[test]
fn test_counter() {
    let mut c = Counter::default();
    assert_eq!(c.get(), Id(0));
    assert_eq!(c.get(), Id(1));
    assert_eq!(c.get(), Id(2));
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Id(u32);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Label {
    Uninitialized,
    PrePrintMove,
    ExMove,
    Travel,
    FeedrateChange,
    Retraction,
    DeRetraction,
    Wipe,
    LiftZ,
    LowerZ,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    G1(G1),
    G90,
    G91,
    M82,
    M83,
    Raw(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GCodeLine {
    pub id: Id,
    pub command: Command,
    comments: String,
}

// intermediary struct for parsing line into vertex
// exists because all of the params are optional
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct G1 {
    pub x: Option<String>,
    pub y: Option<String>,
    pub z: Option<String>,
    pub e: Option<String>,
    pub f: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GCodeModel {
    pub lines: Vec<GCodeLine>, // keep track of line order
    pub rel_xyz: bool,
    pub rel_e: bool,
    id_counter: Counter,
}

use parsers::GCodeParseError;
impl std::str::FromStr for GCodeModel {
    type Err = GCodeParseError;
    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        let gcode = parsers::gcode_parser(&mut s);
        match gcode {
            Ok(gcode) => Ok(gcode),
            Err(e) => Err(e),
        }
    }
}

#[test]
fn from_str_gcode_test() {
    let gcode = "G1 X1 Y2 Z3 E4 F5\nG1 X1 Y2 Z3 E4 F5\nG1 X1 Y2 Z3 E4 F5\n";
    let gcode_model: GCodeModel = gcode.parse().unwrap();
    assert_eq!(gcode_model.lines.len(), 3);
}

impl GCodeModel {
    pub fn from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let gcode = file::open_gcode_file(path)?;
        Ok(gcode.parse()?)
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
