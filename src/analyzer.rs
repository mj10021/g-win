#![allow(dead_code)]
use std::ops::Range;

use crate::{Command, GCodeModel};

struct Cursor<'a> {
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
        self.curr_command = &self.parent.lines[self.idx].command;
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
        if self.idx >= self.parent.lines.len() {
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
    fn print_start(&mut self) -> usize {
        while let Ok(command) = self.next() {
            if let Command::G1 { e, .. } = command {
                if let Ok(e) = e.parse::<f32>() {
                    if e > 0.0 {
                        break;
                    }
                }
            }
        }
        self.idx
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
        while let Ok(_) = self.next() {
            if bool_mod == self.is_extrusion(init) {
                break;
            }
            init = self.state;
            end = self.idx;
        }
        start..end
    }
    fn is_purge_line(&mut self, lines: Range<usize>) -> bool {
        // determining what is a purge line based on 
        //     1) is it the first extrusion of the print
        //     2) is it outside of the print area
        //     3) can you fit the shape to a line
        let Range { start, .. } = lines;
        self.idx = start;
        self.update();
        let mut init = self.state;
        while self.idx > 0 {
            if let Ok(_) = self.prev() {
                if self.is_extrusion(init) {
                    return false;
                }
            }
            init = self.state;
        }
        self.idx = start;
        self.update();
        let mut init = self.state;
        let mut shape_positions = Vec::new();
        while let Ok(_) = self.next() {
            if self.is_extrusion(init) {
                shape_positions.push(self.state);
                if self.state[0] > 2.0 && self.state[1] > 2.0 {
                    return false;
                }
            }
            init = self.state;
        }
        if shape_positions.len() > 2 {
            let dx = shape_positions[1][0] - shape_positions[0][0];
            let dy = shape_positions[1][1] - shape_positions[0][1];
            let mut slope = (dy / dx).abs();
            for i in 2..shape_positions.len() {
                let dx = shape_positions[i][0] - shape_positions[i - 1][0];
                let dy = shape_positions[i][1] - shape_positions[i - 1][1];
                let slope_i = (dy / dx).abs();
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
        while let Ok(_) = self.next() {
            let range = self.next_shape(true);
            shapes.push(range);
        }
        shapes
    }
}