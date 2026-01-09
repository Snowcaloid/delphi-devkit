mod bucket;
pub mod error;
mod lexorank;
mod rank;

pub use lexorank::LexoRank;
pub use bucket::Bucket;
pub use error::ParseError;
pub use rank::Rank;

type ParseResult<T> = std::result::Result<T, ParseError>;
