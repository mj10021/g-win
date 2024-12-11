use std::ops::RangeInclusive;

use crate::*;
use state::Vec5;

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
        self.prev = [Microns::MIN; 5];
        let line = self.parent.lines.get(self.idx).unwrap();
        self.curr_command = &line.command;
        self.state = match self.curr_command {
            Command::G1 { x, y, z, e, f } => [
                x.unwrap_or(self.prev.x()),
                y.unwrap_or(self.prev.y()),
                z.unwrap_or(self.prev.z()),
                e.unwrap_or(self.prev.e()),
                f.unwrap_or(self.prev.f()),
            ],
            Command::Home(_) => [Microns::ZERO; 5],
            _ => [Microns::MIN; 5],
        }
    }

    fn update(&mut self) {
        let line = self.parent.lines.get(self.idx).unwrap();
        self.curr_command = &line.command;
        self.prev = self.state;
        self.state = match self.curr_command {
            Command::G1 { x, y, z, e, f } => [
                x.unwrap_or(self.state.x()),
                y.unwrap_or(self.state.y()),
                z.unwrap_or(self.state.z()),
                e.unwrap_or(self.state.e()),
                f.unwrap_or(self.state.f()),
            ],
            Command::Home(_) => [Microns::ZERO; 5],
            _ => self.state,
        }
    }
    fn peek_next(&self) -> Result<&'a Command, CursorError> {
        if self.idx == self.parent.lines.len() - 1 {
            return Err(CursorError::EndOfFile);
        }
        Ok(&self.parent.lines[self.idx + 1].command)
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

    fn peek_prev(&self) -> Result<&'a Command, CursorError> {
        if self.idx == 0 {
            return Err(CursorError::StartOfFile);
        }
        Ok(&self.parent.lines[self.idx - 1].command)
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
        let start = self.idx;

        let init_state = self.is_extrusion();
        while self.peek_next().is_ok() {
            self.next().unwrap();
            if init_state != self.is_extrusion() {
                self.prev().unwrap();
                break;
            }
        }
        start..=self.idx
    }

    fn is_extrusion(&self) -> bool {
        let (curr, prev) = (self.state, self.prev);
        if curr[3] > Microns::ZERO {
            return (curr.x() - prev.x()).abs() > Microns::ZERO
                || (curr.y() - prev.y()).abs() > Microns::ZERO
                || (curr.z() - prev.z()).abs() > Microns::ZERO;
        }
        false
    }
    fn at_first_extrusion(&self) -> bool {
        let mut temp_cursor = *self;
        while temp_cursor.prev().is_ok() {
            if !self.is_extrusion() {
                return false;
            }
        }
        true
    }

    fn shapes(&mut self) -> Vec<RangeInclusive<usize>> {
        self.reset();
        let mut shapes = vec![self.next_shape()];
        while self.peek_next().is_ok() {
            self.next().unwrap();
            shapes.push(self.next_shape());
        }
        shapes
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
        if let Command::G1 { e: Some(e), .. } = self.curr_command {
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
            if self.is_extrusion() {
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
