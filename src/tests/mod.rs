#![cfg(test)]
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;

use crate::GCodeModel;
pub fn test_gcode_path() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
    std::path::Path::new(&manifest_dir)
        .join("src")
        .join("tests")
}

fn temp_dir() -> std::path::PathBuf {
    test_gcode_path().join("temp")
}

fn test_gcode(input: &str) -> PathBuf {
    let path = temp_dir().join("test.gcode");
    let mut file = File::create_new(path.as_path()).unwrap();
    file.write_all(input.as_bytes()).unwrap();
    path
}


#[test]
fn parse_gcode() {
    let test = BufReader::new("
    G28
    G666; FAKE COMMAND
    FAKE_MACRO FAKE_PARAM 299.00
    G1234 P12.88
    G1 Y-1 Z0.2
    G1 X25 E15.5
    G1 X10 E10
    G1 Z2.0 E-0.2
    G1 M104 S200
    G1 X50 Y50
    G1 E1.23
    G1 Y100 E10
    ".as_bytes());
    let gcode = GCodeModel::try_from(test).unwrap();
    panic!("{:#?}", gcode);
}