use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bucket(u8);

impl Bucket {
    pub fn new(value: u8) -> ParseResult<Self> {
        if value > 2 {
            Err(ParseError(format!(
                "LexoRank bucket value must be between 0 and 2 inclusive. Found: {}",
                value
            )))
        } else {
            Ok(Bucket(value))
        }
    }

    pub fn value(&self) -> u8 {
        self.0
    }

    pub fn next(&self) -> Self {
        if self.0 == 2 {
            Bucket(0)
        } else {
            Bucket(self.0 + 1)
        }
    }

    pub fn prev(&self) -> Self {
        if self.0 == 0 {
            Bucket(2)
        } else {
            Bucket(self.0 - 1)
        }
    }
}

impl TryFrom<u8> for Bucket {
    type Error = ParseError;

    fn try_from(value: u8) -> ParseResult<Self> {
        Bucket::new(value)
    }
}
