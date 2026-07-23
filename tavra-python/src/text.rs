use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use tavra::text;
use tavra::Value;

use crate::value::{map_to_py, py_to_value};

#[pyfunction]
pub fn parse(py: Python<'_>, source: &str) -> PyResult<Py<PyAny>> {
    match text::parse(source) {
        Ok(Value::Map(root)) => map_to_py(py, &root),
        Ok(_) => unreachable!("document root is always a map"),
        Err(e) => Err(PyValueError::new_err(e.to_string())),
    }
}

#[pyfunction]
pub fn format(value: &Bound<'_, PyAny>) -> PyResult<String> {
    let root = match py_to_value(value)? {
        Value::Map(m) => m,
        _ => return Err(PyValueError::new_err("value must be a dict (document root is always a map)")),
    };
    Ok(text::format(&root))
}
