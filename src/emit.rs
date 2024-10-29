use crate::{Command, GCodeLine, GCodeModel, G1};

/// Trait objects that can be emitted to valid gcode, with an optional debug line appended
pub trait Emit {
    fn emit(&self, debug: bool) -> String;
}

impl Emit for Command {
    fn emit(&self, debug: bool) -> String {
        match self {
            Command::G1(g1) => g1.emit(debug),
            Command::G90 => "G90".to_string(),
            Command::G91 => "G91".to_string(),
            Command::M82 => "M82".to_string(),
            Command::M83 => "M83".to_string(),
            Command::Unsupported(s) => s.clone(),
        }
    }
}

impl Emit for GCodeLine {
    fn emit(&self, debug: bool) -> String {
        self.command.emit(debug) + self.comments.as_str()
    }
}

impl Emit for G1 {
    fn emit(&self, _debug: bool) -> String {
        let mut out = String::from("G1 ");
        let G1 {
            x,
            y,
            z,
            e,
            f,
        } = self;
        let params = vec![('X', x), ('Y', y), ('Z', z), ('E', e), ('F', f)];
        for (letter, param) in params {
            if let Some(param) = param {
                out += format!("{}{} ", letter, param).as_str();
            }
        }
        out
    }
}

impl Emit for GCodeModel {
    fn emit(&self, debug: bool) -> String {
        self.lines.iter().map(|line| line.emit(debug)).collect::<Vec<_>>().join("\n")
    }
}
