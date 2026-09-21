//! Import adapters from JSON, TOML, and YAML into the Tavra value model.
//! Import-only, no export back to these formats.

mod error;
#[cfg(feature = "json")]
mod json;
#[cfg(feature = "toml")]
mod toml;
#[cfg(feature = "yaml")]
mod yaml;

pub use error::ConvertError;
#[cfg(feature = "json")]
pub use json::from_json;
#[cfg(feature = "toml")]
pub use toml::from_toml;
#[cfg(feature = "yaml")]
pub use yaml::from_yaml;
