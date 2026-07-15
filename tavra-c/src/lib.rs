//! C API for Tavra: core round-trip only (parse/format, binary encode/
//! decode, envelope seal/open). See `include/tavra.h` for the stable ABI.
//!
//! Every fallible function returns `0` on success and nonzero on error;
//! call `tav_last_error()` for a message. Every function that returns an
//! owned buffer or handle documents its matching free function
//! (`tav_free_bytes`, `tav_value_free`).

mod binary;
mod envelope;
mod error;
mod text;
mod util;
mod value;

pub use binary::{tav_decode, tav_encode};
pub use envelope::{tav_genkey, tav_gensignkey, tav_open_key, tav_open_none, tav_open_password, tav_seal_key, tav_seal_none, tav_seal_password};
pub use error::tav_last_error;
pub use text::{tav_format, tav_parse};
pub use util::tav_free_bytes;
pub use value::{tav_value_free, TavValue};
