use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use tavra::{binary, Value};

use crate::value::{map_to_py, py_to_value};

fn extract_root(value: &Bound<'_, PyAny>) -> PyResult<tavra::Map> {
    match py_to_value(value)? {
        Value::Map(m) => Ok(m),
        _ => Err(PyValueError::new_err("value must be a dict (document root is always a map)")),
    }
}

#[pyfunction]
pub fn encode(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<Py<PyBytes>> {
    let root = extract_root(value)?;
    Ok(PyBytes::new(py, &binary::encode(&root)).unbind())
}

#[pyfunction]
pub fn decode(py: Python<'_>, data: &[u8]) -> PyResult<Py<PyAny>> {
    let root = binary::decode(data).map_err(|e| PyValueError::new_err(e.to_string()))?;
    map_to_py(py, &root)
}

#[pyfunction]
pub fn hash(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<Py<PyBytes>> {
    let root = extract_root(value)?;
    Ok(PyBytes::new(py, &binary::hash(&root)).unbind())
}
