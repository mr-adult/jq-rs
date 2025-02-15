use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum Token {
    ObjectStart,
    ObjectEnd,
    ArrayStart,
    ArrayEnd,
    Colon,
    Comma,
    String(Rc<str>),
    Number(Rc<str>),
    ParsedNumber(f64),
    True,
    False,
    Null,
}

impl Token {
    /// A method to check if this token kind is the start of a JSON value
    pub fn is_value_start(&self) -> bool {
        match self {
            Token::String(_)
            | Token::Number(_)
            | Token::ParsedNumber(_)
            | Token::True
            | Token::False
            | Token::Null
            | Token::ObjectStart
            | Token::ArrayStart => true,
            Token::Comma | Token::Colon | Token::ObjectEnd | Token::ArrayEnd => false,
        }
    }
}
