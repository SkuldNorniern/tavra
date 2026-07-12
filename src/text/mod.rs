//! Tavra text (`.tav`) parser.

mod cursor;
mod error;
mod fmt;
mod number;
mod parser;
mod string;

pub use error::Error;

use crate::value::{Map, Value};
use cursor::Cursor;

pub fn parse(input: &str) -> Result<Value, Error> {
    let mut cur = Cursor::new(input);
    parser::parse_document(&mut cur)
}

/// Renders a document's root map as canonically-ordered `.tav` text.
pub fn format(root: &Map) -> String {
    fmt::format(root)
}
