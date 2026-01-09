use std::error::Error;
use std::fmt;
use std::num::ParseIntError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ParseError {
    fn from(value: &str) -> Self {
        ParseError(value.to_string())
    }
}

impl From<ParseIntError> for ParseError {
    fn from(value: ParseIntError) -> Self {
        ParseError(format!("{}", value))
    }
}
