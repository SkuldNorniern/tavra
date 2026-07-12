use super::error::Error;

pub const MAGIC: &[u8; 4] = b"TAVE";
pub const VERSION: u8 = 0x01;

/// v1 reserves bits 4-7 (bit 4 is `STREAMED`, for future chunked framing;
/// bits 5-7 are unused) — both must be zero.
const RESERVED_MASK: u8 = 0b1111_0000;

const BIT_ENCRYPTED: u8 = 0b0000_0001;
const BIT_SIGNED: u8 = 0b0000_0010;
const BIT_COMPRESSED: u8 = 0b0000_0100;
const BIT_PASSWORD: u8 = 0b0000_1000;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Flags {
    pub encrypted: bool,
    pub signed: bool,
    pub compressed: bool,
    pub password: bool,
}

impl Flags {
    pub fn to_byte(self) -> u8 {
        let mut b = 0u8;
        if self.encrypted {
            b |= BIT_ENCRYPTED;
        }
        if self.signed {
            b |= BIT_SIGNED;
        }
        if self.compressed {
            b |= BIT_COMPRESSED;
        }
        if self.password {
            b |= BIT_PASSWORD;
        }
        b
    }

    pub fn from_byte(b: u8) -> Result<Flags, Error> {
        if b & RESERVED_MASK != 0 {
            return Err(Error::new("reserved flag bits must be zero"));
        }
        let flags = Flags {
            encrypted: b & BIT_ENCRYPTED != 0,
            signed: b & BIT_SIGNED != 0,
            compressed: b & BIT_COMPRESSED != 0,
            password: b & BIT_PASSWORD != 0,
        };
        if flags.password && !flags.encrypted {
            return Err(Error::new("PASSWORD flag requires ENCRYPTED"));
        }
        Ok(flags)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct KdfParams {
    pub salt: [u8; 16],
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u8,
}

impl KdfParams {
    pub const ENCODED_LEN: usize = 16 + 4 + 4 + 1;

    pub fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.salt);
        out.extend_from_slice(&self.m_cost.to_be_bytes());
        out.extend_from_slice(&self.t_cost.to_be_bytes());
        out.push(self.p_cost);
    }

    pub fn decode(bytes: &[u8], pos: &mut usize) -> Result<KdfParams, Error> {
        let raw = read_bytes(bytes, pos, Self::ENCODED_LEN)?;
        let mut salt = [0u8; 16];
        let mut m_cost_bytes = [0u8; 4];
        let mut t_cost_bytes = [0u8; 4];
        salt.copy_from_slice(&raw[0..16]);
        m_cost_bytes.copy_from_slice(&raw[16..20]);
        t_cost_bytes.copy_from_slice(&raw[20..24]);
        Ok(KdfParams {
            salt,
            m_cost: u32::from_be_bytes(m_cost_bytes),
            t_cost: u32::from_be_bytes(t_cost_bytes),
            p_cost: raw[24],
        })
    }
}

/// Builds the AEAD associated data: every header byte that isn't the
/// nonce, signature, or payload — `magic || version || flags || kdf_params`.
pub fn build_aad(flags: Flags, kdf: Option<&KdfParams>) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(flags.to_byte());
    if let Some(k) = kdf {
        k.encode(&mut out);
    }
    out
}

pub fn read_u8(bytes: &[u8], pos: &mut usize) -> Result<u8, Error> {
    let b = *bytes.get(*pos).ok_or_else(|| Error::new("unexpected end of input"))?;
    *pos += 1;
    Ok(b)
}

pub fn read_bytes<'a>(bytes: &'a [u8], pos: &mut usize, n: usize) -> Result<&'a [u8], Error> {
    let end = pos.checked_add(n).ok_or_else(|| Error::new("length overflow"))?;
    let slice = bytes.get(*pos..end).ok_or_else(|| Error::new("unexpected end of input"))?;
    *pos = end;
    Ok(slice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_roundtrip() {
        let f = Flags { encrypted: true, signed: false, compressed: true, password: false };
        assert_eq!(Flags::from_byte(f.to_byte()).unwrap(), f);
    }

    #[test]
    fn rejects_reserved_bits() {
        assert!(Flags::from_byte(0b0001_0000).is_err());
        assert!(Flags::from_byte(0b1000_0000).is_err());
    }

    #[test]
    fn rejects_password_without_encrypted() {
        assert!(Flags::from_byte(BIT_PASSWORD).is_err());
        assert!(Flags::from_byte(BIT_PASSWORD | BIT_ENCRYPTED).is_ok());
    }

    #[test]
    fn kdf_params_roundtrip() {
        let params = KdfParams { salt: [7u8; 16], m_cost: 19456, t_cost: 2, p_cost: 1 };
        let mut buf = Vec::new();
        params.encode(&mut buf);
        let mut pos = 0;
        assert_eq!(KdfParams::decode(&buf, &mut pos).unwrap(), params);
        assert_eq!(pos, buf.len());
    }
}
