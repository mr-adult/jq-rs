use std::rc::Rc;

use crate::{JQStream, JQErr, Sanitized, SanitizedJQStream, Scope, Token};

pub struct ObjectKeyIndex<const EMIT_ERRS: bool, Stream>
where
    Stream: JQStream,
{
    finished: bool,
    stream: Sanitized<Stream>,
    key: Rc<str>,
    matching: bool,
}

impl<const EMIT_ERRS: bool, Stream> ObjectKeyIndex<EMIT_ERRS, Stream>
where
    Stream: JQStream,
{
    pub(crate) fn new(stream: Stream, key: Rc<str>) -> Self {
        Self {
            finished: false,
            stream: stream.sanitize(),
            key,
            matching: false,
        }
    }
}

impl<const EMIT_ERRS: bool, Stream> Iterator for ObjectKeyIndex<EMIT_ERRS, Stream>
where
    Stream: JQStream,
{
    type Item = crate::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        loop {
            let path = self.stream.get_path();
            if path.len() > 1 {
                let next = self.stream.next();
                if self.matching {
                    match next {
                        None => {
                            self.finished = true;
                            return Some(Err(JQErr::InvalidStream));
                        }
                        Some(Err(err)) => {
                            self.finished = true;
                            return Some(Err(err));
                        }
                        Some(Ok(token)) => {
                            return Some(Ok(token));
                        }
                    }
                } else {
                    continue;
                }
            }

            match path.last().cloned() {
                None => match self.stream.next() {
                    None => return None,
                    Some(Err(err)) => {
                        self.finished = true;
                        return Some(Err(err));
                    }
                    Some(Ok(token_kind)) => match token_kind {
                        Token::ArrayStart => {
                            self.matching = false;
                            if EMIT_ERRS {
                                self.finished = true;
                                return Some(Err(JQErr::StreamOperationFailed(
                                    format!("Cannot index array with string \"{}\"", self.key)
                                        .into(),
                                )));
                            }
                        }
                        Token::ObjectStart => {
                            let key = match self.stream.next() {
                                None => {
                                    self.finished = true;
                                    return Some(Err(JQErr::InvalidStream));
                                }
                                Some(Err(err)) => {
                                    self.finished = true;
                                    return Some(Err(err));
                                }
                                Some(Ok(Token::String(key))) => key,
                                Some(Ok(Token::ObjectEnd)) => return Some(Ok(Token::Null)),
                                Some(Ok(_)) => {
                                    self.finished = true;
                                    return Some(Err(JQErr::InvalidStream));
                                }
                            };

                            match self.stream.next() {
                                None => {
                                    self.finished = true;
                                    return Some(Err(JQErr::InvalidStream));
                                }
                                Some(Err(err)) => {
                                    self.finished = true;
                                    return Some(Err(err));
                                }
                                Some(Ok(Token::Colon)) => {}
                                Some(Ok(_)) => {
                                    self.finished = true;
                                    return Some(Err(JQErr::InvalidStream));
                                }
                            }

                            match self.stream.next() {
                                None => {
                                    self.finished = true;
                                    return Some(Err(JQErr::UnexpectedEOF));
                                }
                                Some(Err(err)) => {
                                    self.finished = true;
                                    return Some(Err(err));
                                }
                                Some(Ok(token_kind)) => {
                                    if token_kind.is_value_start() {
                                        self.matching = *key == *self.key;
                                        if self.matching {
                                            return Some(Ok(token_kind));
                                        }
                                    } else {
                                        self.finished = true;
                                        return Some(Err(JQErr::InvalidStream));
                                    }
                                }
                            }
                        }
                        Token::True | Token::False => {
                            if EMIT_ERRS {
                                self.finished = true;
                                return Some(Err(JQErr::StreamOperationFailed(
                                    format!("Cannot index boolean with string \"{}\"", self.key)
                                        .into(),
                                )));
                            } else {
                                continue;
                            }
                        }
                        Token::Null => return Some(Ok(Token::Null)),
                        Token::String(_) => {
                            if EMIT_ERRS {
                                self.finished = true;
                                return Some(Err(JQErr::StreamOperationFailed(
                                    format!("Cannot index string with string \"{}\"", self.key)
                                        .into(),
                                )));
                            } else {
                                continue;
                            }
                        }
                        Token::Number(_) | Token::ParsedNumber(_) => {
                            if EMIT_ERRS {
                                self.finished = true;
                                return Some(Err(JQErr::StreamOperationFailed(
                                    format!("Cannot index number with string \"{}\"", self.key)
                                        .into(),
                                )));
                            } else {
                                continue;
                            }
                        }
                        Token::Colon | Token::Comma | Token::ArrayEnd | Token::ObjectEnd => {
                            self.finished = true;
                            return Some(Err(JQErr::InvalidStream));
                        }
                    },
                },
                Some(Scope::Array(_)) => match self.stream.next() {
                    None => {
                        self.finished = true;
                        return Some(Err(JQErr::InvalidStream));
                    }
                    Some(Err(err)) => {
                        self.finished = true;
                        return Some(Err(err));
                    }
                    Some(Ok(_)) => {
                        continue;
                    }
                },
                Some(Scope::Object | Scope::ObjectAtKey { .. }) => match self.stream.next() {
                    None => {
                        self.finished = true;
                        return Some(Err(JQErr::InvalidStream));
                    }
                    Some(Err(err)) => {
                        self.finished = true;
                        return Some(Err(err));
                    }
                    Some(Ok(mut token_kind)) => {
                        match token_kind {
                            Token::Comma => {}
                            Token::ObjectEnd => continue,
                            Token::ArrayEnd
                            | Token::ArrayStart
                            | Token::Colon
                            | Token::False
                            | Token::Null
                            | Token::Number(_)
                            | Token::ObjectStart
                            | Token::ParsedNumber(_)
                            | Token::String(_)
                            | Token::True => {
                                self.finished = true;
                                return Some(Err(JQErr::InvalidStream));
                            }
                        }

                        match self.stream.next() {
                            None => {
                                self.finished = true;
                                return Some(Err(JQErr::InvalidStream));
                            }
                            Some(Err(err)) => {
                                self.finished = true;
                                return Some(Err(err));
                            }
                            Some(Ok(inner_token_kind)) => token_kind = inner_token_kind,
                        }

                        let key = if let Token::String(key) = token_kind {
                            key
                        } else {
                            unreachable!();
                        };

                        match self.stream.next() {
                            None => {
                                self.finished = true;
                                return Some(Err(JQErr::InvalidStream));
                            }
                            Some(Err(err)) => {
                                self.finished = true;
                                return Some(Err(err));
                            }
                            Some(Ok(Token::Colon)) => {}
                            Some(Ok(_)) => {
                                self.finished = true;
                                return Some(Err(JQErr::InvalidStream));
                            }
                        }

                        match self.stream.next() {
                            Some(Err(err)) => {
                                self.finished = true;
                                return Some(Err(err));
                            }
                            Some(Ok(token_kind)) => {
                                if token_kind.is_value_start() {
                                    self.matching = *key == *self.key;
                                    if self.matching {
                                        return Some(Ok(token_kind));
                                    }
                                } else {
                                    self.finished = true;
                                    return Some(Err(JQErr::InvalidStream));
                                }
                            }
                            None => {
                                self.finished = true;
                                return Some(Err(JQErr::InvalidStream));
                            }
                        }
                    }
                },
            }
        }
    }
}

impl<const EMIT_ERRS: bool, Stream> SanitizedJQStream for ObjectKeyIndex<EMIT_ERRS, Stream> where
    Stream: JQStream
{
}
