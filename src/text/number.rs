use crate::value::{Date, Datetime, Int, LocalDateTime, OffsetDateTime, Time, Value};

use crate::text::cursor::Cursor;
use crate::text::error::Error;

/// Scans an int, float, date, time, or datetime literal. Cursor must be
/// positioned at the leading digit or `-`.
pub fn scan_number_or_datetime(cur: &mut Cursor) -> Result<Value, Error> {
    let mut raw = String::new();
    if cur.peek() == Some('-') {
        raw.push('-');
        cur.bump();
    }
    while let Some(c) = cur.peek() {
        if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '+' | '-') {
            raw.push(c);
            cur.bump();
        } else {
            break;
        }
    }
    // A date/time may join with a single-space separator instead of 'T'.
    if is_plain_date(&raw) && cur.peek() == Some(' ') && cur.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
        cur.bump();
        raw.push('T');
        while let Some(c) = cur.peek() {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | ':' | '+' | '-') {
                raw.push(c);
                cur.bump();
            } else {
                break;
            }
        }
    }
    classify(cur, &raw)
}

fn is_plain_date(s: &str) -> bool {
    s.len() == 10
        && s.as_bytes()[4] == b'-'
        && s.as_bytes()[7] == b'-'
        && s[0..4].bytes().all(|b| b.is_ascii_digit())
        && s[5..7].bytes().all(|b| b.is_ascii_digit())
        && s[8..10].bytes().all(|b| b.is_ascii_digit())
}

fn classify(cur: &Cursor, s: &str) -> Result<Value, Error> {
    if let Some(rest) = s.strip_prefix("0x") {
        return parse_radix_int(cur, rest, 16);
    }
    if let Some(rest) = s.strip_prefix("0o") {
        return parse_radix_int(cur, rest, 8);
    }
    if let Some(rest) = s.strip_prefix("0b") {
        return parse_radix_int(cur, rest, 2);
    }

    let has_time_marker = s.contains(':') || s.contains('T') || s.contains('t');
    if has_time_marker {
        return parse_time_or_datetime(cur, s);
    }
    if is_plain_date(s) {
        let date = parse_date_part(cur, s)?;
        return Ok(Value::Datetime(Datetime::Date(date)));
    }
    if s.contains('.') || s.contains('e') || s.contains('E') {
        return parse_float(cur, s);
    }
    parse_decimal_int(cur, s)
}

fn parse_radix_int(cur: &Cursor, digits: &str, radix: u32) -> Result<Value, Error> {
    let cleaned = strip_underscores(cur, digits)?;
    if cleaned.is_empty() {
        return Err(cur.error("empty numeric literal"));
    }
    // from_str_radix takes leading '+'
    if !cleaned.chars().all(|c| c.is_digit(radix)) {
        return Err(cur.error("invalid digit in integer literal"));
    }
    let v = u64::from_str_radix(&cleaned, radix).map_err(|_| cur.error("integer literal out of range or invalid"))?;
    Ok(Value::Int(Int::from_u64(v)))
}

fn parse_decimal_int(cur: &Cursor, s: &str) -> Result<Value, Error> {
    let (neg, digits) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let cleaned = strip_underscores(cur, digits)?;
    if cleaned.is_empty() {
        return Err(cur.error("empty numeric literal"));
    }
    if cleaned.len() > 1 && cleaned.starts_with('0') {
        return Err(cur.error("leading zeros are not allowed in integers"));
    }
    if !cleaned.bytes().all(|b| b.is_ascii_digit()) {
        return Err(cur.error("invalid integer literal"));
    }
    let magnitude: i128 = cleaned.parse().map_err(|_| cur.error("integer literal out of range"))?;
    let value = if neg { -magnitude } else { magnitude };
    Int::from_i128(value).map(Value::Int).ok_or_else(|| cur.error("integer literal out of range"))
}

fn strip_underscores(cur: &Cursor, s: &str) -> Result<String, Error> {
    if s.starts_with('_') || s.ends_with('_') || s.contains("__") {
        return Err(cur.error("misplaced '_' in numeric literal"));
    }
    Ok(s.replace('_', ""))
}

