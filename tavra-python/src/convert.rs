use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use tavra::convert;

use crate::value::map_to_py;

#[pyfunction]
pub fn from_json(py: Python<'_>, source: &str) -> PyResult<Py<PyAny>> {
    let root = convert::from_json(source).map_err(|e| PyValueError::new_err(e.to_string()))?;
    map_to_py(py, &root)
}

#[pyfunction]
pub fn from_toml(py: Python<'_>, source: &str) -> PyResult<Py<PyAny>> {
    let root = convert::from_toml(source).map_err(|e| PyValueError::new_err(e.to_string()))?;
    map_to_py(py, &root)
}

#[pyfunction]
pub fn from_yaml(py: Python<'_>, source: &str) -> PyResult<Py<PyAny>> {
    let root = convert::from_yaml(source).map_err(|e| PyValueError::new_err(e.to_string()))?;
    map_to_py(py, &root)
}
