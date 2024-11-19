use std::ops::RangeInclusive;

use crate::{Command, GCodeModel};

fn calc_slope(a: [f32; 5], b: [f32; 5]) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    dy / dx
}
fn is_extrusion(curr: [f32; 5], prev: [f32; 5]) -> bool {
    if curr[3] > f32::EPSILON {
        return (curr[0] - prev[0]).abs() > f32::EPSILON || (curr[1] - prev[1]).abs() > f32::EPSILON || (curr[2] - prev[2]).abs() > f32::EPSILON;
    }
    false
}


#[derive(Clone, Copy)]
pub struct Cursor<'a> {
    parent: &'a GCodeModel,
    idx: usize,
    state: [f32; 5],
    prev: [f32; 5],
    curr_command: &'a Command,
}

impl<'a> From<&'a GCodeModel> for Cursor<'a> {
    fn from(parent: &'a GCodeModel) -> Self {
        let mut cursor = Cursor {
            parent,
            idx: 0,
            state: [0.0; 5],
            prev: [0.0; 5],
            curr_command: &parent.lines[0].command,
        };
        cursor.update();
        cursor
    }
}

impl<'a> Cursor<'a> {

    fn reset(&mut self) {
        self.idx = 0;
        self.update();
    }

    fn update(&mut self) {
        let curr = self.state;
        self.prev = curr;
        self.curr_command = match self.parent.lines.get(self.idx) {
            Some(line) => &line.command,
            None => panic!("asdf"),
        };
        if let Command::G1 { x, y, z, e, f } = self.curr_command {
            self.state = [
                x.parse().unwrap_or(curr[0]),
                y.parse().unwrap_or(curr[1]),
                z.parse().unwrap_or(curr[2]),
                e.parse().unwrap_or(curr[3]),
                f.parse().unwrap_or(curr[4]),
            ];
        }
    }

    fn next(&mut self) -> Result<[f32; 5], &'static str> {
        // attempt to move the cursor to the next line
        // and return the line number if successful
        if self.parent.lines.len() - self.idx < 2 {
            return Err("End of file");
        }
        self.idx += 1;
        self.update();
        Ok(self.state)
    }

    fn prev(&mut self) -> Result<&'a Command, &'static str> {
        // attempt to move the cursor to the previous line
        // and return the line number if successful
        if self.idx == 0 {
            return Err("Start of file");
        }
        self.idx -= 1;
        self.update();
        Ok(self.curr_command)
    }

    fn child_at(&self, idx: usize) -> Cursor<'a> {
        let mut child = Cursor::from(self.parent);
        while child.idx < idx {
            let _ = child.next();
        }
        child

    }

    fn next_shape(&mut self) -> Result<RangeInclusive<usize>, &'static str> {
        // keep moving the cursor until a non exstrusion G1 is found if
        // starting from an extrusion, or until an extrusion is found if
        // starting from a non extrusion
        let start = self.idx;
        let mut end = self.idx;
        let init_ex = is_extrusion(self.state, self.prev);
        let mut prev = self.state;
        while let Ok(state) = self.next() {
            if init_ex != is_extrusion(state, prev) {
                break;
            }
            end = self.idx;
            prev = state;
        }
        Ok(start..=end)
    }

    fn at_first_extrusion(&self) -> bool{
        let mut temp_cursor = *self;
        let curr = temp_cursor.state;
        while temp_cursor.prev().is_ok() {
            if is_extrusion(curr, temp_cursor.state) {
                return false;
            }
        }
        true

    }

    fn is_purge_line(&mut self, lines: RangeInclusive<usize>) -> bool {
        // determining what is a purge line based on 
        //     1) is it the first extrusion of the print
        //     2) is it outside of the print area
        //     3) can you fit the shape to a line
        let  start = lines.start();
        let mut cur = self.child_at(*start);
        if !cur.at_first_extrusion() {
            return false;
        }
        let mut init = cur.state;
        let mut shape_positions = Vec::new();
        // load all the shape positions into a vec while
        // checking if any extrusions are inside the main print area
        while cur.next().is_ok() {
            if is_extrusion(init, cur.state) {
                shape_positions.push(cur.state);
                if cur.state[0] > 2.0 && cur.state[1] > 2.0 {
                    return false;
                }
            }
            init = cur.state;
        }
        // now if there are 3 or more points in the shape,
        // check if they are in a line by making sure the
        // slope (abs) is the same for every move

        if shape_positions.len() > 2 {
            let mut slope = calc_slope(shape_positions[0], shape_positions[1]).abs();
            for i in 2..shape_positions.len() {
                let slope_i = calc_slope(shape_positions[i - 1], shape_positions[i]).abs();
                if (slope - slope_i).abs() > f32::EPSILON {
                    return false;
                }
                slope = slope_i;
            } 
        }

        true

    }

    fn shapes(&mut self) -> Result<Vec<RangeInclusive<usize>>, &'static str> {
        let mut shapes = Vec::new();
        self.reset();
        while self.next().is_ok() {
            let range = self.next_shape()?;
            shapes.push(range);
        }
        Ok(shapes)
    }

    pub fn pre_print(&mut self) -> Result<RangeInclusive<usize>, &'static str> {
        let mut shapes = self.shapes()?;
        shapes.reverse();
        if let Some(first) = shapes.pop() {
            if !self.is_purge_line(first.clone()) {
                return Ok(RangeInclusive::new(0, first.start() - 1));
            }
            else if let Some(second) = shapes.pop() {
                return Ok(RangeInclusive::new(0, second.start() - 1));
            }
        }
        Err("No preprint found")
    }

    pub fn post_print(&mut self) -> Result<RangeInclusive<usize>, &'static str> {
        let mut shapes = self.shapes()?;
        if let Some(range_inclusive) = shapes.pop() {
            return Ok(RangeInclusive::new(range_inclusive.end() + 1, self.parent.lines.len() - 1));
        }
        Err("No postprint found")
    }
    fn nonplanar_extrusion(&self, prev: [f32; 5]) -> bool {
        let [_dx, _dy, dz, _de, _df] = self
            .state
            .iter()
            .zip(prev.iter())
            .map(|(a, b)| a - b)
            .collect::<Vec<f32>>()
            .try_into()
            .unwrap();
        if let Command::G1 { e, .. } = self.curr_command {
            if let Ok(e) = e.parse::<f32>() {
                return e > 0.0 && dz.abs() > f32::EPSILON;
            }
        }
        false
    }
    pub fn is_planar(&mut self) -> bool {
        let mut init = self.state;
        while self.next().is_ok() {
            if self.nonplanar_extrusion(init) {
                return false;
            }
            init = self.state;
        }
        true
    }

    pub fn layer_height(&mut self) -> (u32, u32) {
        let mut init = self.state;
        let mut heights = Vec::new();
        if !self.is_planar() {
            return (0, 0);
        }
        while self.next().is_ok() {
            if is_extrusion(self.state,init) {
                heights.push(self.state[2]);
            }
            init = self.state;
        }
        heights.dedup();
        let mut heights = heights.iter().map(|x| (x * 1000.0) as u32).collect::<Vec<u32>>();
        heights.sort();
        if heights.is_empty() {
            return (0, 0);
        }
        if heights.len() == 1 {
            return (heights[0], 0);
        }
        let first = heights[0];
        let second = heights[1];
        let first_layer_height = second - first;
        if heights.len() == 2 {
            return (first_layer_height, 0);
        }
        let second_layer_height = heights[2] - second;
        if heights.len() == 3 {
            return (first_layer_height, second_layer_height);
        }
        for i in 3..heights.len() {
            let layer_height = heights[i] - heights[i - 1];
            if layer_height != second_layer_height {
                return (first_layer_height, 0);
            }
        }
        (first_layer_height, second_layer_height)
    }
}