fn parse_float(cur: &Cursor, s: &str) -> Result<Value, Error> {
    let (neg, rest) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    // digits ['.' digits] [e [+-] digits], checked before '_' is removed
    let (mantissa, exponent) = match rest.find(['e', 'E']) {
        Some(e) => (&rest[..e], Some(&rest[e + 1..])),
        None => (rest, None),
    };
    let (int_part, frac_part) = match mantissa.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (mantissa, None),
    };
    if !is_digit_run(int_part) {
        return Err(if frac_part.is_some() && !int_part.contains('_') {
            cur.error("float requires a digit before '.'")
        } else {
            cur.error("invalid float literal")
        });
    }
    if let Some(f) = frac_part {
        if !is_digit_run(f) {
            return Err(if f.contains('_') { cur.error("invalid float literal") } else { cur.error("float requires a digit after '.'") });
        }
    }
    if let Some(exp) = exponent {
        let exp_digits = exp.strip_prefix(['+', '-']).unwrap_or(exp);
        if !is_digit_run(exp_digits) {
            return Err(cur.error("invalid float exponent"));
        }
    }
    let cleaned = rest.replace('_', "");

    let text = if neg { format!("-{cleaned}") } else { cleaned };
    let v: f64 = text.parse().map_err(|_| cur.error("invalid float literal"))?;
    Ok(Value::from(v))
}

/// Digits with '_' only between two digits.
fn is_digit_run(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b[0].is_ascii_digit()
        && b[b.len() - 1].is_ascii_digit()
        && b.iter().all(|c| c.is_ascii_digit() || *c == b'_')
        && !s.contains("__")
}

/// Two ASCII digits. `str::parse` would take `+1`.
fn two_digits(s: &str) -> Option<u8> {
    let b = s.as_bytes();
    (b.len() == 2 && b[0].is_ascii_digit() && b[1].is_ascii_digit()).then(|| (b[0] - b'0') * 10 + (b[1] - b'0'))
}

fn parse_date_part(cur: &Cursor, s: &str) -> Result<Date, Error> {
    let year: u16 = s[0..4].parse().map_err(|_| cur.error("invalid year"))?;
    let month: u8 = s[5..7].parse().map_err(|_| cur.error("invalid month"))?;
    let day: u8 = s[8..10].parse().map_err(|_| cur.error("invalid day"))?;
    Date::new(year, month, day).map_err(|_| cur.error("date out of range"))
}

fn parse_time_or_datetime(cur: &Cursor, s: &str) -> Result<Value, Error> {
    if let Some(tpos) = s.find(['T', 't']) {
        let date_part = &s[..tpos];
        let time_part = &s[tpos + 1..];
        if !is_plain_date(date_part) {
            return Err(cur.error("invalid datetime literal"));
        }
        let date = parse_date_part(cur, date_part)?;
        let (time, offset) = parse_time_part(cur, time_part)?;
        let local = LocalDateTime { date, time };
        return match offset {
            Some(minutes) => {
                let odt = OffsetDateTime::new(local, minutes).map_err(|_| cur.error("offset out of range"))?;
                Ok(Value::Datetime(Datetime::Offset(odt)))
            }
            None => Ok(Value::Datetime(Datetime::Local(local))),
        };
    }
    // Bare time: no offset permitted.
    let (time, offset) = parse_time_part(cur, s)?;
    if offset.is_some() {
        return Err(cur.error("bare time literal cannot have an offset"));
    }
    Ok(Value::Datetime(Datetime::Time(time)))
}

