use toml::value::{Datetime as TomlDatetime, Offset as TomlOffset};

use crate::convert::error::ConvertError;
use crate::value::{Date, Datetime, DatetimeError, Int, LocalDateTime, Map, OffsetDateTime, Time, Value};

pub fn from_toml(source: &str) -> Result<Map, ConvertError> {
    let value: toml::Value = toml::from_str(source).map_err(|e| ConvertError::new(e.to_string()))?;
    match toml_to_value(value)? {
        Value::Map(m) => Ok(m),
        _ => Err(ConvertError::new("TOML root must be a table")),
    }
}

fn toml_to_value(v: toml::Value) -> Result<Value, ConvertError> {
    Ok(match v {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => Value::Int(Int::from_i64(i)),
        toml::Value::Float(f) => Value::from(f),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Datetime(dt) => Value::Datetime(convert_datetime(dt)?),
        toml::Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(toml_to_value(item)?);
            }
            Value::Array(out)
        }
        toml::Value::Table(table) => {
            let mut out = Map::new();
            for (k, v) in table {
                out.insert(k, toml_to_value(v)?);
            }
            Value::Map(out)
        }
    })
}

fn convert_datetime(dt: TomlDatetime) -> Result<Datetime, ConvertError> {
    let date = dt.date.map(|d| Date::new(d.year, d.month, d.day)).transpose().map_err(datetime_err)?;
    // `second`/`nanosecond` are optional as of the TOML 1.1.0 draft spec
    // (e.g. a bare `07:32` with no seconds) — default both to 0.
    let time = dt
        .time
        .map(|t| Time::new(t.hour, t.minute, t.second.unwrap_or(0), t.nanosecond.unwrap_or(0)))
        .transpose()
        .map_err(datetime_err)?;

    match (date, time, dt.offset) {
        (Some(date), Some(time), Some(offset)) => {
            let minutes = match offset {
                TomlOffset::Z => 0,
                TomlOffset::Custom { minutes } => minutes,
            };
            let odt = OffsetDateTime::new(LocalDateTime { date, time }, minutes).map_err(datetime_err)?;
            Ok(Datetime::Offset(odt))
        }
        (Some(date), Some(time), None) => Ok(Datetime::Local(LocalDateTime { date, time })),
        (Some(date), None, _) => Ok(Datetime::Date(date)),
        (None, Some(time), _) => Ok(Datetime::Time(time)),
        (None, None, _) => Err(ConvertError::new("empty TOML datetime")),
    }
}

fn datetime_err(e: DatetimeError) -> ConvertError {
    ConvertError::new(format!("invalid datetime: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_map_correctly() {
        let doc = from_toml("a = true\nb = 42\nc = 3.5\nd = \"hi\"\n").unwrap();
        assert_eq!(doc["a"], Value::from(true));
        assert_eq!(doc["b"], Value::from(42i64));
        assert_eq!(doc["c"], Value::from(3.5f64));
        assert_eq!(doc["d"], Value::from("hi"));
    }

    #[test]
    fn nested_table_and_array() {
        let doc = from_toml("[server]\nhost = \"0.0.0.0\"\nports = [80, 443]\n").unwrap();
        let server = doc["server"].as_map().unwrap();
        assert_eq!(server["host"], Value::from("0.0.0.0"));
        assert_eq!(server["ports"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn offset_datetime_maps_correctly() {
        let doc = from_toml("a = 2026-07-12T10:30:00+09:00\n").unwrap();
        assert!(matches!(doc["a"], Value::Datetime(Datetime::Offset(_))));
    }

    #[test]
    fn local_date_and_time_map_correctly() {
        let doc = from_toml("a = 2026-07-12\nb = 10:30:00\nc = 2026-07-12T10:30:00\n").unwrap();
        assert!(matches!(doc["a"], Value::Datetime(Datetime::Date(_))));
        assert!(matches!(doc["b"], Value::Datetime(Datetime::Time(_))));
        assert!(matches!(doc["c"], Value::Datetime(Datetime::Local(_))));
    }

    #[test]
    fn duplicate_keys_rejected_by_toml_itself() {
        assert!(from_toml("a = 1\na = 2\n").is_err());
    }

    #[test]
    fn malformed_toml_rejected() {
        assert!(from_toml("not valid [[[ toml").is_err());
    }
}
