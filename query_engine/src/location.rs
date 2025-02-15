use std::fmt::Display;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Location {
    line: usize,
    col: usize,
}

impl Location {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn col(&self) -> usize {
        self.col
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let line: usize = self.line.into();
        let col: usize = self.col.into();
        write!(f, "line: {0}, col: {1}", line, col)
    }
}
