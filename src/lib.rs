pub mod emit;
pub mod file;
pub mod parsers;
use std::io::Write;

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
    pub line_number: usize,
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
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let gcode = file::open_gcode_file(path)?;
        Ok(gcode.parse()?)
    }
    pub fn write_to_file(&self, path: &str) -> Result<(), std::io::Error> {
        use emit::Emit;
        use std::fs::File;
        let out = self.emit(false);
        let mut f = File::create(path)?;
        f.write_all(out.as_bytes())?;
        println!("save successful");
        Ok(())
    }
}

#[cfg(test)]

fn test_gcode_path() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
    std::path::Path::new(&manifest_dir)
        .join("src")
        .join("tests")
}

#[test]
fn write_to_file_test() {
    let gcode = "G1 X1 Y2 Z3 E4 F5\nG1 X1 Y2 Z3 E4 F5\nG1 X1 Y2 Z3 E4 F5\n";
    let gcode_model: GCodeModel = gcode.parse().unwrap();
    let path = test_gcode_path().join("output").join("test_write.gcode");
    gcode_model.write_to_file(path.as_os_str().to_str().unwrap()).unwrap();
    let gcode = GCodeModel::from_file(path.as_os_str().to_str().unwrap()).unwrap();
    assert_eq!(gcode.lines.len(), 3);
}

#[test]
fn integration_test() {
    // FIXME: this always passes 
    let input = test_gcode_path().join("test.gcode");
    let output = test_gcode_path().join("output").join("test_output.gcode");
    let gcode = GCodeModel::from_file(input.as_os_str().to_str().unwrap()).unwrap();
    assert_eq!(gcode.rel_xyz, false);
    assert_eq!(gcode.rel_e, true);
    use crate::emit::Emit;
    use std::fs::File;
    use std::io::Write;
    let mut f = File::create(output.clone()).unwrap();
    f.write_all(gcode.emit(false).as_bytes()).unwrap();
    let gcode2 = GCodeModel::from_file(output.as_os_str().to_str().unwrap()).unwrap();
    let (lines_a, lines_b) = (gcode.lines, gcode2.lines);
    // take a diff of both files
    let set_a = lines_a.iter().collect::<std::collections::HashSet<_>>();
    let set_b = lines_b.iter().collect::<std::collections::HashSet<_>>();
    let _diff = set_a.symmetric_difference(&set_b);
    // assert!(diff.clone().into_iter().count() == 0);
}
