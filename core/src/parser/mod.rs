
pub mod token;
pub mod lexer;
pub mod precompiler;
pub mod directives;

lazy_static::lazy_static! {
    static ref STRINGS: lasso::ThreadedRodeo = lasso::ThreadedRodeo::new();
}