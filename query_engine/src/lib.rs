use std::rc::Rc;

use char_locations::CharLocations;
pub use location::Location;
use span::Span;

pub use array_index::ArrayIndex;
pub use json_err::JQErr;
pub use object_index::ObjectKeyIndex;
pub use raw::RawTokenStream;
pub use sanitized::Sanitized;
pub use scope::Scope;
pub use slurp::Slurp;
pub use to_string_compact::CompactChars;
pub use to_string_pretty::PrettyChars;
pub use token::Token;
pub use values::Values;

mod char_locations;
mod span;
mod location;

mod array_index;
mod fuse;
mod json_err;
mod object_index;
mod raw;
mod sanitized;
mod scope;
mod slurp;
mod stream_context;
mod to_string_compact;
mod to_string_pretty;
mod token;
mod values;

pub(crate) type Item = Result<Token, JQErr>;

pub struct Null {
    value: Option<crate::Item>,
}

impl Default for Null {
    fn default() -> Self {
        Self {
            value: Some(Ok(Token::Null)),
        }
    }
}

impl Iterator for Null {
    type Item = crate::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.value.take()
    }
}

impl SanitizedJQStream for Null {}

pub trait CharStream: Iterator<Item = char> {
    fn into_json_tokens(self) -> RawTokenStream<Self>
    where
        Self: Sized,
    {
        RawTokenStream::new(self)
    }
}

impl<T> CharStream for T where T: Iterator<Item = char> {}

/// A named trait for a JQ iterator. All JQ iterators have
/// an Item type of [`Result`] where the [`Ok`] variant is
/// of type [`Token`] and the [`Err`] variant is of type
/// [`JsonErr`]
pub trait JQStream: Iterator<Item = crate::Item> {
    /// Takes a JSON token stream and wraps it to guarantee
    /// that:
    /// 1. The JSON token stream conforms to the JSON grammar
    /// 2. The JSON token stream will fuse after returning the
    /// first error (this helps prevent getting into weird states
    /// and spining in infinite loops).
    fn sanitize(self) -> Sanitized<Self>
    where
        Self: Sized,
    {
        Sanitized::new(self)
    }
}

/// Similar to [`JQStream`], but implementors of this trait
/// must guarantee that:
/// 1. The JSON token stream conforms to the JSON grammar
/// 2. The JSON token stream will fuse after returning the
/// first error (if this guarantee is violated, other
/// derivative JQStreams may enter weird states and spin
/// in infinite loops).
///
/// Any JQStream can be transformed into a SanitizedJQStream
/// by calling .sanitize().
pub trait SanitizedJQStream: JQStream {
    /// This API is not recommended for use in written Rust code. It is reserved for
    /// jq proc-macro generations. Callers should prefer
    /// [`SanitizedJQStream::at_index`]. This API will adjust the float
    /// to an integer in strange ways before using it as an index to maintain
    /// compatibility with the C-based JQ library.
    ///
    /// Runs a `.[{index}]` operation
    #[deprecated]
    fn at_number_index(self, index: f64) -> ArrayIndex<true, Self>
    where
        Self: Sized,
    {
        ArrayIndex::new(
            self,
            if index < 0.0 {
                index.ceil()
            } else {
                index.floor()
            } as isize,
        )
    }

    /// This API is not recommended for use in written Rust code. It is reserved for
    /// jq proc-macro generations. Callers should prefer
    /// [`SanitizedJQStream::at_index_suppress_errs`]. This API will adjust the float
    /// to an integer in strange ways before using it as an index to maintain
    /// compatibility with the C-based JQ library.
    ///
    /// Runs a `.[{index}]?` operation
    #[deprecated]
    fn at_number_index_suppress_errs(self, index: f64) -> ArrayIndex<false, Self>
    where
        Self: Sized,
    {
        ArrayIndex::new(
            self,
            if index < 0.0 {
                index.ceil()
            } else {
                index.floor()
            } as isize,
        )
    }

    /// Runs a `.[{index}]` operation
    fn at_index(self, index: isize) -> ArrayIndex<true, Self>
    where
        Self: Sized,
    {
        ArrayIndex::new(self, index)
    }

    /// Runs a `.[{index}]?` operation
    fn at_index_suppress_errs(self, index: isize) -> ArrayIndex<false, Self>
    where
        Self: Sized,
    {
        ArrayIndex::new(self, index)
    }

    /// Runs a `.["{key}"]` operation
    fn at_key<Key>(self, key: Key) -> ObjectKeyIndex<true, Self>
    where
        Self: Sized,
        Key: Into<Rc<str>>,
    {
        ObjectKeyIndex::new(self, key.into())
    }

    /// Runs a `.["{key}"]?` operation
    fn at_key_suppress_errs<Key>(self, key: Key) -> ObjectKeyIndex<false, Self>
    where
        Self: Sized,
        Key: Into<Rc<str>>,
    {
        ObjectKeyIndex::new(self, key.into())
    }

    /// Runs a `slurp` operation
    fn slurp(self) -> Slurp<Self>
    where
        Self: Sized,
    {
        Slurp::new(self)
    }

    /// Converts the JSON token stream into a stream of
    /// compactly formatted characters to form the JSON.
    fn to_chars_compact(self) -> CompactChars<Self>
    where
        Self: Sized,
    {
        CompactChars::new(self)
    }

    /// Converts the JSON token stream into a stream of
    /// pretty formatted characters to form the JSON.
    fn to_chars_pretty(self) -> PrettyChars<Self>
    where
        Self: Sized,
    {
        PrettyChars::new(self)
    }

    /// Converts the JSON token stream into a string of
    /// compactly formatted characters to form the JSON.
    fn to_string(self) -> Result<String, JQErr>
    where
        Self: Sized,
    {
        self.to_chars_compact().collect::<Result<String, JQErr>>()
    }

    /// Converts the JSON token stream into a string of
    /// pretty formatted characters to form the JSON.
    fn to_string_pretty(self) -> Result<String, JQErr>
    where
        Self: Sized,
    {
        self.to_chars_pretty().collect::<Result<String, JQErr>>()
    }

    /// Runs a `.[]` operation.
    fn values(self) -> Values<true, Self>
    where
        Self: Sized,
    {
        Values::new(self)
    }

    /// Runs a `.[]?` operation.
    fn values_suppress_errs(self) -> Values<false, Self>
    where
        Self: Sized,
    {
        Values::new(self)
    }
}

impl<T> JQStream for T where T: Iterator<Item = crate::Item> {}
