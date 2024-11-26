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
                    if let Some(param) = param {
                        let param = {
                            if param.0 % 1000 == 0 {
                                format!("{}", *param / Microns(1000))
                            } else {
                                let param: f32 = (*param).into();
                                String::from(
                                    format!("{:.3}", param)
                                        .trim_end_matches('0')
                                        .trim_end_matches('.'),
                                )
                            }
                            //format!("{:.3}", param);
                        };
                        out += format!("{}{} ", letter, param).as_str();
                    }
                }
                write!(f, "{}", out.trim())
            }
            Command::G90 => write!(f, "G90"),
            Command::G91 => write!(f, "G91"),
            Command::M82 => write!(f, "M82"),
            Command::M83 => write!(f, "M83"),
            Command::Home(s) | Command::Raw(s) => write!(f, "{}", s),
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
            writeln!(f, "{}", line)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn g1_tests() {
    let g1 = [
        (
            Command::G1 {
                x: Some(Microns(1000)),
                y: Some(Microns(2000)),
                z: Some(Microns(3000)),
                e: Some(Microns(11)),
                f: Some(Microns(5500)),
            },
            "G1 X1 Y2 Z3 E0.011 F5.5",
        ),
        (
            Command::G1 {
                x: None,
                y: None,
                z: None,
                e: None,
                f: None,
            },
            "G1",
        ),
        (
            Command::G1 {
                x: Some(Microns(1111111)),
                y: None,
                z: None,
                e: None,
                f: Some(Microns(-1111111)),
            },
            "G1 X1111.111 F-1111.111",
        ),
    ];
    for (cmd, expected) in g1 {
        assert_eq!(cmd.to_string(), expected);
    }
}

#[test]
fn parse_emit_test() {
    let tests = [
        "G28 W\n",
        "M666\n",
        "UNKNOWN_MACRO\n",
        "special command\n",
        "T0 11\n",
    ];
    for test in tests.iter() {
        let model: GCodeModel = test.parse().unwrap();
        assert_eq!(model.to_string(), *test);
    }
}
