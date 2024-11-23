use std::ops::RangeInclusive;

use crate::microns::Microns;
use crate::*;

fn calc_slope(a: [Microns; 5], b: [Microns; 5]) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    f32::from(dy) / f32::from(dx)
}
fn is_extrusion(curr: [Microns; 5], prev: [Microns; 5]) -> bool {
    if curr[3] > Microns::ZERO {
        return (curr[0] - prev[0]).abs() > Microns::ZERO
            || (curr[1] - prev[1]).abs() > Microns::ZERO
            || (curr[2] - prev[2]).abs() > Microns::ZERO;
    }
    false
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CursorError {
    StartOfFile,
    EndOfFile,
} 

impl std::fmt::Display for CursorError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CursorError::StartOfFile => write!(f, "Start of file"),
            CursorError::EndOfFile => write!(f, "End of file"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Cursor<'a> {
    parent: &'a GCodeModel,
    idx: usize,
    state: [Microns; 5],
    prev: [Microns; 5],
    curr_command: &'a Command,
}

impl<'a> From<&'a GCodeModel> for Cursor<'a> {
    fn from(parent: &'a GCodeModel) -> Self {
        let mut cursor = Cursor {
            parent,
            idx: 0,
            state: [Microns::MIN; 5],
            prev: [Microns::MIN; 5],
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
        let line = self.parent.lines.get(self.idx).unwrap();
        self.curr_command = &line.command;
        self.state = match self.curr_command {
            Command::G1 { x, y, z, e, f } => [
                x.unwrap_or(self.state[0]),
                y.unwrap_or(self.state[1]),
                z.unwrap_or(self.state[2]),
                e.unwrap_or(self.state[3]),
                f.unwrap_or(self.state[4]),
            ],
            Command::Home(_) => [Microns::ZERO; 5],
            _ => self.state,
        }
    }

    fn next(&mut self) -> Result<[Microns; 5], CursorError> {
        // attempt to move the cursor to the next line
        // and return the line number if successful
        if self.idx == self.parent.lines.len() - 1 {
            return Err(CursorError::EndOfFile);
        }
        let new_prev = self.state;
        self.idx += 1;
        self.prev = new_prev;
        self.update();
        Ok(self.state)
    }

    fn prev(&mut self) -> Result<&'a Command, CursorError> {
        // attempt to move the cursor to the previous line
        // and return the line number if successful
        if self.idx == 0 {
            return Err(CursorError::StartOfFile);
        }
        let new_prev = self.state;
        self.idx -= 1;
        self.prev = new_prev;
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

    fn next_shape(&mut self) -> RangeInclusive<usize> {
        // keep moving the cursor until a non exstrusion G1 is found if
        // starting from an extrusion, or until an extrusion is found if
        // starting from a non extrusion
        let start = self.idx;
        let mut end = self.idx;
        if self.idx == self.parent.lines.len() - 1 {
            return start..=end;
        }

        let init_state = is_extrusion(self.state, self.prev);
        let mut prev = self.state;
        while init_state != is_extrusion(self.state, prev) && self.next().is_ok() {
            end = self.idx;
            prev = self.state;
        }

        if self.idx != self.parent.lines.len() - 1 {
            let _ = self.prev();
        }

        start..=end
    }

    fn at_first_extrusion(&self) -> bool {
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
        let start = lines.start();
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
                if cur.state[0] > Microns::from(2.0) && cur.state[1] > Microns::from(2.0) {
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

    fn shapes(&mut self) -> Vec<RangeInclusive<usize>> {
        let mut shapes = Vec::new();
        self.reset();
        while self.idx < self.parent.lines.len() - 1 {
            shapes.push(self.next_shape());
        }
        shapes
    }

    pub fn pre_print(&mut self) -> Result<RangeInclusive<usize>, &'static str> {
        let mut shapes = self.shapes();
        shapes.reverse();
        if let Some(first) = shapes.pop() {
            if !self.is_purge_line(first.clone()) {
                return Ok(RangeInclusive::new(0, first.start().saturating_sub(1)));
            } else if let Some(second) = shapes.pop() {
                return Ok(RangeInclusive::new(0, second.start().saturating_sub(1)));
            }
        }
        Err("No preprint found")
    }

    pub fn post_print(&mut self) -> Result<RangeInclusive<usize>, &'static str> {
        let mut shapes = self.shapes();
        if let Some(range_inclusive) = shapes.pop() {
            return Ok(RangeInclusive::new(
                range_inclusive.end() + 1,
                self.parent.lines.len() - 1,
            ));
        }
        Err("No postprint found")
    }
    fn nonplanar_extrusion(&self, prev: [Microns; 5]) -> bool {
        let [_dx, _dy, dz, _de, _df] = self
            .state
            .iter()
            .zip(prev.iter())
            .map(|(a, b)| *a - *b)
            .collect::<Vec<Microns>>()
            .try_into()
            .unwrap();
        if let Command::G1 { e: Some(e), .. } = self.curr_command
        {
            return *e > Microns::ZERO && dz.abs() > Microns::ZERO;
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

    pub fn layer_height(&mut self) -> (Microns, Microns) {
        let mut init = self.state;
        let mut heights = Vec::new();
        if !self.is_planar() {
            return (Microns::ZERO, Microns::ZERO);
        }
        while self.next().is_ok() {
            if is_extrusion(self.state, init) {
                heights.push(self.state[2]);
            }
            init = self.state;
        }
        heights.dedup();
        heights.sort();
        if heights.is_empty() {
            return (Microns::ZERO, Microns::ZERO);
        }
        if heights.len() == 1 {
            return (heights[0], Microns::ZERO);
        }
        let first = heights[0];
        let second = heights[1];
        let first_layer_height = second - first;
        if heights.len() == 2 {
            return (first_layer_height, Microns::ZERO);
        }
        let second_layer_height = heights[2] - second;
        if heights.len() == 3 {
            return (first_layer_height, second_layer_height);
        }
        for i in 3..heights.len() {
            let layer_height = heights[i] - heights[i - 1];
            if layer_height != second_layer_height {
                return (first_layer_height, Microns::ZERO);
            }
        }
        (first_layer_height, second_layer_height)
    }
}

#[cfg(test)]
#[test]
fn slope_test() {
    fn hack(x: [f32; 5]) -> [Microns; 5] {
        x.iter()
            .map(|&x| Microns::from(x))
            .collect::<Vec<Microns>>()
            .try_into()
            .unwrap()
    }
    let a = hack([0.0, 0.0, 0.0, 0.0, 0.0]);
    let b = hack([1.0, 1.0, 0.0, 0.0, 0.0]);
    assert_eq!(calc_slope(a, b), 1.0);
    let a = hack([0.0, 0.0, 0.0, 0.0, 0.0]);
    let b = hack([1.0, 0.0, 0.0, 0.0, 0.0]);
    assert_eq!(calc_slope(a, b), 0.0);
    let a = hack([0.0, 0.0, 0.0, 0.0, 0.0]);
    let b = hack([10.0, 1.0, 0.0, 0.0, 0.0]);
    assert_eq!(calc_slope(a, b), 0.10);
}

#[test]
fn is_extrusion_test() {
    let tests = [
        ([0.0, 0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0, 0.0], false),
        ([0.0, 0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 0.0, 0.0], false),
        ([0.0, 1.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0, 0.0], true),
        ([0.0, 1.0, 0.0, 1.0, 0.0], [1.0, 1.0, 0.0, 1.0, 0.0], true),
        (
            [50.0, -1.0, 3.0, 25.0, 900.0],
            [0.0, -1.0, 3.0, 100.0, 900.0],
            true,
        ),
    ];
    let tests = tests
        .iter()
        .map(|(curr, prev, expected)| {
            (
                curr.iter()
                    .map(|&x| Microns::from(x))
                    .collect::<Vec<Microns>>()
                    .try_into()
                    .unwrap(),
                prev.iter()
                    .map(|&x| Microns::from(x))
                    .collect::<Vec<Microns>>()
                    .try_into()
                    .unwrap(),
                *expected,
            )
        })
        .collect::<Vec<([Microns; 5], [Microns; 5], bool)>>();
    for (curr, prev, expected) in tests.iter() {
        assert_eq!(is_extrusion(*curr, *prev), *expected);
    }
}

#[test]
fn planar_test() {
    use crate::tests;
    let model = GCodeModel::try_from(tests::test_gcode_path().join("test.gcode").as_path()).unwrap();
    let mut cursor = Cursor::from(&model);
    assert!(cursor.is_planar());
}

#[test]
fn preprint_test() {
    let model = GCodeModel::try_from(tests::test_gcode_path().join("test.gcode").as_path()).unwrap();
    let mut cursor = Cursor::from(&model);
    let range = cursor.pre_print();
    assert_eq!(range, Ok(0..=100));
}

#[test]
fn test_cursor() {
    let model = GCodeModel::try_from(tests::test_gcode_path().join("test.gcode").as_path()).unwrap();
    //let mut cursor = Cursor::from(&model);
    //assert_eq!(cursor.idx, 0);
    // for i in 0..100 {
    //     let _ = cursor.next();
    //     assert_eq!(cursor.idx, i + 1);
    // }

    // loop {
    //     if let Some(GCodeLine {
    //         command: Command::G1 { .. },
    //         ..
    //     }) = cursor.parent.lines.get(cursor.idx)
    //     {
    //         break;
    //     }
    //     let _ = cursor.next();
    // }
}
#[test]
fn alt_shape_test() {
    let tests = [
        ("G1 X10 Y10 E10", 0..=0),
        ("G1 X10 Y10 E10\nG1 X20 Y20 E20", 0..=1),
        ("G1 X10 Y10 E10\nG1 X20 Y20 E20\nG1 X30 Y30 E30", 0..=2),
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
        assert_eq!(next, expected.clone());
    }
    let shapes = cursor.shapes();
    assert_eq!(shapes, vec![0..=1, 2..=3, 4..=7, 8..=8]);
}
