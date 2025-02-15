use std::iter::{Fuse, Peekable};

use crate::Location;

pub struct CharLocations<Chars>
where
    Chars: IntoIterator<Item = char>,
{
    moved: bool,
    done: bool,
    source: Peekable<Fuse<Chars::IntoIter>>,
    line: usize,
    col: usize,
    previous_was_new_line: bool,
}

impl<Chars> CharLocations<Chars>
where
    Chars: IntoIterator<Item = char>,
{
    pub fn new(source: Chars) -> Self {
        Self {
            moved: false,
            done: false,
            source: source.into_iter().fuse().peekable(),
            line: 0,
            col: 0,
            previous_was_new_line: false,
        }
    }

    pub fn peek_location(&self) -> Location {
        if !self.moved {
            Location::new(self.line, self.col)
        } else if self.previous_was_new_line {
            Location::new(self.line + 1, 0)
        } else {
            Location::new(self.line, self.col + 1)
        }
    }
}

impl<Chars> Iterator for CharLocations<Chars>
where
    Chars: IntoIterator<Item = char>,
{
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let next = match self.source.next() {
            None => {
                self.done = true;
                return None;
            }
            Some(next) => next,
        };

        if self.moved {
            if self.previous_was_new_line {
                self.line += 1;
                self.col = 0;
            } else {
                self.col += 1;
            }
        } else {
            self.moved = true;
        }

        self.previous_was_new_line = next == '\n';
        Some(next)
    }
}
