use rand::RngExt;

/// Fills an array with cryptographically-secure random bytes, from the
/// thread-local CSPRNG (`rand::rng()`, OS-seeded).
pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    rand::rng().fill(&mut buf);
    buf
}
