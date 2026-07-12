use std::fmt::{self, Display, Formatter};

/// A Tavra integer: one logical type covering the range i64 ∪ u64
/// (−2⁶³ ‥ 2⁶⁴−1).
///
/// Values that fit in `i64` are always stored in the `Small` variant, so
/// each integer has exactly one representation and derived `Eq`/`Ord`/`Hash`
/// are canonical (`Big` only ever holds values > `i64::MAX`, which are
/// greater than every `Small`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Int(Repr);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum Repr {
    Small(i64),
    Big(u64),
}

impl Int {
    pub const MIN: Int = Int(Repr::Small(i64::MIN));
    pub const MAX: Int = Int(Repr::Big(u64::MAX));

    pub fn from_i64(v: i64) -> Int {
        Int(Repr::Small(v))
    }

    pub fn from_u64(v: u64) -> Int {
        match i64::try_from(v) {
            Ok(small) => Int(Repr::Small(small)),
            Err(_) => Int(Repr::Big(v)),
        }
    }

    /// Returns `None` when `v` is outside −2⁶³ ‥ 2⁶⁴−1.
    pub fn from_i128(v: i128) -> Option<Int> {
        if let Ok(small) = i64::try_from(v) {
            Some(Int(Repr::Small(small)))
        } else if let Ok(big) = u64::try_from(v) {
            Some(Int::from_u64(big))
        } else {
            None
        }
    }

    pub fn as_i64(self) -> Option<i64> {
        match self.0 {
            Repr::Small(v) => Some(v),
            Repr::Big(_) => None,
        }
    }

    pub fn as_u64(self) -> Option<u64> {
        match self.0 {
            Repr::Small(v) => u64::try_from(v).ok(),
            Repr::Big(v) => Some(v),
        }
    }

    pub fn as_i128(self) -> i128 {
        match self.0 {
            Repr::Small(v) => i128::from(v),
            Repr::Big(v) => i128::from(v),
        }
    }

    pub fn is_negative(self) -> bool {
        matches!(self.0, Repr::Small(v) if v < 0)
    }
}

impl From<i64> for Int {
    fn from(v: i64) -> Int {
        Int::from_i64(v)
    }
}

impl From<u64> for Int {
    fn from(v: u64) -> Int {
        Int::from_u64(v)
    }
}

impl Display for Int {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            Repr::Small(v) => write!(f, "{v}"),
            Repr::Big(v) => write!(f, "{v}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u64_within_i64_range_normalizes_to_small() {
        assert_eq!(Int::from_u64(42), Int::from_i64(42));
        assert_eq!(Int::from_u64(i64::MAX as u64), Int::from_i64(i64::MAX));
    }

    #[test]
    fn big_only_above_i64_max() {
        let v = Int::from_u64(i64::MAX as u64 + 1);
        assert_eq!(v.as_i64(), None);
        assert_eq!(v.as_u64(), Some(i64::MAX as u64 + 1));
    }

    #[test]
    fn ordering_spans_representations() {
        assert!(Int::from_i64(-1) < Int::from_i64(0));
        assert!(Int::from_i64(i64::MAX) < Int::from_u64(u64::MAX));
        assert!(Int::MIN < Int::MAX);
    }

    #[test]
    fn i128_bounds() {
        assert_eq!(Int::from_i128(i128::from(i64::MIN)), Some(Int::MIN));
        assert_eq!(Int::from_i128(i128::from(u64::MAX)), Some(Int::MAX));
        assert_eq!(Int::from_i128(i128::from(i64::MIN) - 1), None);
        assert_eq!(Int::from_i128(i128::from(u64::MAX) + 1), None);
    }
}
