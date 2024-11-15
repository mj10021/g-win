#![allow(dead_code)]
use std::ops::Range;

use crate::{Command, GCodeModel};

fn calc_slope(a: [f32; 5], b: [f32; 5]) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    dy / dx
}

#[derive(Clone, Copy)]
pub struct Cursor<'a> {
    parent: &'a GCodeModel,
    idx: usize,
    state: [f32; 5],
    prev: Option<[f32; 5]>,
    curr_command: &'a Command,
}

impl<'a> From<&'a GCodeModel> for Cursor<'a> {
    fn from(parent: &'a GCodeModel) -> Self {
        Cursor {
            parent,
            idx: 0,
            state: [0.0; 5],
            prev: None,
            curr_command: &parent.lines[0].command,
        }
    }
}

impl<'a> Cursor<'a> {

    fn reset(&mut self) {
        self.idx = 0;
        self.update();
    }

    fn update(&mut self) {
        let curr = self.state;
        self.prev = Some(curr);
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

     fn next(&mut self) -> Result<&'a Command, &'static str> {
        // attempt to move the cursor to the next line
        // and return the line number if successful
        if self.idx > self.parent.lines.len() - 2 {
            return Err("End of file");
        }
        self.idx += 1;
        self.update();
        Ok(self.curr_command)
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
        let mut child = Cursor {
            parent: self.parent,
            idx: 0,
            state: [0.0; 5],
            prev: None,
            curr_command: &self.parent.lines[0].command,
        };
        while child.idx < idx {
            let _ = child.next();
        }
        child

    }
 
    fn is_extrusion(&self, prev: [f32; 5]) -> bool {
        let [dx, dy, dz, _de, _df] = self
            .state
            .iter()
            .zip(prev.iter())
            .map(|(a, b)| a - b)
            .collect::<Vec<f32>>()
            .try_into()
            .unwrap();
        if let Command::G1 { e, .. } = self.curr_command {
            if let Ok(e) = e.parse::<f32>() {
                return e > 0.0 && (dx.abs() > f32::EPSILON || dy.abs() > f32::EPSILON || dz.abs() > f32::EPSILON);
            }
        }
        false
    }

    fn next_shape(&mut self, shape_or_change: bool) -> Range<usize> {
        // keep moving the cursor until a non exstrusion G1 is found
        // shape_or_change should be true for shape detevtion and false for change detection
        // FIXME: to automatically determine shape or change, we need to check the previous state
        // from the initial cursor position
        let bool_mod = !shape_or_change;
        let mut init = self.state;
        let start = self.idx;
        let mut end = self.idx;
        while self.next().is_ok() {
            if bool_mod == self.is_extrusion(init) {
                break;
            }
            init = self.state;
            end = self.idx;
        }
        start..end
    }

    fn at_first_extrusion(&self) -> bool{
        let mut temp_cursor = *self;
        while temp_cursor.prev().is_ok() {
            if temp_cursor.is_extrusion(temp_cursor.state) {
                return false;
            }
        }
        true

    }

    fn is_purge_line(&mut self, lines: Range<usize>) -> bool {
        // determining what is a purge line based on 
        //     1) is it the first extrusion of the print
        //     2) is it outside of the print area
        //     3) can you fit the shape to a line
        let Range { start, .. } = lines;
        let mut cur = self.child_at(start);
        if !cur.at_first_extrusion() {
            return false;
        }
        let mut init = cur.state;
        let mut shape_positions = Vec::new();
        // load all the shape positions into a vec while
        // checking if any extrusions are inside the main print area
        while cur.next().is_ok() {
            if cur.is_extrusion(init) {
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

    fn shapes(&mut self) -> Vec<Range<usize>> {
        let mut shapes = Vec::new();
        self.reset();
        while self.next().is_ok() {
            let range = self.next_shape(true);
            shapes.push(range);
        }
        shapes
    }

    pub fn pre_print(&mut self) -> Range<usize> {
        let mut shapes = self.shapes();
        shapes.reverse();
        if let Some(first) = shapes.pop() {
            if !self.is_purge_line(first.clone()) {
                return 0..first.start;
            }
            else {
                if let Some(second) = shapes.pop() {
                    return 0..second.start;
                }
            }
        }
        0..0
    }

    pub fn post_print(&mut self) -> Range<usize> {
        let mut shapes = self.shapes();
        if let Some(Range {end,..}) = shapes.pop() {
            return end..self.parent.lines.len();
        }
        self.parent.lines.len()..self.parent.lines.len()
    }
    fn nonplanar_extrusion(&self, prev: [f32; 5]) -> bool {
        let [dx, dy, dz, _de, _df] = self
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
            if self.is_extrusion(init) {
                heights.push(self.state[2]);
            }
            init = self.state;
        }
        heights.dedup();
        let mut heights = heights.iter().map(|x| (x * 1000.0) as u32).collect::<Vec<u32>>();
        heights.sort();
        if heights.len() < 1 {
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