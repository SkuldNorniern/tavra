use argon2::{Algorithm, Argon2, Params, Version};

use crate::envelope::aead::KEY_LEN;
use crate::envelope::error::Error;
use crate::envelope::header::KdfParams;
use crate::envelope::random::random_bytes;

pub const SALT_LEN: usize = 16;

/// OWASP's current baseline Argon2id parameters at time of writing: 19 MiB
/// memory, 2 iterations, 1 degree of parallelism.
pub const DEFAULT_M_COST: u32 = 19456;
pub const DEFAULT_T_COST: u32 = 2;
pub const DEFAULT_P_COST: u8 = 1;

/// Limits on Argon2id params read from header. KDF runs before AEAD can
/// reject anything.
pub const MAX_M_COST: u32 = 256 * 1024; // 256 MiB
pub const MAX_T_COST: u32 = 10;
pub const MAX_P_COST: u8 = 4;

const _: () = assert!(DEFAULT_M_COST <= MAX_M_COST && DEFAULT_T_COST <= MAX_T_COST && DEFAULT_P_COST <= MAX_P_COST);

pub fn random_params() -> KdfParams {
    KdfParams { salt: random_bytes::<SALT_LEN>(), m_cost: DEFAULT_M_COST, t_cost: DEFAULT_T_COST, p_cost: DEFAULT_P_COST }
}

pub fn derive_key(password: &[u8], params: &KdfParams) -> Result<[u8; KEY_LEN], Error> {
    if params.m_cost > MAX_M_COST || params.t_cost > MAX_T_COST || params.p_cost > MAX_P_COST {
        return Err(Error::new(format!(
            "Argon2id parameters exceed limits (m_cost {} <= {MAX_M_COST}, t_cost {} <= {MAX_T_COST}, p_cost {} <= {MAX_P_COST})",
            params.m_cost, params.t_cost, params.p_cost
        )));
    }
    let argon2_params = Params::new(params.m_cost, params.t_cost, u32::from(params.p_cost), Some(KEY_LEN))
        .map_err(|e| Error::new(format!("invalid Argon2id parameters: {e}")))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params);
    let mut key = [0u8; KEY_LEN];
    argon2
        .hash_password_into(password, &params.salt, &mut key)
        .map_err(|e| Error::new(format!("Argon2id key derivation failed: {e}")))?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_params(salt: [u8; SALT_LEN]) -> KdfParams {
        // Cheap parameters so tests run fast; production defaults are
        // `DEFAULT_M_COST`/`DEFAULT_T_COST`/`DEFAULT_P_COST`.
        KdfParams { salt, m_cost: 8, t_cost: 1, p_cost: 1 }
    }

    #[test]
    fn same_password_and_salt_same_key() {
        let params = test_params([9u8; SALT_LEN]);
        let a = derive_key(b"correct horse", &params).unwrap();
        let b = derive_key(b"correct horse", &params).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn different_password_different_key() {
        let params = test_params([9u8; SALT_LEN]);
        let a = derive_key(b"password one", &params).unwrap();
        let b = derive_key(b"password two", &params).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn different_salt_different_key() {
        let a = derive_key(b"same password", &test_params([1u8; SALT_LEN])).unwrap();
        let b = derive_key(b"same password", &test_params([2u8; SALT_LEN])).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn rejects_costs_over_limits() {
        let over = [
            KdfParams { m_cost: MAX_M_COST + 1, ..test_params([0u8; SALT_LEN]) },
            KdfParams { t_cost: MAX_T_COST + 1, ..test_params([0u8; SALT_LEN]) },
            KdfParams { p_cost: MAX_P_COST + 1, ..test_params([0u8; SALT_LEN]) },
            KdfParams { m_cost: u32::MAX, t_cost: u32::MAX, p_cost: u8::MAX, salt: [0u8; SALT_LEN] },
        ];
        for params in &over {
            assert!(derive_key(b"pw", params).is_err());
        }
    }

    #[test]
    fn random_params_produce_distinct_salts() {
        assert_ne!(random_params().salt, random_params().salt);
    }
}
