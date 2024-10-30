#[cfg(test)]
use crate::GCodeModel;
#[cfg(test)]
fn test_gcode_path() -> std::path::PathBuf {
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
    gcode_model.write_to_file(&path).unwrap();
    let gcode = GCodeModel::from_file(&path).unwrap();
    assert_eq!(gcode.lines.len(), 3);
}

#[test]
fn integration_test() {
    // FIXME: this always passes
    let input = test_gcode_path().join("test.gcode");
    let output = test_gcode_path().join("output").join("test_output.gcode");
    let gcode = GCodeModel::from_file(&input).unwrap();
    assert_eq!(gcode.rel_xyz, false);
    assert_eq!(gcode.rel_e, true);
    use crate::emit::Emit;
    use std::fs::File;
    use std::io::Write;
    let mut f = File::create(output.clone()).unwrap();
    f.write_all(gcode.emit(false).as_bytes()).unwrap();
    let gcode2 = GCodeModel::from_file(&output).unwrap();
    let (lines_a, lines_b) = (gcode.lines, gcode2.lines);
    // take a diff of both files
    let set_a = lines_a.iter().collect::<std::collections::HashSet<_>>();
    let set_b = lines_b.iter().collect::<std::collections::HashSet<_>>();
    let _diff = set_a.symmetric_difference(&set_b);
    // assert!(diff.clone().into_iter().count() == 0);
}
