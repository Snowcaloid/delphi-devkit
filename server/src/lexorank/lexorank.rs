use serde::{Deserialize, Serialize};

use super::*;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexoRank {
    bucket: Bucket,
    rank: Rank,
}

impl LexoRank {
    pub fn new(bucket: Bucket, rank: Rank) -> Self {
        LexoRank { bucket, rank }
    }

    pub fn from_string(value: &str) -> ParseResult<Self> {
        let parts = value.split('|').collect::<Vec<&str>>();
        let bucket = Bucket::new(parts[0].parse::<u8>()?)?;
        let rank = Rank::new(parts[1])?;

        Ok(LexoRank::new(bucket, rank))
    }

    pub fn from_string_or_default(value: &str) -> Self {
        LexoRank::from_string(value).unwrap_or_else(|_| LexoRank::default())
    }

    pub fn bucket(&self) -> &Bucket {
        &self.bucket
    }

    pub fn rank(&self) -> &Rank {
        &self.rank
    }

    pub fn next(&self) -> Self {
        LexoRank::new(self.bucket, self.rank.next())
    }

    pub fn prev(&self) -> Self {
        LexoRank::new(self.bucket, self.rank.prev())
    }

    pub fn between(&self, rank2: &Self) -> Option<Self> {
        self.rank
            .between(&rank2.rank)
            .map(|rank| LexoRank::new(self.bucket, rank))
    }
}

lazy_static::lazy_static! {
    static ref MIDDLE: LexoRank = LexoRank::new(
        Bucket::new(1).unwrap(),
        Rank::new("h").unwrap(),
    );
}

impl Default for LexoRank {
    fn default() -> Self {
        MIDDLE.clone()
    }
}

impl PartialOrd for LexoRank {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LexoRank {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.bucket.cmp(&other.bucket).then_with(|| self.rank.cmp(&other.rank))
    }
}

impl TryFrom<&str> for LexoRank {
    type Error = ParseError;

    fn try_from(value: &str) -> ParseResult<Self> {
        LexoRank::from_string(value)
    }
}

impl fmt::Display for LexoRank {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
        write!(f, "{}|{}", self.bucket.value(), self.rank.value())
    }
}

impl Serialize for LexoRank {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for LexoRank {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        LexoRank::from_string(&s).map_err(serde::de::Error::custom)
    }
}
