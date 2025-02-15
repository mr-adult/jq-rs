use std::{collections::VecDeque, ops::Range, vec::IntoIter};

use crate::{stream_context::StreamContext, JQStream, JsonErr, SanitizedJQStream, Scope, Token};

pub struct ArraySliceIndex<const EMIT_ERRS: bool, Stream>
where
    Stream: JQStream,
{
    finished: bool,
    stream: StreamContext<Stream>,
    range: Range<isize>,
    matching: bool,
    queue: VecDeque<Vec<Token>>,
    matched_vec: Option<IntoIter<Token>>,
}

impl<const EMIT_ERRS: bool, Stream> ArraySliceIndex<EMIT_ERRS, Stream>
where
    Stream: JQStream,
{
    pub(crate) fn new(stream: Stream, range: Range<isize>) -> Self {
        Self {
            finished: false,
            stream: StreamContext::new(stream),
            range,
            matching: false,
            queue: VecDeque::new(),
            matched_vec: None,
        }
    }

    pub fn queue(&self) -> &VecDeque<Vec<Token>> {
        &self.queue
    }

    fn finish(&mut self) {
        self.matching = false;
        self.finished = true;
        self.drop_queue();
    }

    fn drop_queue(&mut self) {
        self.queue.clear();
        self.queue.shrink_to(self.index.abs() as usize);
    }
}

