//! Tavra: a data and configuration format with a canonical binary form and
//! a secure envelope (compression, authenticated encryption, signatures).

pub mod text;
pub mod value;

pub use value::{Datetime, Float, Int, Map, Value};
