use crate::raw::JsonParsingState;
use crate::{JQErr, Scope, Token};

use crate::JQStream;

/// A struct which handles tracking the context of the current token's path
/// as well as validation of the underlying stream to ensure it produces valid JSON.
pub(crate) struct StreamContext<Stream>
where
    Stream: JQStream,
{
    scopes: Vec<Scope>,
    state: JsonParsingState,
    stream: Stream,
    index_in_current_object: usize,
}

impl<Stream> StreamContext<Stream>
where
    Stream: JQStream,
{
    pub(crate) fn new(stream: Stream) -> Self {
        Self {
            scopes: Vec::new(),
            state: JsonParsingState::Value,
            stream,
            index_in_current_object: 0,
        }
    }

    /// Gets the current path in the JSON of this StreamValidator.
    pub(crate) fn get_path(&self) -> &[Scope] {
        &self.scopes
    }
}

impl<Stream> Iterator for StreamContext<Stream>
where
    Stream: JQStream,
{
    type Item = crate::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            JsonParsingState::Finished => return None,
            JsonParsingState::Value | JsonParsingState::FirstArrayValue => match self.stream.next()
            {
                None => {
                    self.state = JsonParsingState::Finished;
                    if self.scopes.is_empty() {
                        return None;
                    } else {
                        return Some(Err(JQErr::UnexpectedEOF));
                    }
                }
                Some(Err(err)) => return Some(Err(err)),
                Some(Ok(token_kind)) => match token_kind {
                    Token::ObjectStart => {
                        self.scopes.push(Scope::Object);
                        self.state = JsonParsingState::FirstObjectKey;
                        return Some(Ok(Token::ObjectStart));
                    }
                    Token::ArrayStart => {
                        self.scopes.push(Scope::Array(0));
                        self.state = JsonParsingState::FirstArrayValue;
                        return Some(Ok(Token::ArrayStart));
                    }
                    Token::ArrayEnd => {
                        if matches!(self.state, JsonParsingState::FirstArrayValue) {
                            self.state = JsonParsingState::Value;
                            assert!(matches!(self.scopes.pop(), Some(Scope::Array(_))));
                            return Some(Ok(Token::ArrayEnd));
                        } else {
                            return Some(Err(JQErr::InvalidStream));
                        }
                    }
                    Token::String(_)
                    | Token::Number(_)
                    | Token::ParsedNumber(_)
                    | Token::True
                    | Token::False
                    | Token::Null => {
                        self.state = JsonParsingState::AfterValue;
                        return Some(Ok(token_kind));
                    }
                    Token::ObjectEnd | Token::Colon | Token::Comma => {
                        return Some(Err(JQErr::InvalidStream))
                    }
                },
            },
            JsonParsingState::FirstObjectKey | JsonParsingState::ObjectKey => {
                match self.stream.next() {
                    None => {
                        self.state = JsonParsingState::Finished;
                        if self.scopes.is_empty() {
                            return None;
                        } else {
                            return Some(Err(JQErr::UnexpectedEOF));
                        }
                    }
                    Some(Err(err)) => return Some(Err(err)),
                    Some(Ok(token_kind)) => match token_kind {
                        Token::ObjectEnd => {
                            if matches!(self.state, JsonParsingState::FirstObjectKey) {
                                assert!(matches!(
                                    self.scopes.pop(),
                                    Some(Scope::Object | Scope::ObjectAtKey { .. })
                                ));
                                self.state = match self.scopes.last() {
                                    None => JsonParsingState::Value,
                                    Some(_) => JsonParsingState::AfterValue,
                                };

                                return Some(Ok(Token::ObjectEnd));
                            } else {
                                return Some(Err(JQErr::InvalidStream));
                            }
                        }
                        Token::String(key) => {
                            assert!(matches!(self.scopes.pop(), Some(Scope::Object)));
                            self.scopes.push(Scope::ObjectAtKey {
                                index: if matches!(self.state, JsonParsingState::FirstObjectKey) {
                                    0
                                } else {
                                    self.index_in_current_object
                                },
                                key: key.clone(),
                            });

                            self.state = JsonParsingState::ObjectColon;
                            return Some(Ok(Token::String(key)));
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
                        | Token::Null => return Some(Err(JQErr::InvalidStream)),
                    },
                }
            }
            JsonParsingState::ObjectColon => match self.stream.next() {
                None => {
                    self.state = JsonParsingState::Finished;
                    if self.scopes.is_empty() {
                        return None;
                    } else {
                        return Some(Err(JQErr::UnexpectedEOF));
                    }
                }
                Some(Err(err)) => return Some(Err(err)),
                Some(Ok(token_kind)) => match token_kind {
                    Token::Colon => {
                        self.state = JsonParsingState::Value;
                        return Some(Ok(Token::Colon));
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
                    | Token::Null => return Some(Err(JQErr::InvalidStream)),
                },
            },
            JsonParsingState::AfterValue => match self.stream.next() {
                None => {
                    if self.scopes.is_empty() {
                        return None;
                    } else {
                        return Some(Err(JQErr::UnexpectedEOF));
                    }
                }
                Some(Err(err)) => return Some(Err(err)),
                Some(Ok(token_kind)) => match token_kind {
                    Token::Comma => match self.scopes.pop() {
                        None => {
                            return Some(Err(JQErr::InvalidStream));
                        }
                        Some(Scope::Array(index)) => {
                            self.scopes.push(Scope::Array(index + 1));
                            self.state = JsonParsingState::Value;
                            return Some(Ok(Token::Comma));
                        }
                        Some(Scope::ObjectAtKey { index, .. }) => {
                            self.index_in_current_object = index;
                            self.scopes.push(Scope::Object);
                            self.state = JsonParsingState::ObjectKey;
                            return Some(Ok(Token::Comma));
                        }
                        Some(Scope::Object) => return Some(Err(JQErr::InvalidStream)),
                    },
                    Token::ObjectEnd => {
                        if matches!(
                            self.scopes.pop(),
                            Some(Scope::Object | Scope::ObjectAtKey { .. })
                        ) {
                            self.state = match self.scopes.last() {
                                None => JsonParsingState::Value,
                                Some(_) => JsonParsingState::AfterValue,
                            };

                            return Some(Ok(Token::ObjectEnd));
                        } else {
                            return Some(Err(JQErr::InvalidStream));
                        }
                    }
                    Token::ArrayEnd => {
                        if matches!(self.scopes.pop(), Some(Scope::Array(_))) {
                            self.state = match self.scopes.last() {
                                None => JsonParsingState::Value,
                                Some(_) => JsonParsingState::AfterValue,
                            };

                            return Some(Ok(Token::ArrayEnd));
                        } else {
                            return Some(Err(JQErr::InvalidStream));
                        }
                    }
                    Token::ObjectStart => {
                        self.scopes.push(Scope::Object);
                        self.state = JsonParsingState::FirstObjectKey;
                        return Some(Ok(Token::ObjectStart));
                    }
                    Token::ArrayStart => {
                        self.scopes.push(Scope::Array(0));
                        self.state = JsonParsingState::FirstArrayValue;
                        return Some(Ok(Token::ArrayStart));
                    }
                    Token::String(_)
                    | Token::Number(_)
                    | Token::ParsedNumber(_)
                    | Token::True
                    | Token::False
                    | Token::Null => {
                        self.state = JsonParsingState::AfterValue;
                        return Some(Ok(token_kind));
                    }
                    Token::Colon => return Some(Err(JQErr::InvalidStream)),
                },
            },
        }
    }
}
