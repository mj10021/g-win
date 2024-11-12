use std::ops::Range;

use crate::{Command, GCodeLine, GCodeModel};

struct Cursor<'a> {
    parent: &'a GCodeModel,
    idx: usize,
    state: [f32; 5],
    curr_command: &'a Command,
}

impl<'a> From<&'a GCodeModel> for Cursor<'a> {
    fn from(parent: &'a GCodeModel) -> Self {
        Cursor {
            parent,
            idx: 0,
            state: [0.0; 5],
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
        /// attempt to move the cursor to the next line
        /// and return the line number if successful
        if self.idx >= self.parent.lines.len() {
            return Err("End of file");
        }
        self.idx += 1;
        self.update();
        Ok(self.curr_command)
    }
    fn prev(&mut self) -> Result<&'a Command, &'static str> {
        /// attempt to move the cursor to the previous line
        /// and return the line number if successful
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
    fn next_shape(&mut self) -> Range<usize> {
        // keep moving the cursor until a non exstrusion G1 is found
        let start = self.idx;
        let mut end = self.idx;
        while let Ok(command) = self.next() {
            if let Command::G1 { e, .. } = command {
                if let Ok(e) = e.parse::<f32>() {
                    if e < f32::EPSILON {
                        break;
                    }
                }
            }
            end = self.idx;
        }
        start..end
    }

    fn next_intershape(&mut self) -> Range<usize> {
        // keep moving the cursor until an exstrusion G1 is found
        let mut init = self.state;
        let start = self.idx;
        let mut end = self.idx;
        while let Ok(command) = self.next() {
            let [dx, dy, dz, de, df] = self
                .state
                .iter()
                .zip(init.iter())
                .map(|(a, b)| a - b)
                .collect::<Vec<f32>>()
                .try_into()
                .unwrap();
            init = self.state;
            if let Command::G1 { e, .. } = command {
                if let Ok(e) = e.parse::<f32>() {
                    if e > f32::EPSILON && (dx.abs() > f32::EPSILON || dy.abs() > f32::EPSILON || dz.abs() > f32::EPSILON) {
                        break;
                    }
                }
            }
            end = self.idx;
        }
        start..end
    }
    fn get_shapes(&mut self) -> Vec<Range<usize>> {
        self.reset();
        let mut shapes = Vec::new();
        while self.idx < self.parent.lines.len() {
            shapes.push(self.next_shape());
        }
        shapes
    }
}
