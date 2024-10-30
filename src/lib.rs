// include readme in doctests
#![doc = include_str!("../README.md")]

mod emit;
mod file;
mod parsers;
mod tests;

use std::{io::Write, path::Path};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
struct G1 {
    x: Option<String>,
    y: Option<String>,
    z: Option<String>,
    e: Option<String>,
    f: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Command {
    G1(G1),
    G90,
    G91,
    M82,
    M83,
    Raw(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct GCodeLine {
    id: Id,
    command: Command,
    comments: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GCodeModel {
    lines: Vec<GCodeLine>, // keep track of line order
    rel_xyz: bool,
    rel_e: bool,
    id_counter: Counter,
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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
struct Id(u32);

