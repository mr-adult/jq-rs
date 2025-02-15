use crate::location::Location;

#[derive(Clone, Debug, Default)]
pub(crate) struct Span {
    pub start: Location,
    pub end: Location,
}
