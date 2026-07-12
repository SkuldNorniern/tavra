use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

/// The canonical quiet NaN bit pattern all NaNs collapse to.
const CANONICAL_NAN_BITS: u64 = 0x7FF8_0000_0000_0000;

/// A Tavra float: IEEE 754 binary64 with a canonical NaN.
///
/// Construction normalizes every NaN payload to the single quiet NaN, so
/// bit-level comparison is well-defined: NaN == NaN, and 0.0 != -0.0.
/// The inner field stays private to keep that invariant.
#[derive(Clone, Copy, Debug)]
pub struct Float(f64);

impl Float {
    pub fn new(v: f64) -> Float {
        if v.is_nan() {
            Float(f64::from_bits(CANONICAL_NAN_BITS))
        } else {
            Float(v)
        }
    }

    pub fn get(self) -> f64 {
        self.0
    }

    pub fn to_bits(self) -> u64 {
        self.0.to_bits()
    }

    /// Builds a `Float` from raw bits, rejecting any NaN pattern other than
    /// the canonical one — used by the binary decoder, where a
    /// non-canonical NaN encoding must be a decode error rather than
    /// silently normalized.
    pub fn from_bits_checked(bits: u64) -> Option<Float> {
        let is_nan = (bits & 0x7FF0_0000_0000_0000 == 0x7FF0_0000_0000_0000) && (bits & 0x000F_FFFF_FFFF_FFFF != 0);
        if is_nan && bits != CANONICAL_NAN_BITS {
            None
        } else {
            Some(Float(f64::from_bits(bits)))
        }
    }
}

impl PartialEq for Float {
    fn eq(&self, other: &Float) -> bool {
        self.to_bits() == other.to_bits()
    }
}

impl Eq for Float {}

impl PartialOrd for Float {
    fn partial_cmp(&self, other: &Float) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Float {
    fn cmp(&self, other: &Float) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Hash for Float {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_bits().hash(state);
    }
}

impl From<f64> for Float {
    fn from(v: f64) -> Float {
        Float::new(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nan_payloads_collapse() {
        let signaling_ish = f64::from_bits(0xFFF0_0000_0000_0001);
        assert!(signaling_ish.is_nan());
        assert_eq!(Float::new(signaling_ish).to_bits(), CANONICAL_NAN_BITS);
        assert_eq!(Float::new(f64::NAN), Float::new(signaling_ish));
    }

    #[test]
    fn zero_signs_distinct() {
        assert_ne!(Float::new(0.0), Float::new(-0.0));
        assert!(Float::new(-0.0) < Float::new(0.0));
    }

    #[test]
    fn from_bits_checked_rejects_noncanonical_nan() {
        assert!(Float::from_bits_checked(CANONICAL_NAN_BITS).is_some());
        assert!(Float::from_bits_checked(0xFFF0_0000_0000_0001).is_none());
        assert!(Float::from_bits_checked(0.0f64.to_bits()).is_some());
    }

    #[test]
    fn total_order_consistent_with_eq() {
        let a = Float::new(f64::NAN);
        let b = Float::new(-f64::NAN);
        assert_eq!(a.cmp(&b), Ordering::Equal);
        assert_eq!(a, b);
    }
}
