use crate::{stream_context::StreamContext, JQStream, JQErr, SanitizedJQStream, Scope, Token};

/// A struct for handling the '.[]' or '.[]?' jq query.
pub struct Values<const EMIT_ERRS: bool, Stream>
where
    Stream: JQStream,
{
    finished: bool,
    stream: StreamContext<Stream>,
}

impl<const EMIT_ERRS: bool, Stream> Values<EMIT_ERRS, Stream>
where
    Stream: JQStream,
{
    pub fn new(stream: Stream) -> Self {
        Self {
            finished: false,
            stream: StreamContext::new(stream),
        }
    }
}

impl<const EMIT_ERRS: bool, Stream> Iterator for Values<EMIT_ERRS, Stream>
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
                match self.stream.next() {
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
            }

            match path.last().cloned() {
                None => match self.stream.next() {
                    None => return None,
                    Some(Err(err)) => {
                        self.finished = true;
                        return Some(Err(err));
                    }
                    Some(Ok(token_kind)) => match token_kind {
                        Token::ArrayStart => match self.stream.next() {
                            None => {
                                self.finished = true;
                                return Some(Err(JQErr::InvalidStream));
                            }
                            Some(Err(err)) => {
                                self.finished = true;
                                return Some(Err(err));
                            }
                            Some(Ok(token_kind)) => {
                                if token_kind.is_value_start() {
                                    return Some(Ok(token_kind));
                                } else {
                                    self.finished = true;
                                    return Some(Err(JQErr::InvalidStream));
                                }
                            }
                        },
                        Token::ObjectStart => {
                            match self.stream.next() {
                                None => {
                                    self.finished = true;
                                    return Some(Err(JQErr::InvalidStream));
                                }
                                Some(Err(err)) => {
                                    self.finished = true;
                                    return Some(Err(err));
                                }
                                Some(Ok(Token::String(_))) => {}
                                Some(Ok(_)) => {
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
                                        return Some(Ok(token_kind));
                                    } else {
                                        self.finished = true;
                                        return Some(Err(JQErr::InvalidStream));
                                    }
                                }
                            }
                        }
                        Token::False => {
                            if EMIT_ERRS {
                                self.finished = true;
                                return Some(Err(JQErr::StreamOperationFailed(
                                    "Cannot iterate over boolean (false)".into(),
                                )));
                            } else {
                                continue;
                            }
                        }
                        Token::True => {
                            if EMIT_ERRS {
                                self.finished = true;
                                return Some(Err(JQErr::StreamOperationFailed(
                                    "Cannot iterate over boolean (true)".into(),
                                )));
                            } else {
                                continue;
                            }
                        }
                        Token::Null => {
                            if EMIT_ERRS {
                                self.finished = true;
                                return Some(Err(JQErr::StreamOperationFailed(
                                    "Cannot iterate over null (null)".into(),
                                )));
                            } else {
                                continue;
                            }
                        }
                        Token::String(value) => {
                            if EMIT_ERRS {
                                self.finished = true;
                                return Some(Err(JQErr::StreamOperationFailed(
                                    format!("Cannot iterator over string ({value})").into(),
                                )));
                            } else {
                                continue;
                            }
                        }
                        Token::Number(value) => {
                            if EMIT_ERRS {
                                self.finished = true;
                                return Some(Err(JQErr::StreamOperationFailed(
                                    format!("Cannot iterate over number ({value})").into(),
                                )));
                            } else {
                                continue;
                            }
                        }
                        Token::ParsedNumber(value) => {
                            if EMIT_ERRS {
                                self.finished = true;
                                return Some(Err(JQErr::StreamOperationFailed(
                                    format!("Cannot iterate over number ({value})").into(),
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
                    Some(Ok(mut token_kind)) => {
                        match token_kind {
                            Token::Comma => {}
                            Token::ArrayEnd => continue,
                            Token::ArrayStart
                            | Token::Colon
                            | Token::False
                            | Token::Null
                            | Token::Number(_)
                            | Token::ObjectEnd
                            | Token::ObjectStart
                            | Token::ParsedNumber(_)
                            | Token::String(_)
                            | Token::True => {
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
                            Some(Ok(inner_token_kind)) => token_kind = inner_token_kind,
                        }

                        if token_kind.is_value_start() {
                            return Some(Ok(token_kind));
                        } else {
                            self.finished = true;
                            return Some(Err(JQErr::InvalidStream));
                        }
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
                        assert!(matches!(token_kind, Token::String(_)));

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
                                    return Some(Ok(token_kind));
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

impl<const EMIT_ERRS: bool, Stream> SanitizedJQStream for Values<EMIT_ERRS, Stream> where
    Stream: JQStream
{
}
