//! Python bindings for Tavra (full parity: value model, text, binary,
//! envelope, schema, and JSON/TOML/YAML import).

use pyo3::prelude::*;

mod binary;
mod convert;
mod envelope;
mod schema;
mod text;
mod value;

#[pymodule]
fn tavra(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(text::parse, m)?)?;
    m.add_function(wrap_pyfunction!(text::format, m)?)?;

    m.add_function(wrap_pyfunction!(binary::encode, m)?)?;
    m.add_function(wrap_pyfunction!(binary::decode, m)?)?;
    m.add_function(wrap_pyfunction!(binary::hash, m)?)?;

    m.add_function(wrap_pyfunction!(envelope::seal, m)?)?;
    m.add_function(wrap_pyfunction!(envelope::open, m)?)?;
    m.add_function(wrap_pyfunction!(envelope::generate_key, m)?)?;
    m.add_function(wrap_pyfunction!(envelope::generate_signing_key, m)?)?;

    m.add_function(wrap_pyfunction!(schema::validate, m)?)?;

    m.add_function(wrap_pyfunction!(convert::from_json, m)?)?;
    m.add_function(wrap_pyfunction!(convert::from_toml, m)?)?;
    m.add_function(wrap_pyfunction!(convert::from_yaml, m)?)?;

    Ok(())
}
