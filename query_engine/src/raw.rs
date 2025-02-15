use crate::{JQErr, SanitizedJQStream, Scope, Token};
use crate::{CharLocations, Location, Span};

pub enum JsonParsingState {
    /// The state of parsing a value.
    Value,
    /// The state of parsing the first value member of an array.
    FirstArrayValue,
    /// The state of parsing the first key value pair member of an object.
    FirstObjectKey,
    /// The state of parsing an object's key.
    ObjectKey,
    /// The state of parsing an object's colon.
    ObjectColon,
    /// The state after a value has been parsed - used for matcing commas.
    AfterValue,
    /// The finished state - an entire value has been consumed.
    Finished,
}

pub struct RawTokenStream<Chars>
where
    Chars: Iterator<Item = char>,
{
    scopes: Vec<Scope>,
    state: JsonParsingState,
    source: Tokenizer<Chars>,
    current_object_key_index: usize,
}

impl<Chars> RawTokenStream<Chars>
where
    Chars: Iterator<Item = char>,
{
    pub(crate) fn new(chars: Chars) -> Self {
        Self {
            scopes: Vec::new(),
            state: JsonParsingState::Value,
            source: Tokenizer::new(chars),
            current_object_key_index: 0,
        }
    }
}

impl<Chars> Iterator for RawTokenStream<Chars>
where
    Chars: Iterator<Item = char>,
{
    type Item = Result<Token, JQErr>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            JsonParsingState::Finished => None,
            JsonParsingState::Value | JsonParsingState::FirstArrayValue => match self.source.next()
            {
                None => {
                    self.state = JsonParsingState::Finished;
                    if self.scopes.is_empty() {
                        None
                    } else {
                        Some(Err(JQErr::UnexpectedEOF))
                    }
                }
                Some(Err(err)) => Some(Err(err)),
                Some(Ok(token)) => match token.kind {
                    Token::ObjectStart => {
                        self.current_object_key_index = 0;
                        self.state = JsonParsingState::FirstObjectKey;
                        Some(Ok(Token::ObjectStart))
                    }
                    Token::ArrayStart => {
                        self.scopes.push(Scope::Array(0));
                        self.state = JsonParsingState::FirstArrayValue;
                        Some(Ok(Token::ArrayStart))
                    }
                    Token::ArrayEnd => {
                        if matches!(self.state, JsonParsingState::FirstArrayValue) {
                            self.state = JsonParsingState::Value;
                            assert!(matches!(self.scopes.pop(), Some(Scope::Array(_))));
                            Some(Ok(Token::ArrayEnd))
                        } else {
                            Some(Err(JQErr::UnexpectedCharacter(token.span.start)))
                        }
                    }
                    Token::String(_)
                    | Token::Number(_)
                    | Token::ParsedNumber(_)
                    | Token::True
                    | Token::False
                    | Token::Null => {
                        self.state = JsonParsingState::AfterValue;
                        Some(Ok(token.kind))
                    }
                    Token::ObjectEnd | Token::Colon | Token::Comma => {
                        Some(Err(JQErr::UnexpectedCharacter(token.span.start)))
                    }
                },
            },
            JsonParsingState::FirstObjectKey | JsonParsingState::ObjectKey => {
                match self.source.next() {
                    None => Some(Err(JQErr::UnexpectedEOF)),
                    Some(Err(err)) => Some(Err(err)),
                    Some(Ok(token)) => match token.kind {
                        Token::ObjectEnd => {
                            if matches!(self.state, JsonParsingState::FirstObjectKey) {
                                self.current_object_key_index = 0;
                                self.state = match self.scopes.last() {
                                    None => JsonParsingState::Value,
                                    Some(_) => JsonParsingState::AfterValue,
                                };

                                Some(Ok(Token::ObjectEnd))
                            } else {
                                Some(Err(JQErr::UnexpectedCharacter(token.span.start)))
                            }
                        }
                        Token::String(key) => {
                            if matches!(self.state, JsonParsingState::FirstObjectKey) {
                                self.scopes.push(Scope::ObjectAtKey {
                                    index: 0,
                                    key: key.clone(),
                                });
                            } else {
                                self.scopes.push(Scope::ObjectAtKey {
                                    index: self.current_object_key_index,
                                    key: key.clone(),
                                });
                            }

                            self.state = JsonParsingState::ObjectColon;
                            Some(Ok(Token::String(key)))
                        }
                        Token::ObjectStart
                        | Token::ArrayStart
                        | Token::ArrayEnd
                        | Token::Colon
                        | Token::Comma
                        | Token::Number(_)
                        | Token::ParsedNumber(_)
                        | Token::True
                        | Token::False
                        | Token::Null => Some(Err(JQErr::UnexpectedCharacter(token.span.start))),
                    },
                }
            }
            JsonParsingState::ObjectColon => match self.source.next() {
                None => Some(Err(JQErr::UnexpectedEOF)),
                Some(Err(err)) => Some(Err(err)),
                Some(Ok(token)) => match token.kind {
                    Token::Colon => {
                        self.state = JsonParsingState::Value;
                        Some(Ok(Token::Colon))
                    }
                    Token::ObjectStart
                    | Token::ObjectEnd
                    | Token::ArrayStart
                    | Token::ArrayEnd
                    | Token::Comma
                    | Token::String(_)
                    | Token::Number(_)
                    | Token::ParsedNumber(_)
                    | Token::True
                    | Token::False
                    | Token::Null => Some(Err(JQErr::UnexpectedCharacter(token.span.start))),
                },
            },
            JsonParsingState::AfterValue => match self.source.next() {
                None => {
                    if self.scopes.is_empty() {
                        None
                    } else {
                        Some(Err(JQErr::UnexpectedEOF))
                    }
                }
                Some(Err(err)) => Some(Err(err)),
                Some(Ok(token)) => match token.kind {
                    Token::Comma => match self.scopes.pop() {
                        None => Some(Err(JQErr::UnexpectedCharacter(token.span.start))),
                        Some(Scope::Array(index)) => {
                            self.scopes.push(Scope::Array(index + 1));
                            self.state = JsonParsingState::Value;
                            Some(Ok(Token::Comma))
                        }
                        Some(Scope::ObjectAtKey { index, .. }) => {
                            self.current_object_key_index = index + 1;
                            self.state = JsonParsingState::ObjectKey;
                            Some(Ok(Token::Comma))
                        }
                        Some(Scope::Object) => {
                            unreachable!("RawJsonTokenStream doesn't use Scope::Object")
                        }
                    },
                    Token::ObjectEnd => {
                        if matches!(
                            self.scopes.pop(),
                            Some(Scope::Object | Scope::ObjectAtKey { .. })
                        ) {
                            self.state = match self.scopes.last() {
                                None => JsonParsingState::Value,
                                _ => JsonParsingState::AfterValue,
                            };

                            Some(Ok(Token::ObjectEnd))
                        } else {
                            Some(Err(JQErr::UnexpectedCharacter(token.span.start)))
                        }
                    }
                    Token::ArrayEnd => {
                        if matches!(self.scopes.pop(), Some(Scope::Array(_))) {
                            self.state = match self.scopes.last() {
                                None => JsonParsingState::Value,
                                _ => JsonParsingState::AfterValue,
                            };

                            Some(Ok(Token::ArrayEnd))
                        } else {
                            Some(Err(JQErr::UnexpectedCharacter(token.span.start)))
                        }
                    }
                    Token::ObjectStart => {
                        self.current_object_key_index = 0;
                        self.state = JsonParsingState::FirstObjectKey;
                        Some(Ok(Token::ObjectStart))
                    }
                    Token::ArrayStart => {
                        self.scopes.push(Scope::Array(0));
                        self.state = JsonParsingState::FirstArrayValue;
                        Some(Ok(Token::ArrayStart))
                    }
                    Token::String(_)
                    | Token::Number(_)
                    | Token::ParsedNumber(_)
                    | Token::True
                    | Token::False
                    | Token::Null => {
                        if !self.scopes.is_empty() {
                            Some(Err(JQErr::UnexpectedCharacter(token.span.start)))
                        } else {
                            self.state = JsonParsingState::AfterValue;
                            Some(Ok(token.kind))
                        }
                    }
                    Token::Colon => Some(Err(JQErr::UnexpectedCharacter(token.span.start))),
                },
            },
        }
    }
}

