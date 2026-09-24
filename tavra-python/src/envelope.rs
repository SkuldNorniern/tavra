use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyTuple};

use tavra::envelope::{self, OpenMode, SealMode, SealOptions};
use tavra::Value;

use crate::value::{map_to_py, py_to_value};

fn extract_root(value: &Bound<'_, PyAny>) -> PyResult<tavra::Map> {
    match py_to_value(value)? {
        Value::Map(m) => Ok(m),
        _ => Err(PyValueError::new_err("value must be a dict (document root is always a map)")),
    }
}

fn require_32(bytes: &[u8], what: &str) -> PyResult<[u8; 32]> {
    bytes.try_into().map_err(|_| PyValueError::new_err(format!("{what} must be exactly 32 bytes, got {}", bytes.len())))
}

/// Seals a value into a `.tave` document. Exactly one of `key`/`password`
/// may be given; neither means unencrypted.
#[pyfunction]
#[pyo3(signature = (value, key=None, password=None, compress=false, sign_with=None))]
pub fn seal(py: Python<'_>, value: &Bound<'_, PyAny>, key: Option<&[u8]>, password: Option<&[u8]>, compress: bool, sign_with: Option<&[u8]>) -> PyResult<Py<PyBytes>> {
    if key.is_some() && password.is_some() {
        return Err(PyTypeError::new_err("key and password are mutually exclusive"));
    }
    let root = extract_root(value)?;

    let key_buf;
    let mode = match (key, password) {
        (Some(k), None) => {
            key_buf = require_32(k, "key")?;
            SealMode::Key(&key_buf)
        }
        (None, Some(p)) => SealMode::Password(p),
        (None, None) => SealMode::None,
        (Some(_), Some(_)) => unreachable!("checked above"),
    };

    let sign_buf;
    let sign_with = match sign_with {
        Some(s) => {
            sign_buf = require_32(s, "sign_with")?;
            Some(&sign_buf)
        }
        None => None,
    };

    let opts = SealOptions { mode, compress, sign_with };
    let sealed = envelope::seal(&root, &opts);
    Ok(PyBytes::new(py, &sealed).unbind())
}

/// Opens a `.tave` document. Exactly one of `key`/`password` may be given
/// if the document is encrypted. With `verify_with`, document must be
/// signed by that key.
#[pyfunction]
#[pyo3(signature = (data, key=None, password=None, verify_with=None))]
pub fn open(py: Python<'_>, data: &[u8], key: Option<&[u8]>, password: Option<&[u8]>, verify_with: Option<&[u8]>) -> PyResult<Py<PyAny>> {
    if key.is_some() && password.is_some() {
        return Err(PyTypeError::new_err("key and password are mutually exclusive"));
    }

    let key_buf;
    let mode = match (key, password) {
        (Some(k), None) => {
            key_buf = require_32(k, "key")?;
            OpenMode::Key(&key_buf)
        }
        (None, Some(p)) => OpenMode::Password(p),
        (None, None) => OpenMode::None,
        (Some(_), Some(_)) => unreachable!("checked above"),
    };

    let verify_buf;
    let verify_with = match verify_with {
        Some(v) => {
            verify_buf = require_32(v, "verify_with")?;
            Some(&verify_buf)
        }
        None => None,
    };

    let root = envelope::open(data, &mode, verify_with).map_err(|e| PyValueError::new_err(e.to_string()))?;
    map_to_py(py, &root)
}

#[pyfunction]
pub fn generate_key(py: Python<'_>) -> Py<PyBytes> {
    PyBytes::new(py, &envelope::generate_key()).unbind()
}

#[pyfunction]
pub fn generate_signing_key(py: Python<'_>) -> Py<PyTuple> {
    let (secret, public) = envelope::generate_signing_key();
    let secret = PyBytes::new(py, &secret);
    let public = PyBytes::new(py, &public);
    PyTuple::new(py, [secret, public]).unwrap_or_else(|_| unreachable!("2-element tuple construction cannot fail")).unbind()
}
