use crate::{Command, GCodeLine, GCodeModel};

/// Trait objects that can be emitted to valid gcode, with an optional debug line appended
pub trait Emit {
    fn emit(&self, debug: bool) -> String;
}

impl ToString for Command {
    fn to_string(&self) -> String {
        match self {
            Command::G1{x, y, z, e, f} => {
                let mut out = String::from("G1 ");
                    let params = vec![('X', x), ('Y', y), ('Z', z), ('E', e), ('F', f)];
                    for (letter, param) in params {
                        if !param.is_empty() {
                            out += format!("{}{} ", letter, param).as_str();
                        }
                    }
                out
            }
            Command::G90 => "G90".to_string(),
            Command::G91 => "G91".to_string(),
            Command::M82 => "M82".to_string(),
            Command::M83 => "M83".to_string(),
            Command::Raw(s) => s.clone(),
        }
    }
}

impl ToString for GCodeLine {
    fn to_string(&self) -> String {
        let comments = if self.comments.is_empty() {
            String::from("")
        } else {
            format!(";{}", self.comments)
        };
        self.command.to_string() + comments.as_str()
    }
}

impl ToString for GCodeModel {
    fn to_string(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.to_string() + "\n")
            .collect()
    }
}
