//! Conversion between `tavra::Value` and native Python objects: `None`,
//! `bool`, `int`, `float`, `str`, `bytes`, `datetime.{date,time,datetime}`,
//! `list`, `dict`.
//!
//! Python's `datetime` only holds microsecond precision; Tavra's `Time`
//! holds nanoseconds. Converting to Python truncates (not rounds) to
//! microseconds — there's no finer-grained stdlib type to map onto, so
//! this isn't a choice between alternatives, just the ceiling of what
//! Python can represent.

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyDict, PyList, PyString};

use tavra::value::{Date, Datetime, LocalDateTime, MAX_DEPTH, OffsetDateTime, Time};
use tavra::{Int, Map, Value};

pub fn value_to_py(py: Python<'_>, value: &Value) -> PyResult<Py<PyAny>> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().unbind().into()),
        Value::Int(i) => int_to_py(py, *i),
        Value::Float(f) => Ok(f.get().into_pyobject(py)?.unbind().into()),
        Value::String(s) => Ok(PyString::new(py, s).unbind().into()),
        Value::Bytes(b) => Ok(PyBytes::new(py, b).unbind().into()),
        Value::Datetime(dt) => datetime_to_py(py, dt),
        Value::Array(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(value_to_py(py, item)?)?;
            }
            Ok(list.unbind().into())
        }
        Value::Map(map) => map_to_py(py, map),
    }
}

pub fn map_to_py(py: Python<'_>, map: &Map) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    for (k, v) in map.iter() {
        dict.set_item(k, value_to_py(py, v)?)?;
    }
    Ok(dict.unbind().into())
}

fn int_to_py(py: Python<'_>, i: Int) -> PyResult<Py<PyAny>> {
    if let Some(v) = i.as_i64() {
        Ok(v.into_pyobject(py)?.unbind().into())
    } else {
        let v = i.as_u64().ok_or_else(|| PyValueError::new_err("integer out of i64/u64 range"))?;
        Ok(v.into_pyobject(py)?.unbind().into())
    }
}

fn datetime_to_py(py: Python<'_>, dt: &Datetime) -> PyResult<Py<PyAny>> {
    let time = match dt {
        Datetime::Date(_) => None,
        Datetime::Time(t) => Some(t),
        Datetime::Local(ldt) => Some(&ldt.time),
        Datetime::Offset(odt) => Some(&odt.datetime.time),
    };
    if time.is_some_and(|t| t.second() == 60) {
        return Err(PyValueError::new_err("leap second (:60) has no Python datetime equivalent"));
    }
    let datetime_mod = py.import("datetime")?;
    match dt {
        Datetime::Date(d) => {
            let cls = datetime_mod.getattr("date")?;
            Ok(cls.call1((d.year(), d.month(), d.day()))?.unbind())
        }
        Datetime::Time(t) => {
            let cls = datetime_mod.getattr("time")?;
            Ok(cls.call1((t.hour(), t.minute(), t.second(), t.nanosecond() / 1000))?.unbind())
        }
        Datetime::Local(ldt) => local_datetime_to_py(py, &datetime_mod, ldt, None),
        Datetime::Offset(odt) => {
            let timedelta = datetime_mod.getattr("timedelta")?.call((), Some(&minutes_kwargs(py, odt.offset_minutes())?))?;
            let tzinfo = datetime_mod.getattr("timezone")?.call1((timedelta,))?;
            local_datetime_to_py(py, &datetime_mod, &odt.datetime, Some(&tzinfo))
        }
    }
}

fn minutes_kwargs(py: Python<'_>, minutes: i16) -> PyResult<Bound<'_, PyDict>> {
    let kwargs = PyDict::new(py);
    kwargs.set_item("minutes", minutes)?;
    Ok(kwargs)
}

fn local_datetime_to_py(py: Python<'_>, datetime_mod: &Bound<'_, PyModule>, ldt: &LocalDateTime, tzinfo: Option<&Bound<'_, PyAny>>) -> PyResult<Py<PyAny>> {
    let cls = datetime_mod.getattr("datetime")?;
    let microsecond = ldt.time.nanosecond() / 1000;
    let args = (ldt.date.year(), ldt.date.month(), ldt.date.day(), ldt.time.hour(), ldt.time.minute(), ldt.time.second(), microsecond);
    match tzinfo {
        Some(tz) => {
            let kwargs = PyDict::new(py);
            kwargs.set_item("tzinfo", tz)?;
            Ok(cls.call(args, Some(&kwargs))?.unbind())
        }
        None => Ok(cls.call1(args)?.unbind()),
    }
}

/// Converts document root. Top-level dict is root map, so its values are depth 0.
pub fn py_to_value(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    match obj.cast::<PyDict>() {
        Ok(dict) => py_to_map_at(dict, 0).map(Value::Map),
        Err(_) => py_to_value_at(obj, 0),
    }
}

/// `depth` = lists/dicts around `obj`, below root. Stops self-referencing list.
fn py_to_value_at(obj: &Bound<'_, PyAny>, depth: u32) -> PyResult<Value> {
    if obj.is_none() {
        return Ok(Value::Null);
    }
    if let Ok(b) = obj.cast::<PyBool>() {
        return Ok(Value::Bool(b.is_true()));
    }
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(Value::Int(Int::from_i64(i)));
    }
    if let Ok(u) = obj.extract::<u64>() {
        return Ok(Value::Int(Int::from_u64(u)));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(Value::from(f));
    }
    if let Ok(s) = obj.extract::<String>() {
        return Ok(Value::String(s));
    }
    if let Ok(b) = obj.cast::<PyBytes>() {
        return Ok(Value::Bytes(b.as_bytes().to_vec()));
    }
    if let Some(dt) = py_to_datetime(obj)? {
        return Ok(Value::Datetime(dt));
    }
    if (obj.cast::<PyList>().is_ok() || obj.cast::<PyDict>().is_ok()) && depth >= MAX_DEPTH {
        return Err(PyValueError::new_err(format!("nested deeper than {MAX_DEPTH}")));
    }
    if let Ok(list) = obj.cast::<PyList>() {
        let mut items = Vec::with_capacity(list.len());
        for item in list.iter() {
            items.push(py_to_value_at(&item, depth + 1)?);
        }
        return Ok(Value::Array(items));
    }
    if let Ok(dict) = obj.cast::<PyDict>() {
        return Ok(Value::Map(py_to_map_at(dict, depth + 1)?));
    }
    Err(PyTypeError::new_err(format!("unsupported Python type for Tavra value: {}", obj.get_type().name()?)))
}

