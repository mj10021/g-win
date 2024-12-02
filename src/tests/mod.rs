#![cfg(test)]
use crate::GCodeModel;
pub fn test_gcode_path() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
    std::path::Path::new(&manifest_dir)
        .join("src")
        .join("tests")
}

#[test]
fn write_to_file_test() {
    let gcode = "G1 X1 Y2 Z3 E4 F5\nG1 X1 Y2 Z3 E4 F5\nG1 X1 Y2 Z3 E4 F5\n";
    let gcode_model: GCodeModel = gcode.parse().unwrap();
    let path = test_gcode_path().join("output").join("test_write.gcode");
    let output = gcode_model.to_string();
    std::fs::write(path.as_path(), output).unwrap();
    let gcode = GCodeModel::try_from(path.as_path()).unwrap();
    assert_eq!(gcode.lines.len(), 3);
}

#[test]
fn integration_test() {
    // FIXME: this always passes
    let input = test_gcode_path().join("test.gcode");
    let output = test_gcode_path().join("output").join("test_output.gcode");
    let gcode = GCodeModel::try_from(input.as_path()).unwrap();
    use std::fs::File;
    use std::io::Write;
    let mut f = File::create(output.clone()).unwrap();
    f.write_all(gcode.to_string().as_bytes()).unwrap();
    let gcode2 = GCodeModel::try_from(output.as_path()).unwrap();
    let (lines_a, lines_b) = (gcode.lines, gcode2.lines);
    // take a diff of both files
    let set_a = lines_a.iter().collect::<std::collections::HashSet<_>>();
    let set_b = lines_b.iter().collect::<std::collections::HashSet<_>>();
    let _diff = set_a.symmetric_difference(&set_b);
    // assert!(diff.clone().into_iter().count() == 0);
}

#[test]
fn from_str_gcode_test() {
    let gcode = "G1 X1 Y2 Z3 E4 F5\nG1 X1 Y2 Z3 E4 F5\nG1 X1 Y2 Z3 E4 F5\n";
    let gcode_model: GCodeModel = gcode.parse().unwrap();
    assert_eq!(gcode_model.lines.len(), 3);
}
