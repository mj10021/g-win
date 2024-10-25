#![allow(dead_code)]

pub mod emit;
pub mod file;
pub mod parsers;
use std::{collections::HashMap, io::Write, ops::Range};

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

impl G1 {
    fn build(params: Vec<(&str, String)>) -> G1 {
        let mut out = G1::default();
        for param in params {
            match param {
                ("X", val) => out.x = Some(val),
                ("Y", val) => out.y = Some(val),
                ("Z", val) => out.z = Some(val),
                ("E", val) => out.e = Some(val),
                ("F", val) => out.f = Some(val),
                (comment, _) => out.comments = Some(comment.to_owned()),
            }
        }
        out
    }
    fn params(&self) -> Vec<(&str, String)> {
        let mut out = Vec::new();
        if let Some(x) = &self.x {
            out.push(("X", x.clone()));
        };
        if let Some(y) = &self.y {
            out.push(("Y", y.clone()));
        };
        if let Some(z) = &self.z {
            out.push(("Z", z.clone()));
        };
        if let Some(e) = &self.e {
            out.push(("E", e.clone()));
        };
        if let Some(f) = &self.f {
            out.push(("F", f.clone()));
        };
        out
    }
}
// state tracking struct for vertices
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pos {
    // abs x, y, z and rel e
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub e: f32,
    pub f: f32,
}

impl Pos {
    fn from_g1(g1: &G1, prev: Option<Pos>) -> Self {
        let prev = {
            if let Some(prev) = prev {
                prev
            } else {
                Pos::home()
            }
        };
        let mut out = prev;
        for param in g1.params() {
            match param {
                ("X", val) => out.x = val.parse().unwrap(),
                ("Y", val) => out.y = val.parse().unwrap(),
                ("Z", val) => out.z = val.parse().unwrap(),
                ("E", val) => out.e = val.parse().unwrap(),
                ("F", val) => out.f = val.parse().unwrap(),
                _ => (),
            }
        }
        out
    }
    pub fn home() -> Pos {
        Pos {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            e: 0.0,
            f: f32::NEG_INFINITY, // this will not emit if a feedrate is never set
        }
    }
    pub fn dist(&self, p: &Pos) -> f32 {
        ((self.x - p.x).powf(2.0) + (self.y - p.y).powf(2.0) + (self.z - p.z).powf(2.0)).sqrt()
    }
}
fn pre_home(p: Pos) -> bool {
    if p.x == f32::NEG_INFINITY
        || p.y == f32::NEG_INFINITY
        || p.z == f32::NEG_INFINITY
        || p.e == f32::NEG_INFINITY
    {
        return true;
    }
    false
}
#[derive(Clone, Copy, PartialEq)]
pub struct Vertex {
    pub id: Id,
    pub label: Label,
    // this is the id of the previous extrusion move
    pub prev: Option<Id>,
    // this is the id of the next extrusion move
    pub next: Option<Id>,
    pub to: Pos,
}
impl std::fmt::Debug for Vertex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vertex")
            .field("label", &self.label)
            .field("to", &self.to)
            .finish()
    }
}

impl Vertex {
    fn build(parsed: &mut GCodeModel, prev: Option<Id>, g1: G1) -> Vertex {
        let id = parsed.id_counter.get();
        if prev.is_none() {
            let mut vrtx = Self {
                id,
                label: Label::Uninitialized,
                to: Pos::from_g1(&g1, None),
                prev,
                next: None,
            };
            vrtx.label(parsed);
            return vrtx;
        }
        let p = parsed.vertices.get_mut(&prev.unwrap()).unwrap();
        let mut vrtx = Vertex {
            id,
            label: Label::Uninitialized,
            to: Pos::from_g1(&g1, Some(p.to)),
            prev,
            next: p.next,
        };
        p.next = Some(id);
        vrtx.label(parsed);
        vrtx
    }
    pub fn get_from(&self, parsed: &GCodeModel) -> Pos {
        if let Some(prev) = self.prev {
            parsed.vertices.get(&prev).unwrap().to
        } else {
            Pos::home()
        }
    }
    fn label(&mut self, parsed: &GCodeModel) {
        let from = self.get_from(parsed);
        let dx = self.to.x - from.x;
        let dy = self.to.y - from.y;
        let dz = self.to.z - from.z;
        let de = self.to.e;
        self.label = {
            if self.to.x < 5.0 || self.to.y < 5.0 {
                Label::PrePrintMove
            } else if de > 0.0 {
                if dx.abs() + dy.abs() > 0.0 - f32::EPSILON {
                    Label::ExMove
                } else {
                    Label::DeRetraction
                }
            } else if dz.abs() > f32::EPSILON {
                if dz < 0.0 {
                    Label::LowerZ
                } else {
                    Label::LiftZ
                }
            } else if de.abs() > f32::EPSILON {
                if dx.abs() + dy.abs() > f32::EPSILON {
                    Label::Wipe
                } else {
                    Label::Retraction
                }
            } else if dx.abs() + dy.abs() > f32::EPSILON {
                Label::Travel
            } else if from.f != self.to.f {
                Label::FeedrateChange
            } else {
                Label::Uninitialized
            }
        };
    }
    pub fn change_move(&self) -> bool {
        self.label == Label::LiftZ || self.label == Label::Wipe || self.label == Label::Retraction
    }
    pub fn extrusion_move(&self) -> bool {
        self.label == Label::ExMove
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GCodeModel {
    pub lines: Vec<GCodeLine>, // keep track of line order
    pub vertices: HashMap<Id, Vertex>,
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
