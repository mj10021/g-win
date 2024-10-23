use crate::{GCodeLine, GCodeModel, Command, G1};

/// Trait objects that can be emitted to valid gcode, with an optional debug line appended
pub trait Emit {
    fn emit(&self, debug: bool) -> String;
}

impl Emit for Command {
    fn emit(&self, debug: bool) -> String {
        match self {
            Command::G1(g1) => g1.emit(debug),
            Command::G28 => "G28".to_string(),
            Command::G90 => "G90".to_string(),
            Command::G91 => "G91".to_string(),
            Command::M82 => "M82".to_string(),
            Command::M83 => "M83".to_string(),
        }
    }
}

impl Emit for GCodeLine {
    fn emit(&self, debug: bool) -> String {
        match self {
            GCodeLine::Unprocessed(_, _, raw_string) => raw_string.clone(),
            GCodeLine::Processed(_, _, command) => command.emit(debug),
        }
    }
}

impl Emit for G1 {
    fn emit(&self, _debug: bool) -> String {
        let mut out = String::new();
        let G1 {x, y, z, e, f, comments} = self;
        let params = [x, y, z, e, f, comments];
        for param in params {
            if let Some(param) = param {
                out += param.as_str();
            }
        }
        out
    }
}

impl Emit for GCodeModel {
    fn emit(&self, debug: bool) -> String {
        let mut out = String::new();
        for line in &self.lines {
            out += line.emit(debug).as_str();
        }
        out
    }
}