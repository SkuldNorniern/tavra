//! Tavra: a data and configuration format with a canonical binary form and
//! a secure envelope (compression, authenticated encryption, signatures).

#[cfg(feature = "binary")]
pub mod binary;
pub mod convert;
#[cfg(feature = "envelope")]
pub mod envelope;
pub mod limits;
pub mod schema;
pub mod text;
pub mod value;

pub use limits::Limits;
pub use value::{Datetime, Float, Int, Map, Value};
