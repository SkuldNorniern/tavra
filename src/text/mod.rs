//! Tavra text (`.tav`) parser.

mod cursor;
mod error;
mod fmt;
mod number;
mod parser;
mod string;

pub use error::Error;

use crate::limits::Limits;
use crate::value::{Map, Value};
use cursor::Cursor;

pub fn parse(input: &str) -> Result<Value, Error> {
    parse_with_limits(input, &Limits::default())
}

/// [`parse`] with caller's limits. Only `max_depth` applies to text.
pub fn parse_with_limits(input: &str, limits: &Limits) -> Result<Value, Error> {
    let mut cur = Cursor::with_limits(input, limits);
    parser::parse_document(&mut cur)
}

/// Renders a document's root map as canonically-ordered `.tav` text.
pub fn format(root: &Map) -> String {
    fmt::format(root)
}