impl<const EMIT_ERRS: bool, Stream> Iterator for ArraySliceIndex<EMIT_ERRS, Stream>
where
    Stream: JQStream,
{
    type Item = crate::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        if let Some(iter) = &mut self.matched_vec {
            if let Some(token) = iter.next() {
                return Some(Ok(token));
            }
        }

        loop {
            let path = self.stream.get_path();
            if path.len() > 1 {
                let next = self.stream.next();
                if !self.matching {
                    continue;
                } else {
                    match next {
                        None => {
                            self.finish();
                            return Some(Err(JsonErr::InvalidStream));
                        }
                        Some(Err(err)) => {
                            self.finish();
                            return Some(Err(err));
                        }
                        Some(Ok(token)) => {
                            if self.index < 0 {
                                self.queue.iter_mut().last().unwrap().push(token);
                            } else {
                                return Some(Ok(token));
                            }
                            continue;
                        }
                    }
                }
            }

            match path.last().cloned() {
                None => match self.stream.next() {
                    None => return None,
                    Some(Err(err)) => {
                        self.finish();
                        return Some(Err(err));
                    }
                    Some(Ok(token_kind)) => match token_kind {
                        Token::ArrayStart => match self.stream.next() {
                            None => {
                                self.finish();
                                return Some(Err(JsonErr::InvalidStream));
                            }
                            Some(Err(err)) => {
                                self.finish();
                                return Some(Err(err));
                            }
                            Some(Ok(token_kind)) => {
                                if token_kind.is_value_start() {
                                    self.drop_queue();

                                    self.matching = self.index <= 0;

                                    if self.index == 0 {
                                        return Some(Ok(token_kind));
                                    } else if self.index < 0 {
                                        let mut value_pieces = Vec::new();
                                        value_pieces.push(token_kind);
                                        self.queue.push_back(value_pieces);
                                    }

                                    continue;
                                } else if matches!(token_kind, Token::ArrayEnd) {
                                    self.matching = false;
                                    self.drop_queue();
                                    return Some(Ok(Token::Null));
                                } else {
                                    self.finish();
                                    return Some(Err(JsonErr::InvalidStream));
                                }
                            }
                        },
                        Token::ObjectStart => {
                            if EMIT_ERRS {
                                self.finish();
                                return Some(Err(JsonErr::StreamOperationFailed(
                                    "Cannot index object with number".into(),
                                )));
                            } else {
                                self.matching = false;
                                continue;
                            }
                        }
                        Token::True | Token::False => {
                            if EMIT_ERRS {
                                self.finish();
                                return Some(Err(JsonErr::StreamOperationFailed(
                                    "Cannot index boolean with number".into(),
                                )));
                            } else {
                                self.matching = false;
                                continue;
                            }
                        }
                        Token::Null => {
                            self.matching = false;
                            return Some(Ok(Token::Null));
                        }
                        Token::String(_) => {
                            if EMIT_ERRS {
                                self.finish();
                                return Some(Err(JsonErr::StreamOperationFailed(
                                    format!("Cannot index string with number").into(),
                                )));
                            } else {
                                self.matching = false;
                                continue;
                            }
                        }
                        Token::Number(_) | Token::ParsedNumber(_) => {
                            if EMIT_ERRS {
                                self.finish();
                                return Some(Err(JsonErr::StreamOperationFailed(
                                    format!("Cannot index number with number").into(),
                                )));
                            } else {
                                self.matching = false;
                                continue;
                            }
                        }
                        Token::Colon | Token::Comma | Token::ArrayEnd | Token::ObjectEnd => {
                            self.finish();
                            return Some(Err(JsonErr::InvalidStream));
                        }
                    },
                },
                Some(Scope::Array(index)) => match self.stream.next() {
                    None => {
                        self.finish();
                        return Some(Err(JsonErr::InvalidStream));
                    }
                    Some(Err(err)) => {
                        self.finish();
                        return Some(Err(err));
                    }
                    Some(Ok(mut token_kind)) => {
                        match token_kind {
                            Token::Comma => {
                                let index_abs = self.index.abs() as usize;
                                if self.index < 0 {
                                    self.matching = true;

                                    if self.queue.len() == index_abs {
                                        self.queue.pop_front();
                                    }

                                    self.queue.push_back(Vec::new());
                                } else if index_abs == (index + 1) {
                                    self.matching = true;
                                } else {
                                    self.matching = false;
                                }
                            }
                            Token::ArrayEnd => {
                                self.matching = false;

                                let target_index_abs = self.index.abs() as usize;
                                if self.index < 0 {
                                    let queue_len = self.queue.len();
                                    if queue_len < target_index_abs {
                                        self.drop_queue();
                                        return Some(Ok(Token::Null));
                                    }

                                    match self.queue.get_mut(queue_len - target_index_abs).take() {
                                        None => {
                                            self.drop_queue();
                                            return Some(Ok(Token::Null));
                                        }
                                        Some(vec) => {
                                            let vec = std::mem::take(vec);
                                            let mut iter = vec.into_iter();
                                            if let Some(next) = iter.next() {
                                                self.matched_vec = Some(iter);
                                                return Some(Ok(next));
                                            } else {
                                                return Some(Err(JsonErr::InvalidStream));
                                            }
                                        }
                                    }
                                } else if target_index_abs > index {
                                    return Some(Ok(Token::Null));
                                }

                                continue;
                            }
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
                                self.finish();
                                return Some(Err(JsonErr::InvalidStream));
                            }
                        };

                        match self.stream.next() {
                            None => {
                                self.finish();
                                return Some(Err(JsonErr::InvalidStream));
                            }
                            Some(Err(err)) => {
                                self.finish();
                                return Some(Err(err));
                            }
                            Some(Ok(inner_token_kind)) => token_kind = inner_token_kind,
                        }

                        if token_kind.is_value_start() {
                            if self.matching {
                                if self.index < 0 {
                                    self.queue.iter_mut().last().unwrap().push(token_kind);
                                } else {
                                    return Some(Ok(token_kind));
                                }
                            }

                            continue;
                        } else {
                            self.finish();
                            return Some(Err(JsonErr::InvalidStream));
                        }
                    }
                },
                Some(Scope::Object | Scope::ObjectAtKey { .. }) => match self.stream.next() {
                    None => {
                        self.finish();
                        return Some(Err(JsonErr::InvalidStream));
                    }
                    Some(Err(err)) => {
                        self.finish();
                        return Some(Err(err));
                    }
                    Some(Ok(_)) => {
                        self.matching = false;
                        continue;
                    }
                },
            }
        }
    }
}

impl<const EMIT_ERRS: bool, Stream> SanitizedJQStream for ArraySliceIndex<EMIT_ERRS, Stream> where
    Stream: JQStream
{
}
