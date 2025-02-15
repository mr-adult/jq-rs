use std::collections::VecDeque;

use crate::{JQStream, JQErr, Sanitized, Token};

pub struct CompactChars<Stream>
where
    Stream: JQStream,
{
    stream: Sanitized<Stream>,
    buf: VecDeque<char>,
}

impl<Stream> CompactChars<Stream>
where
    Stream: JQStream,
{
    pub fn new(stream: Stream) -> Self {
        Self {
            stream: stream.sanitize(),
            buf: VecDeque::new(),
        }
    }
}

impl<Stream> Iterator for CompactChars<Stream>
where
    Stream: JQStream,
{
    type Item = Result<char, JQErr>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(front) = self.buf.pop_front() {
            return Some(Ok(front));
        }

        let result = match self.stream.next()? {
            Err(err) => Some(Err(err)),
            Ok(token_kind) => match token_kind {
                Token::ObjectStart => Some(Ok('{')),
                Token::ObjectEnd => Some(Ok('}')),
                Token::ArrayStart => Some(Ok('[')),
                Token::ArrayEnd => Some(Ok(']')),
                Token::Colon => Some(Ok(':')),
                Token::Comma => Some(Ok(',')),
                Token::Number(str) | Token::String(str) => {
                    for ch in str.chars() {
                        self.buf.push_back(ch);
                    }
                    self.buf.push_back('"');
                    Some(Ok('"'))
                }
                Token::ParsedNumber(value) => {
                    let str = value.to_string();
                    for ch in str.chars() {
                        self.buf.push_back(ch);
                    }
                    Some(Ok(self.buf.pop_front().expect(
                        "A float to generate a string with at least one character",
                    )))
                }
                Token::True => {
                    self.buf.push_back('r');
                    self.buf.push_back('u');
                    self.buf.push_back('e');
                    Some(Ok('t'))
                }
                Token::False => {
                    self.buf.push_back('a');
                    self.buf.push_back('l');
                    self.buf.push_back('s');
                    self.buf.push_back('e');
                    Some(Ok('f'))
                }
                Token::Null => {
                    self.buf.push_back('u');
                    self.buf.push_back('l');
                    self.buf.push_back('l');
                    Some(Ok('n'))
                }
            },
        };

        if self.stream.get_path().is_empty() {
            self.buf.push_back('\n');
        }

        result
    }
}
