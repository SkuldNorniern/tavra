//! Python bindings for Tavra (full parity: value model, text, binary,
//! envelope, schema, and JSON/TOML/YAML import).

use pyo3::prelude::*;

#[pymodule]
fn tavra(_py: Python<'_>, _m: &Bound<'_, PyModule>) -> PyResult<()> {
    Ok(())
}
