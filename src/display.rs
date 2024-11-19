use crate::*;
use std::fmt::{self, Display};

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Command::G1 {
                x,
                y,
                z,
                e,
                f: feed,
            } => {
                let mut out = String::from("G1 ");
                let params = vec![('X', x), ('Y', y), ('Z', z), ('E', e), ('F', feed)];
                for (letter, param) in params {
                    if !param.is_empty() {
                        out += format!("{}{} ", letter, param).as_str();
                    }
                }
                write!(f, "{}", out.trim())
            }
            Command::G90 => write!(f, "G90"),
            Command::G91 => write!(f, "G91"),
            Command::M82 => write!(f, "M82"),
            Command::M83 => write!(f, "M83"),
            Command::Raw(s) => write!(f, "{}", s),
        }
    }
}

impl Display for GCodeLine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.command)?;
        if !self.comments.is_empty() {
            write!(f, ";{}", self.comments)?;
        }
        Ok(())
    }
}

impl Display for GCodeModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for line in &self.lines {
            writeln!(f, "{}\n", line)?;
        }
        Ok(())
    }
}
