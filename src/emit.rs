use crate::GCodeLine;

pub trait Emit {
    fn emit(&self) -> String;
    fn debug(&self) -> String;
}
impl Emit for GCodeLine {
    fn emit(&self) -> String {
        match self {
            GCodeLine::Unprocessed(_, raw_string) => raw_string,
            GCodeLine::Processed(_ g1) => g1.emit(),
        }
    }
    fn debug(&self) -> String {
        
    }
}
impl Emit for Parsed {
    fn emit(&self) -> String {
        
    }
    fn debug(&self) -> String {
        
    }
}
