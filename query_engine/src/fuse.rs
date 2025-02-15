use std::iter::FusedIterator;

use crate::{JQErr, Token};

pub(crate) struct FuseOnErr<Stream>
where
    Stream: Iterator<Item = crate::Item>,
{
    done: bool,
    stream: Stream,
}

impl<Stream> FuseOnErr<Stream>
where
    Stream: Iterator<Item = crate::Item>,
{
    pub(crate) fn new(stream: Stream) -> Self {
        Self {
            done: false,
            stream,
        }
    }
}

impl<Stream> Iterator for FuseOnErr<Stream>
where
    Stream: Iterator<Item = Result<Token, JQErr>>,
{
    type Item = crate::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        match self.stream.next() {
            None => {
                self.done = true;
                return None;
            }
            Some(Err(err)) => {
                self.done = true;
                return Some(Err(err));
            }
            Some(Ok(token_kind)) => return Some(Ok(token_kind)),
        }
    }
}

impl<Stream> FusedIterator for FuseOnErr<Stream> where
    Stream: Iterator<Item = Result<Token, JQErr>>
{
}