fn py_to_map_at(dict: &Bound<'_, PyDict>, depth: u32) -> PyResult<Map> {
    let mut map = Map::new();
    for (k, v) in dict.iter() {
        let key: String = k.extract().map_err(|_| PyTypeError::new_err("Tavra map keys must be strings"))?;
        let value = py_to_value_at(&v, depth)?;
        if map.insert(key.clone(), value).is_some() {
            return Err(PyValueError::new_err(format!("duplicate key '{key}'")));
        }
    }
    Ok(map)
}

fn py_to_datetime(obj: &Bound<'_, PyAny>) -> PyResult<Option<Datetime>> {
    let py = obj.py();
    let datetime_mod = py.import("datetime")?;

    // datetime.datetime is a subclass of datetime.date, so it must be
    // checked first.
    let datetime_cls = datetime_mod.getattr("datetime")?;
    if obj.is_instance(&datetime_cls)? {
        let year: u16 = obj.getattr("year")?.extract()?;
        let month: u8 = obj.getattr("month")?.extract()?;
        let day: u8 = obj.getattr("day")?.extract()?;
        let hour: u8 = obj.getattr("hour")?.extract()?;
        let minute: u8 = obj.getattr("minute")?.extract()?;
        let second: u8 = obj.getattr("second")?.extract()?;
        let microsecond: u32 = obj.getattr("microsecond")?.extract()?;
        let date = Date::new(year, month, day).map_err(|e| PyValueError::new_err(format!("{e:?}")))?;
        let time = Time::new(hour, minute, second, microsecond * 1000).map_err(|e| PyValueError::new_err(format!("{e:?}")))?;
        let local = LocalDateTime { date, time };

        // tzinfo can return None, then it's naive
        let offset_delta = obj.call_method0("utcoffset")?;
        if offset_delta.is_none() {
            return Ok(Some(Datetime::Local(local)));
        }
        // tavra offsets are whole minutes
        let days: i64 = offset_delta.getattr("days")?.extract()?;
        let seconds: i64 = offset_delta.getattr("seconds")?.extract()?;
        let microseconds: i64 = offset_delta.getattr("microseconds")?.extract()?;
        let total_seconds = days * 86_400 + seconds;
        if microseconds != 0 || total_seconds % 60 != 0 {
            return Err(PyValueError::new_err("UTC offset must be a whole number of minutes"));
        }
        let offset_minutes = i16::try_from(total_seconds / 60).map_err(|_| PyValueError::new_err("UTC offset out of range"))?;
        let odt = OffsetDateTime::new(local, offset_minutes).map_err(|e| PyValueError::new_err(format!("{e:?}")))?;
        return Ok(Some(Datetime::Offset(odt)));
    }

    let date_cls = datetime_mod.getattr("date")?;
    if obj.is_instance(&date_cls)? {
        let year: u16 = obj.getattr("year")?.extract()?;
        let month: u8 = obj.getattr("month")?.extract()?;
        let day: u8 = obj.getattr("day")?.extract()?;
        let date = Date::new(year, month, day).map_err(|e| PyValueError::new_err(format!("{e:?}")))?;
        return Ok(Some(Datetime::Date(date)));
    }

    let time_cls = datetime_mod.getattr("time")?;
    if obj.is_instance(&time_cls)? {
        let hour: u8 = obj.getattr("hour")?.extract()?;
        let minute: u8 = obj.getattr("minute")?.extract()?;
        let second: u8 = obj.getattr("second")?.extract()?;
        let microsecond: u32 = obj.getattr("microsecond")?.extract()?;
        if !obj.getattr("tzinfo")?.is_none() {
            return Err(PyValueError::new_err("a time with tzinfo has no Tavra equivalent (bare times carry no offset)"));
        }
        let time = Time::new(hour, minute, second, microsecond * 1000).map_err(|e| PyValueError::new_err(format!("{e:?}")))?;
        return Ok(Some(Datetime::Time(time)));
    }

    Ok(None)
}
