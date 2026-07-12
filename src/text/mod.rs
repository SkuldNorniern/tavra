//! Tavra text (`.tav`) parser.

mod cursor;
mod error;
mod number;
mod parser;
mod string;

pub use error::Error;

use crate::value::Value;
use cursor::Cursor;

pub fn parse(input: &str) -> Result<Value, Error> {
    let mut cur = Cursor::new(input);
    parser::parse_document(&mut cur)
}
