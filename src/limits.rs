//! Resource limits for decoding untrusted input.

use crate::value::MAX_DEPTH;

/// Limits for decoding. `Default` is what `text::parse`, `binary::decode`
/// and `envelope::open` use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Max array/map nesting below root. Values above [`MAX_DEPTH`] are
    /// treated as `MAX_DEPTH`, since decoding recurses.
    pub max_depth: u32,
    /// Max zstd output of a compressed envelope, in bytes.
    pub max_decompressed_len: usize,
    /// Max Argon2id memory in KiB, read from a password envelope.
    pub max_kdf_m_cost: u32,
    /// Max Argon2id passes.
    pub max_kdf_t_cost: u32,
    /// Max Argon2id lanes.
    pub max_kdf_p_cost: u8,
}

impl Limits {
    pub(crate) fn depth(&self) -> u32 {
        self.max_depth.min(MAX_DEPTH)
    }
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_depth: MAX_DEPTH,
            max_decompressed_len: 256 * 1024 * 1024,
            max_kdf_m_cost: 256 * 1024,
            max_kdf_t_cost: 10,
            max_kdf_p_cost: 4,
        }
    }
}
