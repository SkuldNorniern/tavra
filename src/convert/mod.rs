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

#[cfg(any(feature = "json", feature = "toml", feature = "yaml"))]
use crate::value::{MAX_DEPTH, Map, Value};

/// Rejects nesting past [`MAX_DEPTH`]. YAML parser allows deeper than tav.
#[cfg(any(feature = "json", feature = "toml", feature = "yaml"))]
fn check_depth(root: Map) -> Result<Map, ConvertError> {
    fn exceeds(v: &Value, depth: u32) -> bool {
        match v {
            Value::Array(items) => depth >= MAX_DEPTH || items.iter().any(|i| exceeds(i, depth + 1)),
            Value::Map(m) => depth >= MAX_DEPTH || m.values().any(|i| exceeds(i, depth + 1)),
            _ => false,
        }
    }
    if root.values().any(|v| exceeds(v, 0)) {
        return Err(ConvertError::new(format!("nested deeper than {MAX_DEPTH}")));
    }
    Ok(root)
}

#[cfg(all(test, feature = "yaml"))]
mod tests {
    use crate::value::MAX_DEPTH;

    #[test]
    fn imports_nested_past_the_limit_are_refused() {
        let depth = MAX_DEPTH as usize;
        let at_limit = format!("a: {}{}\n", "[".repeat(depth), "]".repeat(depth));
        assert!(super::from_yaml(&at_limit).is_ok());
        let past = format!("a: {}{}\n", "[".repeat(depth + 1), "]".repeat(depth + 1));
        let err = super::from_yaml(&past).unwrap_err();
        assert!(err.to_string().contains("nested deeper"), "{err}");
    }
}
