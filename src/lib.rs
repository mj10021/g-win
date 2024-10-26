pub mod emit;
pub mod file;
pub mod parsers;
use std::io::Write;

#[derive(Clone, Debug, Default, PartialEq)]
struct Counter {
    count: u32,
}

impl Counter {
    fn new() -> Self {
        Counter { count: 0 }
    }
    fn get(&mut self) -> Id {
        let out = self.count;
        self.count += 1;
        Id(out)
    }
}

#[test]
fn test_counter() {
    let mut c = Counter::new();
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
    G28,
    G90,
    G91,
    M82,
    M83,
    Unsupported(String),
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
    pub comments: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GCodeModel {
    pub lines: Vec<GCodeLine>, // keep track of line order
    pub rel_xyz: bool,
    pub rel_e: bool,
    id_counter: Counter,
}

impl GCodeModel {
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
