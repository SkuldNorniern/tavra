/// Datetime types: four distinct kinds, offsets preserved as written (never
/// normalized to UTC), nanosecond precision, leap second (second == 60)
/// representable.

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Datetime {
    Offset(OffsetDateTime),
    Local(LocalDateTime),
    Date(Date),
    Time(Time),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Date {
    year: u16,
    month: u8,
    day: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Time {
    hour: u8,
    minute: u8,
    second: u8,
    nanosecond: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct LocalDateTime {
    pub date: Date,
    pub time: Time,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct OffsetDateTime {
    pub datetime: LocalDateTime,
    /// Offset from UTC in minutes, −1439 ‥ 1439. `Z` is 0.
    offset_minutes: i16,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DatetimeError {
    YearOutOfRange,
    MonthOutOfRange,
    DayOutOfRange,
    HourOutOfRange,
    MinuteOutOfRange,
    SecondOutOfRange,
    NanosecondOutOfRange,
    OffsetOutOfRange,
}

impl Date {
    pub fn new(year: u16, month: u8, day: u8) -> Result<Date, DatetimeError> {
        if year > 9999 {
            return Err(DatetimeError::YearOutOfRange);
        }
        if !(1..=12).contains(&month) {
            return Err(DatetimeError::MonthOutOfRange);
        }
        if day < 1 || day > days_in_month(year, month) {
            return Err(DatetimeError::DayOutOfRange);
        }
        Ok(Date { year, month, day })
    }

    pub fn year(self) -> u16 {
        self.year
    }

    pub fn month(self) -> u8 {
        self.month
    }

    pub fn day(self) -> u8 {
        self.day
    }
}

impl Time {
    pub fn new(hour: u8, minute: u8, second: u8, nanosecond: u32) -> Result<Time, DatetimeError> {
        if hour > 23 {
            return Err(DatetimeError::HourOutOfRange);
        }
        if minute > 59 {
            return Err(DatetimeError::MinuteOutOfRange);
        }
        // 60 allowed: leap second.
        if second > 60 {
            return Err(DatetimeError::SecondOutOfRange);
        }
        if nanosecond >= 1_000_000_000 {
            return Err(DatetimeError::NanosecondOutOfRange);
        }
        Ok(Time { hour, minute, second, nanosecond })
    }

    pub fn hour(self) -> u8 {
        self.hour
    }

    pub fn minute(self) -> u8 {
        self.minute
    }

    pub fn second(self) -> u8 {
        self.second
    }

    pub fn nanosecond(self) -> u32 {
        self.nanosecond
    }
}

impl OffsetDateTime {
    pub fn new(datetime: LocalDateTime, offset_minutes: i16) -> Result<OffsetDateTime, DatetimeError> {
        if !(-1439..=1439).contains(&offset_minutes) {
            return Err(DatetimeError::OffsetOutOfRange);
        }
        Ok(OffsetDateTime { datetime, offset_minutes })
    }

    pub fn offset_minutes(self) -> i16 {
        self.offset_minutes
    }
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => unreachable!("month validated before call"),
    }
}

fn is_leap_year(year: u16) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leap_year_days() {
        assert!(Date::new(2024, 2, 29).is_ok());
        assert_eq!(Date::new(2023, 2, 29), Err(DatetimeError::DayOutOfRange));
        assert_eq!(Date::new(1900, 2, 29), Err(DatetimeError::DayOutOfRange));
        assert!(Date::new(2000, 2, 29).is_ok());
    }

    #[test]
    fn leap_second_allowed() {
        assert!(Time::new(23, 59, 60, 0).is_ok());
        assert_eq!(Time::new(23, 59, 61, 0), Err(DatetimeError::SecondOutOfRange));
    }

    #[test]
    fn offset_bounds() {
        let dt = LocalDateTime {
            date: Date::new(2026, 7, 12).unwrap(),
            time: Time::new(10, 30, 0, 0).unwrap(),
        };
        assert!(OffsetDateTime::new(dt, 0).is_ok());
        assert!(OffsetDateTime::new(dt, 1439).is_ok());
        assert!(OffsetDateTime::new(dt, -1439).is_ok());
        assert_eq!(OffsetDateTime::new(dt, 1440), Err(DatetimeError::OffsetOutOfRange));
    }

    #[test]
    fn kinds_are_distinct() {
        let date = Date::new(2026, 7, 12).unwrap();
        let time = Time::new(0, 0, 0, 0).unwrap();
        let local = LocalDateTime { date, time };
        let offset = OffsetDateTime::new(local, 0).unwrap();
        assert_ne!(Datetime::Local(local), Datetime::Offset(offset));
        assert_ne!(Datetime::Date(date), Datetime::Local(local));
    }
}