/// Parses `HH:MM:SS[.fraction][offset]`, returning the time and an optional
/// offset in minutes (`Z` = `Some(0)`, no suffix = `None`).
fn parse_time_part(cur: &Cursor, s: &str) -> Result<(Time, Option<i16>), Error> {
    if s.len() < 8 || s.as_bytes()[2] != b':' || s.as_bytes()[5] != b':' {
        return Err(cur.error("invalid time literal"));
    }
    let hour = two_digits(&s[0..2]).ok_or_else(|| cur.error("invalid hour"))?;
    let minute = two_digits(&s[3..5]).ok_or_else(|| cur.error("invalid minute"))?;
    let second = two_digits(&s[6..8]).ok_or_else(|| cur.error("invalid second"))?;

    let mut rest = &s[8..];
    let mut nanosecond: u32 = 0;
    if let Some(frac_rest) = rest.strip_prefix('.') {
        let end = frac_rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(frac_rest.len());
        let digits = &frac_rest[..end];
        if digits.is_empty() || digits.len() > 9 {
            return Err(cur.error("fractional seconds must have 1-9 digits"));
        }
        let mut padded = digits.to_string();
        while padded.len() < 9 {
            padded.push('0');
        }
        nanosecond = padded.parse().map_err(|_| cur.error("invalid fractional seconds"))?;
        rest = &frac_rest[end..];
    }

    let time = Time::new(hour, minute, second, nanosecond).map_err(|_| cur.error("time out of range"))?;

    if rest.is_empty() {
        return Ok((time, None));
    }
    if rest == "Z" || rest == "z" {
        return Ok((time, Some(0)));
    }
    let (sign, digits) = match rest.strip_prefix('+') {
        Some(d) => (1i16, d),
        None => match rest.strip_prefix('-') {
            Some(d) => (-1i16, d),
            None => return Err(cur.error("invalid offset")),
        },
    };
    if digits.len() != 5 || digits.as_bytes()[2] != b':' {
        return Err(cur.error("invalid offset"));
    }
    let oh = two_digits(&digits[0..2]).filter(|h| *h <= 23).ok_or_else(|| cur.error("invalid offset hour"))?;
    let om = two_digits(&digits[3..5]).filter(|m| *m <= 59).ok_or_else(|| cur.error("invalid offset minute"))?;
    let total = sign * (i16::from(oh) * 60 + i16::from(om));
    if total == 0 && sign < 0 {
        return Err(cur.error("-00:00 offset is not allowed"));
    }
    Ok((time, Some(total)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(input: &str) -> Result<Value, Error> {
        let mut cur = Cursor::new(input);
        scan_number_or_datetime(&mut cur)
    }

    #[test]
    fn decimal_ints() {
        assert_eq!(scan("42").unwrap(), Value::from(42i64));
        assert_eq!(scan("-17").unwrap(), Value::from(-17i64));
        assert_eq!(scan("1_000_000").unwrap(), Value::from(1_000_000i64));
    }

    #[test]
    fn leading_zero_rejected() {
        assert!(scan("007").is_err());
    }

    #[test]
    fn radix_ints() {
        assert_eq!(scan("0xDEAD_BEEF").unwrap(), Value::from(0xDEAD_BEEFi64));
        assert_eq!(scan("0o755").unwrap(), Value::from(0o755i64));
        assert_eq!(scan("0b1010").unwrap(), Value::from(0b1010i64));
    }

    #[test]
    fn floats() {
        assert_eq!(scan("3.25").unwrap(), Value::from(3.25_f64));
        assert_eq!(scan("-2.5e10").unwrap(), Value::from(-2.5e10f64));
        assert_eq!(scan("1e-3").unwrap(), Value::from(1e-3f64));
    }

    #[test]
    fn bare_dot_rejected() {
        assert!(scan("1.").is_err());
    }

    #[test]
    fn date_time_offset() {
        let v = scan("2026-07-12T10:30:00Z").unwrap();
        assert!(matches!(v, Value::Datetime(Datetime::Offset(_))));
        let v = scan("2026-07-12T10:30:00.5+09:00").unwrap();
        assert!(matches!(v, Value::Datetime(Datetime::Offset(_))));
        let v = scan("2026-07-12").unwrap();
        assert!(matches!(v, Value::Datetime(Datetime::Date(_))));
        let v = scan("10:30:00").unwrap();
        assert!(matches!(v, Value::Datetime(Datetime::Time(_))));
        let v = scan("2026-07-12T10:30:00").unwrap();
        assert!(matches!(v, Value::Datetime(Datetime::Local(_))));
    }

    #[test]
    fn offset_components_bounded() {
        assert!(scan("2026-01-01T00:00:00+00:99").is_err());
        assert!(scan("2026-01-01T00:00:00+24:00").is_err());
        assert!(scan("2026-01-01T00:00:00-23:59").is_ok());
    }

    #[test]
    fn signs_inside_fixed_width_fields_rejected() {
        assert!(scan("2026-01-01T00:00:00++1:00").is_err());
        assert!(scan("2026-01-01T+1:00:00").is_err());
        assert!(scan("00:+1:00").is_err());
        assert!(scan("0x+F").is_err());
        assert!(scan("0b+1").is_err());
    }

    #[test]
    fn float_underscores_only_between_digits() {
        for bad in ["1_.0", "1._0", "1e_3", "1_e3", "1.0e+_3", "1.0e3_", "1.0_e3", "1.0e_+3", "1.e3", "1.0e", "1.0e+"] {
            assert!(scan(bad).is_err(), "{bad}");
        }
        assert_eq!(scan("1_000.000_1e1_0").unwrap(), Value::from(1_000.000_1e1_0_f64));
        assert_eq!(scan("-1e+3").unwrap(), Value::from(-1e3_f64));
    }

    #[test]
    fn negative_offset_zero_rejected() {
        assert!(scan("2026-07-12T10:30:00-00:00").is_err());
    }

    #[test]
    fn out_of_range_int_rejected() {
        assert!(scan("99999999999999999999999999").is_err());
    }
}
