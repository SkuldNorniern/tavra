//! Import adapters from JSON, TOML, and YAML into the Tavra value model.
//! Import-only — there is no export back to these formats (see
//! `docs/status.md` for why: Tavra has bytes/datetime/nan/inf, none of
//! which map cleanly back to plain JSON).

mod error;
mod json;
mod toml_fmt;
mod yaml;

pub use error::ConvertError;
pub use json::from_json;
pub use toml_fmt::from_toml;
pub use yaml::from_yaml;