#[cfg(test)]

#[test]
fn slope_test(){
    let a = [0.0, 0.0, 0.0, 0.0, 0.0];
    let b = [1.0, 1.0, 0.0, 0.0, 0.0];
    assert_eq!(calc_slope(a, b), 1.0);
    let a = [0.0, 0.0, 0.0, 0.0, 0.0];
    let b = [1.0, 0.0, 0.0, 0.0, 0.0];
    assert_eq!(calc_slope(a, b), 0.0);
    let a = [0.0, 0.0, 0.0, 0.0, 0.0];
    let b = [10.0, 1.0, 0.0, 0.0, 0.0];
    assert_eq!(calc_slope(a, b), 0.10);
}

#[test]
fn is_extrusion_test() {
    let tests = [
        ([0.0, 0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0, 0.0], false),
        ([0.0, 0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 0.0, 0.0], false),
        ([0.0, 1.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0, 0.0], true),
        ([0.0, 1.0, 0.0, 1.0, 0.0], [1.0, 1.0, 0.0, 1.0, 0.0], true),
        ([50.0, -1.0, 3.0, 25.0, 900.0], [0.0, -1.0, 3.0, 0.0, 900.0], true),
    ];
    for (curr, prev, expected) in tests.iter() {
        assert_eq!(is_extrusion(*curr, *prev), *expected);
    }
}

#[test]
fn planar_test() {
    use crate::tests;
    let model = GCodeModel::from_file(&tests::test_gcode_path().join("test.gcode")).unwrap();
    let mut cursor = Cursor::from(&model);
    assert!(cursor.is_planar());
}

#[test]
fn preprint_test() {
    let model = GCodeModel::from_file(&crate::tests::test_gcode_path().join("test.gcode")).unwrap();
    let mut cursor = Cursor::from(&model);
    let range = cursor.pre_print();
    assert_eq!(range, Ok(0..=100));
}

#[test]
fn test_cursor() {
let model = GCodeModel::from_file(&crate::tests::test_gcode_path().join("test.gcode")).unwrap();
    let mut cursor = Cursor::from(&model);
    assert!(cursor.is_planar());
    let (_first, second) = cursor.layer_height();
    //assert_eq!(first, 200);
    assert_eq!(second, 200);
}
#[test]
fn alt_shape_test() {
    let tests = [
        ("G1 X10 Y10 E10", Ok(0..=0)),
        ("G1 X10 Y10 E10\nG1 X20 Y20 E20", Ok(0..=1)),
        ("G1 X10 Y10 E10\nG1 X20 Y20 E20\nG1 X30 Y30 E30", Ok(0..=2)),
    ];
    for (line, expected) in tests.iter() {
        let model: GCodeModel = line.parse().unwrap();
        let mut cursor = Cursor::from(&model);
        let next = cursor.next_shape();
        assert_eq!(next, *expected);
    }
}
#[test]
fn shape_test() {
    let test_gcode = "
        G1 Z3 F900
        G1 X0 Y-1
        G1 X50 Y-1 E25
        G1 X25 E10
        G1 E-1.5
        G1 Z1
        G1 X50 Y50
        G1 Z0.2 E1
        G1 X50 Y100 E12.222 
    ";
    let model: GCodeModel = test_gcode.parse().unwrap();
    let mut cursor = Cursor::from(&model);
    let expected_results = [0..=1, 2..=3, 4..=7, 8..=8];
    for expected in expected_results.iter() {
        let next = cursor.next_shape();
        assert_eq!(next, Ok(expected.clone()));
    }
    let mut cursor = Cursor::from(&model);
    let shapes = cursor.shapes();
    assert_eq!(shapes, Ok(vec![0..=1, 2..=3, 4..=7, 8..=8]));
}