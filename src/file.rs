use std::path::Path;

// check that path is to a file with the correct extension and read to String
pub fn open_gcode_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    // check path extension
    if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
        match extension {
            "gcode" => return Ok(String::from_utf8(std::fs::read(path)?)?),
            _ => return Err(Box::from(format!("invalid file extension: {}", extension))),
        }
    }
    Err(Box::from("unable to parse file extension"))
}
