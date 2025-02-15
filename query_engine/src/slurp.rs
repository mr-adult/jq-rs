use crate::{JQStream, JQErr, Sanitized, SanitizedJQStream, Token};

enum SlurpState {
    Start,
    Item,
    Finished,
}

pub struct Slurp<Stream>
where
    Stream: Iterator<Item = crate::Item>,
{
    stream: Sanitized<Stream>,
    state: SlurpState,
    comma_or_err_on_deck: Option<crate::Item>,
    value_on_deck: Option<Token>,
}

impl<Stream> Slurp<Stream>
where
    Stream: JQStream,
{
    pub(crate) fn new(stream: Stream) -> Self {
        Self {
            stream: stream.sanitize(),
            state: SlurpState::Start,
            comma_or_err_on_deck: None,
            value_on_deck: None,
        }
    }
}

impl<Stream> Iterator for Slurp<Stream>
where
    Stream: Iterator<Item = crate::Item>,
{
    type Item = crate::Item;

    fn next(&mut self) -> Option<crate::Item> {
        if let Some(on_deck) = self.comma_or_err_on_deck.take() {
            return Some(on_deck);
        }

        match self.state {
            SlurpState::Start => {
                self.state = SlurpState::Item;
                Some(Ok(Token::ArrayStart))
            }
            SlurpState::Item => match self
                .value_on_deck
                .take()
                .map(|token| Ok(token))
                .or_else(|| self.stream.next())
            {
                Some(Err(err)) => {
                    self.state = SlurpState::Finished;
                    Some(Err(err))
                }
                Some(Ok(token)) => match token {
                    Token::String(_)
                    | Token::Number(_)
                    | Token::ParsedNumber(_)
                    | Token::True
                    | Token::False
                    | Token::Null
                    | Token::ObjectEnd
                    | Token::ArrayEnd => {
                        if self.stream.get_path().is_empty() {
                            match self.stream.next() {
                                None => {}
                                Some(Err(err)) => {
                                    self.state = SlurpState::Finished;
                                    self.comma_or_err_on_deck = Some(Err(err));
                                }
                                Some(Ok(token)) => {
                                    if token.is_value_start() {
                                        self.comma_or_err_on_deck = Some(Ok(Token::Comma));
                                        self.value_on_deck = Some(token);
                                    } else {
                                        self.state = SlurpState::Finished;
                                        self.comma_or_err_on_deck =
                                            Some(Err(JQErr::InvalidStream));
                                    }
                                }
                            }
                        }

                        Some(Ok(token))
                    }
                    Token::ObjectStart | Token::ArrayStart | Token::Colon | Token::Comma => {
                        Some(Ok(token))
                    }
                },
                None => {
                    self.state = SlurpState::Finished;
                    Some(Ok(Token::ArrayEnd))
                }
            },
            SlurpState::Finished => None,
        }
    }
}

impl<Stream> SanitizedJQStream for Slurp<Stream> where Stream: SanitizedJQStream {}
