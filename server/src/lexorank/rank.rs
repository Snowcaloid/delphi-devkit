use super::*;
use error::ParseError;
use helpers::{decrement_char, increment_char};
use lazy_static::lazy_static;
use regex::Regex;
use substring::Substring;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rank(String);

impl Rank {
    pub fn new(value: &str) -> ParseResult<Self> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^[0-9a-z]*[1-9a-z]$").unwrap();
        }

        if !RE.is_match(value) {
            Err(ParseError(format!(
                "Lexorank value must only include 0-9 and a-z and must not end with 0. Found: {}",
                value,
            )))
        } else {
            Ok(Rank(value.to_owned()))
        }
    }

    pub fn value(&self) -> &str {
        &self.0
    }

    pub fn next(&self) -> Self {
        for (i, c) in self.0.chars().rev().enumerate() {
            let i = self.0.len() - i - 1;

            if c == 'z' {
                continue;
            };

            let next_c = increment_char(&c).expect(&format!("failed to increment char '{}'", c));
            let new_value = self.0.substring(0, i).to_owned() + &next_c.to_string();
            return Rank(new_value);
        }

        let new_value = self.0.to_owned() + "1";
        return Rank(new_value);
    }

    pub fn prev(&self) -> Self {
        let length = self.0.len();
        let c = self.0.chars().rev().nth(0).unwrap();

        if c != '1' {
            let new_value = self.0.substring(0, length - 1).to_owned()
                + &decrement_char(&c)
                    .expect(&format!("failed to decrement char '{}'", c))
                    .to_string();

            return Rank(new_value);
        }

        for (i, c) in self.0.chars().rev().enumerate() {
            // Skip last char which is '1'
            if i == 0 {
                continue;
            }

            // Use correct string index
            let i = self.0.len() - i - 1;

            // Must have non-zero leading chars in order to truncate
            if c == '0' {
                continue;
            };

            let new_value = self.0.substring(0, i + 1).to_owned();
            return Rank(new_value);
        }

        // Can't decrement char or truncate so slap a 0 on the front
        let new_value = "0".to_owned() + &self.0;
        return Rank(new_value);
    }

    pub fn between(&self, rank2: &Self) -> Option<Self> {
        if self == rank2 {
            return None;
        }

        let (lesser, greater) = if self < rank2 {
            (self, rank2)
        } else {
            (rank2, self)
        };

        let incremented = lesser.next();
        if incremented < *greater {
            return Some(incremented);
        }

        let plus1 = lesser.append("1");
        if plus1 < *greater {
            return Some(plus1);
        }

        let mut pre = "0".to_owned();
        let mut plus01 = lesser.append(&format!("{}1", pre));

        while plus01 >= *greater {
            pre.push('0');
            plus01 = lesser.append(&format!("{}1", pre));
        }

        Some(plus01)
    }

    fn append(&self, value: &str) -> Self {
        Rank(self.0.to_owned() + value)
    }
}

impl TryFrom<&str> for Rank {
    type Error = ParseError;

    fn try_from(value: &str) -> ParseResult<Self> {
        Rank::new(value)
    }
}

mod helpers {
    pub fn increment_char(c: &char) -> Option<char> {
        if c == &'z' {
            None
        } else if c == &'9' {
            Some('a')
        } else {
            Some(char::from_u32(*c as u32 + 1).unwrap())
        }
    }

    pub fn decrement_char(c: &char) -> Option<char> {
        if c == &'0' {
            None
        } else if c == &'a' {
            Some('9')
        } else {
            Some(char::from_u32(*c as u32 - 1).unwrap())
        }
    }
}
