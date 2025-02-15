use std::collections::VecDeque;

use crate::{JQStream, JQErr, Sanitized, Token};

pub struct PrettyChars<Stream>
where
    Stream: JQStream,
{
    stream: Sanitized<Stream>,
    buf: VecDeque<char>,
    indent_level: usize,
    previous: Option<Token>,
}

impl<Stream> PrettyChars<Stream>
where
    Stream: JQStream,
{
    const fn indent_str() -> &'static str {
        "  "
    }

    const fn new_line() -> &'static str {
        "\n"
    }

    pub fn new(stream: Stream) -> Self {
        Self {
            stream: stream.sanitize(),
            buf: VecDeque::new(),
            indent_level: 0,
            previous: None,
        }
    }

    fn add_new_line(&mut self) {
        {
            self.buf.extend(Self::new_line().chars());

            for _ in 0..self.indent_level {
                self.buf.extend(Self::indent_str().chars());
            }
        }
    }
}

impl<Stream> Iterator for PrettyChars<Stream>
where
    Stream: JQStream,
{
    type Item = Result<char, JQErr>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(front) = self.buf.pop_front() {
            return Some(Ok(front));
        }

        match self.stream.next()? {
            Err(err) => return Some(Err(err)),
            Ok(token_kind) => {
                if self.stream.get_path().is_empty()
                    && self.previous.is_some()
                    && !matches!(token_kind, Token::ArrayEnd | Token::ObjectEnd)
                {
                    self.add_new_line();
                }

                match token_kind.clone() {
                    Token::ObjectStart => {
                        if let Some(Token::ObjectStart | Token::ArrayStart) = self.previous {
                            self.add_new_line();
                        }
                        self.indent_level += 1;
                        self.buf.push_back('{');
                    }
                    Token::ObjectEnd => {
                        self.indent_level -= 1;
                        if let Some(Token::ObjectStart) = self.previous {
                            self.buf.push_back('}');
                        } else {
                            self.add_new_line();
                            self.buf.push_back('}');
                        }
                    }
                    Token::ArrayStart => {
                        if let Some(Token::ObjectStart | Token::ArrayStart) = self.previous {
                            self.add_new_line();
                        }

                        self.indent_level += 1;
                        self.buf.push_back('[');
                    }
                    Token::ArrayEnd => {
                        self.indent_level -= 1;
                        if let Some(Token::ArrayStart) = self.previous {
                            return Some(Ok(']'));
                        } else {
                            self.add_new_line();
                            self.buf.push_back(']');
                        }
                    }
                    Token::Colon => {
                        self.buf.push_back(':');
                        self.buf.push_back(' ');
                    }
                    Token::Comma => {
                        self.buf.push_back(',');
                        self.add_new_line();
                    }
                    Token::String(str) | Token::Number(str) => {
                        if let Some(Token::ObjectStart | Token::ArrayStart) = self.previous {
                            self.add_new_line();
                        }

                        self.buf.extend(str.chars());
                    }
                    Token::ParsedNumber(value) => {
                        if let Some(Token::ObjectStart | Token::ArrayStart) = self.previous {
                            self.add_new_line();
                        }

                        let string = value.to_string();
                        self.buf.extend(string.chars());
                    }
                    Token::True => {
                        if let Some(Token::ObjectStart | Token::ArrayStart) = self.previous {
                            self.add_new_line();
                        }
                        self.buf.extend("true".chars());
                    }
                    Token::False => {
                        if let Some(Token::ObjectStart | Token::ArrayStart) = self.previous {
                            self.add_new_line();
                        }
                        self.buf.extend("false".chars());
                    }
                    Token::Null => {
                        if let Some(Token::ObjectStart | Token::ArrayStart) = self.previous {
                            self.add_new_line();
                        }
                        self.buf.extend("null".chars());
                    }
                }

                self.previous = Some(token_kind.clone());
                return Some(Ok(self
                    .buf
                    .pop_front()
                    .expect("buf to have a character in it")));
            }
        }
    }
}
