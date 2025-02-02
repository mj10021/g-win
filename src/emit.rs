use crate::{GCodeLine, GCodeModel, Instruction, G1};

/// Trait objects that can be emitted to valid gcode, with an optional debug line appended
pub trait Emit {
    fn emit(&self, debug: bool) -> String;
}

impl Emit for Instruction {
    fn emit(&self, debug: bool) -> String {
        match self {
            Instruction::G1(g1) => g1.emit(debug),
            Instruction::G90 => "G90".to_string(),
            Instruction::G91 => "G91".to_string(),
            Instruction::M82 => "M82".to_string(),
            Instruction::M83 => "M83".to_string(),
            Instruction::Raw(s) => s.clone(),
        }
    }
}

impl Emit for GCodeLine {
    fn emit(&self, debug: bool) -> String {
        let comments = if self.comments.is_empty() {
            String::from("")
        } else {
            format!(";{}", self.comments)
        };
        self.command.emit(debug) + comments.as_str()
    }
}

impl Emit for G1 {
    fn emit(&self, _debug: bool) -> String {
        let mut out = String::from("G1 ");
        let G1 { x, y, z, e, f, .. } = self;
        let params = vec![('X', x), ('Y', y), ('Z', z), ('E', e), ('F', f)];
        for (letter, param) in params {
            if let Some(param) = param {
                out += format!("{}{} ", letter, f32::from(*param)).as_str();
            }
        }
        out
    }
}

impl Emit for GCodeModel {
    fn emit(&self, debug: bool) -> String {
        self.lines
            .iter()
            .map(|line| line.emit(debug) + "\n")
            .collect()
    }
}