impl<Chars> SanitizedJQStream for RawTokenStream<Chars> where Chars: Iterator<Item = char> {}

pub(crate) struct Tokenizer<Chars>
where
    Chars: IntoIterator<Item = char>,
{
    peeked: Option<char>,
    chars: CharLocations<Chars>,
}

impl<Chars> Tokenizer<Chars>
where
    Chars: IntoIterator<Item = char>,
{
    pub(crate) fn new(source: Chars) -> Self {
        Self {
            peeked: None,
            chars: CharLocations::new(source),
        }
    }

    fn next_char(&mut self) -> Option<char> {
        if let Some(peeked) = self.peeked.take() {
            Some(peeked)
        } else {
            self.chars.next()
        }
    }

    fn peek_location(&mut self) -> Location {
        let loc = self.chars.peek_location();

        if let Some(peeked) = &self.peeked {
            Location::new(
                loc.line(),
                loc.col() - char::len_utf8(*peeked).min(loc.col()),
            )
        } else {
            loc
        }
    }
}

impl<Chars> Iterator for Tokenizer<Chars>
where
    Chars: IntoIterator<Item = char>,
{
    type Item = Result<TokenWithSpan, JQErr>;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.peek_location();

        loop {
            match self.next_char() {
                None => return None,
                Some(ch) => {
                    match ch {
                        '{' => {
                            return Some(Ok(TokenWithSpan {
                                span: Span {
                                    start,
                                    end: self.peek_location(),
                                },
                                kind: Token::ObjectStart,
                            }))
                        }
                        '}' => {
                            return Some(Ok(TokenWithSpan {
                                span: Span {
                                    start,
                                    end: self.peek_location(),
                                },
                                kind: Token::ObjectEnd,
                            }))
                        }
                        '[' => {
                            return Some(Ok(TokenWithSpan {
                                span: Span {
                                    start,
                                    end: self.peek_location(),
                                },
                                kind: Token::ArrayStart,
                            }))
                        }
                        ']' => {
                            return Some(Ok(TokenWithSpan {
                                span: Span {
                                    start,
                                    end: self.peek_location(),
                                },
                                kind: Token::ArrayEnd,
                            }))
                        }
                        ':' => {
                            return Some(Ok(TokenWithSpan {
                                span: Span {
                                    start,
                                    end: self.peek_location(),
                                },
                                kind: Token::Colon,
                            }))
                        }
                        ',' => {
                            return Some(Ok(TokenWithSpan {
                                span: Span {
                                    start,
                                    end: self.peek_location(),
                                },
                                kind: Token::Comma,
                            }))
                        }
                        ' ' | '\n' | '\r' | '\t' => continue,
                        't' => {
                            if let Some('r') = self.next_char() {
                                if let Some('u') = self.next_char() {
                                    if let Some('e') = self.next_char() {
                                        return Some(Ok(TokenWithSpan {
                                            span: Span {
                                                start,
                                                end: self.peek_location(),
                                            },
                                            kind: Token::True,
                                        }));
                                    }
                                }
                            }

                            let peeked = self.peek_location();
                            return Some(Err(JQErr::UnexpectedCharacter(Location::new(
                                peeked.line(),
                                peeked.col() - 1,
                            ))));
                        }
                        'f' => {
                            if let Some('a') = self.next_char() {
                                if let Some('l') = self.next_char() {
                                    if let Some('s') = self.next_char() {
                                        if let Some('e') = self.next_char() {
                                            return Some(Ok(TokenWithSpan {
                                                span: Span {
                                                    start,
                                                    end: self.peek_location(),
                                                },
                                                kind: Token::False,
                                            }));
                                        }
                                    }
                                }
                            }

                            let peeked = self.peek_location();
                            return Some(Err(JQErr::UnexpectedCharacter(Location::new(
                                peeked.line(),
                                peeked.col() - 1,
                            ))));
                        }
                        'n' => {
                            if let Some('u') = self.next_char() {
                                if let Some('l') = self.next_char() {
                                    if let Some('l') = self.next_char() {
                                        return Some(Ok(TokenWithSpan {
                                            span: Span {
                                                start,
                                                end: self.peek_location(),
                                            },
                                            kind: Token::Null,
                                        }));
                                    }
                                }
                            }

                            let peeked = self.peek_location();
                            return Some(Err(JQErr::UnexpectedCharacter(Location::new(
                                peeked.line(),
                                peeked.col() - 1,
                            ))));
                        }
                        '"' => {
                            let mut string = String::new();
                            loop {
                                match self.next_char() {
                                    None => {
                                        return Some(Err(JQErr::UnexpectedEOF));
                                    }
                                    Some(ch) => {
                                        match ch {
                                            '"' => {
                                                return Some(Ok(TokenWithSpan {
                                                    span: Span {
                                                        start,
                                                        end: self.peek_location(),
                                                    },
                                                    kind: Token::String(string.into()),
                                                }));
                                            }
                                            '\\' => {
                                                match self.next_char() {
                                                    None => {
                                                        return Some(Err(JQErr::UnexpectedEOF))
                                                    }
                                                    Some('u') => {
                                                        let mut unicode_code =
                                                            String::with_capacity(4);
                                                        for _ in 0..4 {
                                                            if let Some(hex_digit) =
                                                                self.next_char()
                                                            {
                                                                if hex_digit.is_ascii_hexdigit() {
                                                                    unicode_code.push(hex_digit);
                                                                    continue;
                                                                }
                                                            }

                                                            let peeked = self.peek_location();
                                                            return Some(Err(
                                                                JQErr::UnexpectedCharacter(
                                                                    Location::new(
                                                                        peeked.line(),
                                                                        peeked.col() - 1,
                                                                    ),
                                                                ),
                                                            ));
                                                        }

                                                        let code = unicode_code
                                                            .parse::<u32>()
                                                            .expect(
                                                            "Hex code to infallibly parse to u32",
                                                        );
                                                        string.push(char::from_u32(code).expect(
                                                            "Hex code to translate to char",
                                                        ));
                                                    }
                                                    Some('"') => string.push('"'),
                                                    Some('\\') => string.push('\\'),
                                                    Some('/') => string.push('/'),
                                                    Some('b') => string.push('\u{0008}'), // backspace
                                                    Some('f') => string.push('\u{000C}'), // form feed
                                                    Some('n') => string.push('\n'),
                                                    Some('r') => string.push('\r'),
                                                    Some('t') => string.push('\t'),
                                                    Some('\u{0000}'..='\u{001F}') => {
                                                        let peeked = self.peek_location();
                                                        return Some(Err(
                                                            JQErr::UnescapedEscapeCharacter(
                                                                Location::new(
                                                                    peeked.line(),
                                                                    peeked.col() - 1,
                                                                ),
                                                            ),
                                                        ));
                                                    }
                                                    _ => {
                                                        let peeked = self.peek_location();
                                                        return Some(Err(
                                                            JQErr::InvalidEscapeSequence(
                                                                Location::new(
                                                                    peeked.line(),
                                                                    peeked.col() - 1,
                                                                ),
                                                            ),
                                                        ));
                                                    }
                                                }
                                            }
                                            '\u{0000}'..='\u{001F}' | '/' => {
                                                let peeked = self.peek_location();
                                                return Some(Err(
                                                    JQErr::UnescapedEscapeCharacter(
                                                        Location::new(
                                                            peeked.line(),
                                                            peeked.col() - 1,
                                                        ),
                                                    ),
                                                ));
                                            }
                                            ch => {
                                                string.push(ch);
                                            } // just continue
                                        }
                                    }
                                }
                            }
                        }
                        '-' | '0'..='9' => {
                            let mut digit = String::new();
                            digit.push(ch);

                            let first_digit = if matches!(ch, '-') {
                                if let Some(digit) = self.next_char() {
                                    if matches!(digit, '0'..='9') {
                                        digit
                                    } else {
                                        let peeked = self.peek_location();
                                        return Some(Err(JQErr::UnexpectedCharacter(
                                            Location::new(peeked.line(), peeked.col() - 1),
                                        )));
                                    }
                                } else {
                                    return Some(Err(JQErr::UnexpectedEOF));
                                }
                            } else {
                                ch
                            };

                            let next = self.next_char();
                            match next {
                                None => {
                                    return Some(Ok(TokenWithSpan {
                                        span: Span {
                                            start,
                                            end: self.peek_location(),
                                        },
                                        kind: Token::Number(digit.into()),
                                    }))
                                }
                                Some('0'..='9') => {
                                    if matches!(first_digit, '0') {
                                        let peeked = self.peek_location();
                                        return Some(Err(JQErr::IllegalLeading0(Location::new(
                                            peeked.col(),
                                            peeked.col() - 1,
                                        ))));
                                    } else {
                                        digit.push(next.unwrap());
                                    }
                                }
                                Some('.') => digit.push('.'),
                                Some(other) => {
                                    self.peeked = Some(other);
                                    return Some(Ok(TokenWithSpan {
                                        span: Span {
                                            start,
                                            end: self.peek_location(),
                                        },
                                        kind: Token::Number(digit.into()),
                                    }));
                                }
                            }

                            if !matches!(digit.chars().last(), Some('.')) {
                                loop {
                                    let next = self.next_char();
                                    match next {
                                        None => {
                                            return Some(Ok(TokenWithSpan {
                                                span: Span {
                                                    start,
                                                    end: self.peek_location(),
                                                },
                                                kind: Token::Number(digit.into()),
                                            }))
                                        }
                                        Some('0'..='9') => {
                                            digit.push(next.unwrap());
                                        }
                                        Some('.') => {
                                            digit.push('.');
                                            break;
                                        }
                                        Some(other) => {
                                            self.peeked = Some(other);
                                            return Some(Ok(TokenWithSpan {
                                                span: Span {
                                                    start,
                                                    end: self.peek_location(),
                                                },
                                                kind: Token::Number(digit.into()),
                                            }));
                                        }
                                    }
                                }
                            }

                            // If we matched a '.', at least one digit is required.
                            let mut matched_one_digit = !matches!(digit.chars().last(), Some('.'));
                            loop {
                                let next = self.next_char();
                                match next {
                                    None => {
                                        if !matched_one_digit {
                                            let peeked = self.peek_location();
                                            return Some(Err(JQErr::UnexpectedCharacter(
                                                Location::new(peeked.line(), peeked.col() - 1),
                                            )));
                                        }

                                        return Some(Ok(TokenWithSpan {
                                            span: Span {
                                                start,
                                                end: self.peek_location(),
                                            },
                                            kind: Token::Number(digit.into()),
                                        }));
                                    }
                                    Some(next) => match next {
                                        '0'..='9' => {
                                            matched_one_digit = true;
                                            digit.push(next);
                                        }
                                        'e' | 'E' => {
                                            if !matched_one_digit {
                                                let peeked = self.peek_location();
                                                return Some(Err(JQErr::UnexpectedCharacter(
                                                    Location::new(peeked.line(), peeked.col() - 1),
                                                )));
                                            }

                                            digit.push(next);
                                            break;
                                        }
                                        other => {
                                            self.peeked = Some(other);

                                            if !matched_one_digit {
                                                let peeked = self.peek_location();
                                                return Some(Err(JQErr::UnexpectedCharacter(
                                                    Location::new(peeked.line(), peeked.col() - 1),
                                                )));
                                            }

                                            return Some(Ok(TokenWithSpan {
                                                span: Span {
                                                    start,
                                                    end: self.peek_location(),
                                                },
                                                kind: Token::Number(digit.into()),
                                            }));
                                        }
                                    },
                                }
                            }

                            // If we matched an 'e' or 'E', at least one digit is required.
                            let mut matched_one_digit =
                                !matches!(digit.chars().last(), Some('e' | 'E'));
                            loop {
                                let next = self.next_char();
                                match next {
                                    None => {
                                        if !matched_one_digit {
                                            let peeked = self.peek_location();
                                            return Some(Err(JQErr::UnexpectedCharacter(
                                                Location::new(peeked.line(), peeked.col() - 1),
                                            )));
                                        }

                                        return Some(Ok(TokenWithSpan {
                                            span: Span {
                                                start,
                                                end: self.peek_location(),
                                            },
                                            kind: Token::Number(digit.into()),
                                        }));
                                    }
                                    Some(next) => match next {
                                        '0'..='9' => {
                                            matched_one_digit = true;
                                            digit.push(next);
                                        }
                                        other => {
                                            if !matched_one_digit {
                                                let peeked = self.peek_location();
                                                return Some(Err(JQErr::UnexpectedCharacter(
                                                    Location::new(peeked.line(), peeked.col() - 1),
                                                )));
                                            }

                                            self.peeked = Some(other);
                                            return Some(Ok(TokenWithSpan {
                                                span: Span {
                                                    start,
                                                    end: self.peek_location(),
                                                },
                                                kind: Token::Number(digit.into()),
                                            }));
                                        }
                                    },
                                }
                            }
                        }
                        other => {
                            let peeked = self.peek_location();
                            return Some(Err(JQErr::UnexpectedCharacter(Location::new(
                                peeked.line(),
                                peeked.col() - char::len_utf8(other).min(peeked.col()),
                            ))));
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TokenWithSpan {
    pub(crate) span: Span,
    pub(crate) kind: Token,
}
