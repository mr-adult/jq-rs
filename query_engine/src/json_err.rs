use std::{error::Error, fmt::Display, rc::Rc};

use crate::Location;

#[derive(Debug, Clone)]
pub enum JQErr {
    /// Yielded when an End of File is encountered before parsing has
    /// concluded.
    UnexpectedEOF,
    /// (Internal to JQ) yielded when an invalid stream is encountered.
    /// Most callers can just panic on this error.
    InvalidStream,
    /// Yielded when a number contains an illegal leading '0'.
    IllegalLeading0(Location),
    /// Yielded when characters that do not comply to the JSON spec are
    /// encountered during parsing.
    UnexpectedCharacter(Location),
    /// Yielded when an unescaped control character is found in a JSON
    /// string.
    UnescapedEscapeCharacter(Location),
    /// Yielded if an illegal backslash escape sequence is encountered.
    InvalidEscapeSequence(Location),
    StreamOperationFailed(Rc<str>),
}

impl Error for JQErr {}
impl Display for JQErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JQErr::UnexpectedEOF => {
                write!(f, "Unexpected EOF.")
            }
            JQErr::InvalidStream => {
                write!(f, "Underlying stream was invalid.")
            }
            JQErr::IllegalLeading0(location) => {
                write!(f, "Found illegal leading 0 at {}.", location)
            }
            JQErr::UnexpectedCharacter(loc) => {
                write!(f, "Found unexpected character at {}.", loc)
            }
            JQErr::InvalidEscapeSequence(loc) => {
                write!(f, "Found invalid escape sequence at {}.", loc)
            }
            JQErr::UnescapedEscapeCharacter(loc) => {
                write!(
                    f,
                    "Found unescaped version of a character which is required to be escaped at {}.",
                    loc
                )
            }
            JQErr::StreamOperationFailed(msg) => {
                write!(f, "error: {msg}")
            }
        }
    }
}
