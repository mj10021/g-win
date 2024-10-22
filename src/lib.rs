use winnow::{
    ascii::multispace0,
    combinator::{preceded, rest, separated_pair},
    error::InputError,
    token::{literal, one_of, take, take_while},
    PResult, Parser,
};

pub mod emit;
pub mod file;
pub mod parsers;
use std::{collections::{HashMap, HashSet}, str::FromStr};
use std::io::Write;



#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Id(u32);
impl Default for Id {
    fn default() -> Self {
        Id(0)
    }
}
impl Id {
    fn get(&mut self) -> Self {
        let out = self.0;
        self.0 += 1;
        Id(out)
    }
}
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct TextLocation {
    line: u32,
    col: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Span {
    start: TextLocation,
    end: TextLocation
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GCodeLine {
    Unprocessed(Span, Id, String),
    Processed(Span, Id, G1),
}

impl GCodeLine {
    fn id(&self) -> Id {
        match self {
            GCodeLine::Unprocessed(_, id, _) => *id,
            GCodeLine::Processed(_, id, _) => *id,
        }
    } fn span(&self) -> Span {
        match self {
            GCodeLine::Unprocessed(span, _, _) => *span,
            GCodeLine::Processed(span, _, _) => *span,
        }
    }
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
        let mut out = prev.clone();
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

#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub id: Id,
    pub lines: Vec<Id>,
    layer: f32,
}

impl Shape {
    pub fn build(gcode: &mut GCodeModel) -> Self {
        let id = gcode.id_counter.get();
        Shape {
            id,
            lines: Vec::new(),
            layer: -1.0,
        }
    }
    fn get_layer(&mut self, gcode: &GCodeModel) {
        let mut layer = HashMap::new();
        for line in &self.lines {
            let v = gcode.vertices.get(line).unwrap();
            let z = format!("{}", v.to.z);
            layer.entry(z).and_modify(|c| *c += 1).or_insert(1);
        }
        layer
            .iter()
            .collect::<Vec<(&String, &u32)>>()
            .sort_by(|(_, a), (_, b)| a.cmp(b));
        if !layer.is_empty() {
            self.layer = layer.iter().next().unwrap().0.parse().unwrap();
        }
    }
    pub fn _len(&self, gcode: &mut GCodeModel) -> f32 {
        let mut out = 0.0;
        for line in &self.lines {
            if gcode.vertices.contains_key(line) {
                out += gcode.dist_from_prev(line);
            }
        }
        out
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GCodeModel {
    pub lines: Vec<GCodeLine>, // keep track of line order
    pub vertices: HashMap<Id, Vertex>,
    pub shapes: Vec<Shape>,
    pub rel_xyz: bool,
    pub rel_e: bool,
    id_counter: Id,
}
impl FromStr for GCodeModel {
    type Err = Box<dyn std::error::Error>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let gcode: Vec<String> = String::from_utf8(std::fs::read(s)?)?
            .lines()
            .filter_map(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            })
            .collect();
        let mut parsed = Self {
            lines: Vec::new(),
            vertices: HashMap::new(),
            shapes: Vec::new(),
            rel_xyz: false,
            rel_e: true,
            id_counter: Id(0),
        };
        let mut prev = None;
        for line in gcode {
            let command = {
                if let Ok(g1) = g1_parse(&mut line.as_str()) {
                    let id = parsed.id_counter.get();
                    let vrtx = Vertex::build(&mut parsed, prev, g1);
                    prev = Some(id);
                    parsed.vertices.insert(id, vrtx);
                    GCodeLine::Processed(id)
                } else {
                    GCodeLine::Unprocessed(parsed.id_counter.get(), line)
                }
            };
            parsed.lines.push(command);
        }
        parsed.assign_shapes();
        Ok(parsed)
    }
    
}
impl GCodeModel {
    
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let gcode = file::read(path)?;
        let mut parsed = Self {
            lines: Vec::new(),
            vertices: HashMap::new(),
            shapes: Vec::new(),
            rel_xyz: false,
            rel_e: true,
            id_counter: Id(0),
        };
        let mut prev = None;
        for line in gcode {
            let command = {
                if let Ok(g1) = g1_parse(&mut line.as_str()) {
                    let id = parsed.id_counter.get();
                    let vrtx = Vertex::build(&mut parsed, prev, g1);
                    prev = Some(id);
                    parsed.vertices.insert(id, vrtx);
                    GCodeLine::Processed(id)
                } else {
                    GCodeLine::Unprocessed(parsed.id_counter.get(), line)
                }
            };
            parsed.lines.push(command);
        }
        parsed.assign_shapes();
        Ok(parsed)
    }

    pub fn assign_shapes(&mut self) {
        let mut out = Vec::new();
        let mut shape = Shape::build(self);
        for line in &self.lines {
            let line_id = line.id();
            let next_id = self.id_counter.get();
            if let Some(v) = self.vertices.get(&line_id) {
                if v.to.e > 0.0 && self.dist_from_prev(&line_id) > 0.0 {
                    shape.lines.push(line_id);
                } else {
                    shape.get_layer(self);
                    out.push(shape);
                    shape = Shape {
                        id: next_id,
                        lines: Vec::new(),
                        layer: -1.0,
                    };
                }
            }
        }
        if !shape.lines.is_empty() {
            out.push(shape);
        }
        self.shapes = out;
    }
    pub fn get_centroid(&self, vertices: &HashSet<Id>) -> Vec3 {
        let (mut x, mut y, mut z, mut count) = (0.0, 0.0, 0.0, 0.0);
        for vertex in vertices {
            count += 1.0;
            let v = self.vertices.get(vertex).unwrap();
            x += v.to.x;
            y += v.to.y;
            z += v.to.z;
        }
        let mut out = Vec3 { x, y, z };
        out /= count;
        out
    }

    pub fn dist_from_prev(&self, id: &Id) -> f32 {
        let v = self.vertices.get(id).expect("vertex not found in map");
        if let Some(p) = v.prev {
            let p = self
                .vertices
                .get(&p)
                .expect("dist from vertex with no prev");
            p.to.dist(&v.to)
        } else {
            0.0
        }
    }
    pub fn get_flow(&self, id: &Id) -> f32 {
        let v = self.vertices.get(id).expect("vertex not found");
        let dist = self.dist_from_prev(&v.id);
        let flow = v.to.e; // assumes relative extrusion
        flow / dist
    }

    pub fn hole_delete(&mut self, lines_to_delete: &mut HashSet<Id>) {
        for (id, v) in self.vertices.iter_mut() {
            if lines_to_delete.contains(id) {
                v.to.e = 0.0;
            }
        }
    }
    pub fn merge_delete(&mut self, lines_to_delete: &mut HashSet<Id>) {
        let mut temp = Vec::new();

        for line in &self.lines {
            if lines_to_delete.is_empty() {
                break;
            }
            if lines_to_delete.contains(&line.id()) {
                lines_to_delete.remove(&line.id());
                //  keep track of the prev node of the first vertex deleted in a block of verteces
                let (_, vertex) = self
                    .vertices
                    .remove_entry(&line.id())
                    .expect("removing non-existent vertex");
                if let Some(n) = vertex.next {
                    let n = self.vertices.get_mut(&n).unwrap();
                    n.prev = vertex.prev;
                }
                if let Some(p) = vertex.prev {
                    let p = self.vertices.get_mut(&p).unwrap();
                    p.next = vertex.next;
                }
            } else {
                temp.push(line);
            }
        }
    }

    fn insert_lines_before(&mut self, mut lines: Vec<GCodeLine>, id: &Id) {
        let mut i = 0;
        for line in &self.lines {
            if line.id() == *id {
                break;
            }
            i += 1;
        }
        while let Some(line) = lines.pop() {
            self.lines.insert(i, line);
        }
    }
    fn subdivide_vertex(&mut self, id: &Id, count: usize) {
        // FIXME: THIS IS DELETING MOVES
        if count < 1 {
            return;
        }
        // this is assuming relative e
        let v = self.vertices.get(id).unwrap();
        // don't subdivide moves with no extrustion
        if v.label != Label::ExMove {
            return;
        }
        let (xi, yi, zi) = {
            if v.prev.is_none() {
                (0.0, 0.0, 0.0)
            } else {
                let prev = self.vertices.get(&v.prev.unwrap()).unwrap();
                (prev.to.x, prev.to.y, prev.to.z)
            }
        };
        let (xf, yf, zf, ef, f) = (v.to.x, v.to.y, v.to.z, v.to.e, v.to.f);
        let countf = count as f32;
        let (step_x, step_y, step_z) = ((xf - xi) / countf, (yf - yi) / countf, (zf - zi) / countf);
        let mut prev = v.prev;
        let mut vertices = Vec::new();
        let mut new_ids = Vec::new();
        for i in 1..count {
            let i = i as f32;
            let mut new = Vertex {
                id: self.id_counter.get(),
                label: Label::Uninitialized,
                prev,
                to: Pos {
                    x: xi + (step_x * i),
                    y: yi + (step_y * i),
                    z: zi + (step_z * i),
                    e: ef / countf,
                    f,
                },
                next: None, // this gets set as part of set_counts
            };
            new.label(self);
            self.vertices.insert(new.id, new);
            prev = Some(new.id);
            new_ids.push(GCodeLine::Processed(new.id));
            vertices.push(new);
        }
        // i think this is to reset the prev to the last inserted vertex
        for id in &new_ids {
            if let GCodeLine::Processed(id) = id {
                prev = Some(*id);
            }
        }
        self.insert_lines_before(new_ids, id);
        let v = self.vertices.get_mut(id).unwrap();
        v.to.e = ef / countf;
        v.prev = prev;
    }
    pub fn subdivide_vertices(&mut self, vertices: HashSet<Id>, count: usize) {
        for id in vertices {
            self.subdivide_vertex(&id, count);
        }
    }
    // FIXME: add ui for this
    pub fn subdivide_all(&mut self, max_dist: f32) {
        let vertices = self.vertices.clone();
        for id in vertices.keys() {
            if self.vertices.contains_key(id) {
                let dist = self.dist_from_prev(id);
                let count = (dist / max_dist).round() as usize;
                self.subdivide_vertex(id, count);
            }
        }
    }

    pub fn get_shape(&self, vertex: &Id) -> Vec<Id> {
        for shape in self.shapes.iter() {
            if shape.lines.contains(vertex) {
                return shape.lines.clone();
            }
        }
        Vec::new()
    }
    pub fn get_same_z(&self, vertex: &Id) -> Vec<Id> {
        let mut out = Vec::new();
        let z = self.vertices.get(vertex).unwrap().to.z;
        for (_, vertex) in self.vertices.iter() {
            if (vertex.to.z - z).abs() < f32::EPSILON {
                out.push(vertex.id);
            }
        }
        out
    }
    pub fn write_to_file(&self, path: &str) -> Result<(), std::io::Error> {
        use emit::Emit;
        use std::fs::File;
        let out = self.emit(self, false);
        let mut f = File::create(path)?;
        f.write_all(out.as_bytes())?;
        println!("save successful");
        Ok(())
    }
}
// fn insert_before(feature)
// fn modify(feature)
// fn replace_with(feature, gcode_sequence)
// fn insert_after(feature)
