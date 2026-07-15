use std::{ptr, slice};

use tavra::envelope::{self, OpenMode, SealMode, SealOptions};

use crate::error::set_last_error;
use crate::util::{box_slice_into_raw, read_32};
use crate::value::TavValue;

unsafe fn optional_key(ptr: *const u8) -> Option<[u8; 32]> {
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { read_32(ptr) })
    }
}

fn do_seal(root: &tavra::Map, mode: SealMode<'_>, compress: bool, sign_key: Option<[u8; 32]>, out_data: *mut *mut u8, out_len: *mut usize) -> i32 {
    let opts = SealOptions { mode, compress, sign_with: sign_key.as_ref() };
    let sealed = envelope::seal(root, &opts);
    let (ptr, len) = box_slice_into_raw(sealed);
    unsafe {
        *out_data = ptr;
        *out_len = len;
    }
    0
}

/// Seals a value unencrypted (optionally compressed and/or signed).
///
/// # Safety
/// `value` must be a valid, non-null handle. `sign_key`, if non-null, must
/// point to 32 readable bytes (an Ed25519 signing seed). `out_data`/
/// `out_len` must be valid, non-null pointers to write to.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tav_seal_none(value: *const TavValue, compress: bool, sign_key: *const u8, out_data: *mut *mut u8, out_len: *mut usize) -> i32 {
    if value.is_null() || out_data.is_null() || out_len.is_null() {
        set_last_error("null pointer argument");
        return 1;
    }
    let root = unsafe { &(*value).0 };
    let sign_key = unsafe { optional_key(sign_key) };
    do_seal(root, SealMode::None, compress, sign_key, out_data, out_len)
}

/// Seals a value with a raw 32-byte XChaCha20-Poly1305 key.
///
/// # Safety
/// `value` and `key` must be valid, non-null (`key` pointing to 32
/// readable bytes). `sign_key`, if non-null, must point to 32 readable
/// bytes. `out_data`/`out_len` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tav_seal_key(value: *const TavValue, key: *const u8, compress: bool, sign_key: *const u8, out_data: *mut *mut u8, out_len: *mut usize) -> i32 {
    if value.is_null() || key.is_null() || out_data.is_null() || out_len.is_null() {
        set_last_error("null pointer argument");
        return 1;
    }
    let root = unsafe { &(*value).0 };
    let key = unsafe { read_32(key) };
    let sign_key = unsafe { optional_key(sign_key) };
    do_seal(root, SealMode::Key(&key), compress, sign_key, out_data, out_len)
}

/// Seals a value with an Argon2id-derived key from a password.
///
/// # Safety
/// `value` must be valid and non-null. `password` must point to
/// `password_len` readable bytes. `sign_key`, if non-null, must point to
/// 32 readable bytes. `out_data`/`out_len` must be valid, non-null
/// pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tav_seal_password(
    value: *const TavValue,
    password: *const u8,
    password_len: usize,
    compress: bool,
    sign_key: *const u8,
    out_data: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if value.is_null() || password.is_null() || out_data.is_null() || out_len.is_null() {
        set_last_error("null pointer argument");
        return 1;
    }
    let root = unsafe { &(*value).0 };
    let password_bytes = unsafe { slice::from_raw_parts(password, password_len) };
    let sign_key = unsafe { optional_key(sign_key) };
    do_seal(root, SealMode::Password(password_bytes), compress, sign_key, out_data, out_len)
}

fn do_open(bytes: &[u8], mode: OpenMode<'_>, verify_key: Option<[u8; 32]>, out_value: *mut *mut TavValue) -> i32 {
    match envelope::open(bytes, &mode, verify_key.as_ref()) {
        Ok(root) => {
            let handle = Box::into_raw(Box::new(TavValue(root)));
            unsafe {
                *out_value = handle;
            }
            0
        }
        Err(e) => {
            set_last_error(e);
            1
        }
    }
}

/// Opens an unencrypted `.tave` document. `verify_key`, if non-null,
/// checks a present signature against that Ed25519 public key.
///
/// # Safety
/// `data` must point to `len` readable bytes. `verify_key`, if non-null,
/// must point to 32 readable bytes. `out_value` must be a valid, non-null
/// pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tav_open_none(data: *const u8, len: usize, verify_key: *const u8, out_value: *mut *mut TavValue) -> i32 {
    if data.is_null() || out_value.is_null() {
        set_last_error("null pointer argument");
        return 1;
    }
    let bytes = unsafe { slice::from_raw_parts(data, len) };
    let verify_key = unsafe { optional_key(verify_key) };
    do_open(bytes, OpenMode::None, verify_key, out_value)
}

/// Opens a `.tave` document encrypted with a raw 32-byte key.
///
/// # Safety
/// `data`/`key` must be valid and non-null (`key` pointing to 32 readable
/// bytes). `verify_key`, if non-null, must point to 32 readable bytes.
/// `out_value` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tav_open_key(data: *const u8, len: usize, key: *const u8, verify_key: *const u8, out_value: *mut *mut TavValue) -> i32 {
    if data.is_null() || key.is_null() || out_value.is_null() {
        set_last_error("null pointer argument");
        return 1;
    }
    let bytes = unsafe { slice::from_raw_parts(data, len) };
    let key = unsafe { read_32(key) };
    let verify_key = unsafe { optional_key(verify_key) };
    do_open(bytes, OpenMode::Key(&key), verify_key, out_value)
}

/// Opens a `.tave` document encrypted with an Argon2id-derived password key.
///
/// # Safety
/// `data` and `password` must point to `len`/`password_len` readable
/// bytes respectively. `verify_key`, if non-null, must point to 32
/// readable bytes. `out_value` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tav_open_password(
    data: *const u8,
    len: usize,
    password: *const u8,
    password_len: usize,
    verify_key: *const u8,
    out_value: *mut *mut TavValue,
) -> i32 {
    if data.is_null() || password.is_null() || out_value.is_null() {
        set_last_error("null pointer argument");
        return 1;
    }
    let bytes = unsafe { slice::from_raw_parts(data, len) };
    let password_bytes = unsafe { slice::from_raw_parts(password, password_len) };
    let verify_key = unsafe { optional_key(verify_key) };
    do_open(bytes, OpenMode::Password(password_bytes), verify_key, out_value)
}

/// Generates a fresh 32-byte XChaCha20-Poly1305 key into `out` (must point
/// to 32 writable bytes).
///
/// # Safety
/// `out` must be non-null and point to 32 writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tav_genkey(out: *mut u8) -> i32 {
    if out.is_null() {
        set_last_error("null pointer argument");
        return 1;
    }
    let key = envelope::generate_key();
    unsafe {
        ptr::copy_nonoverlapping(key.as_ptr(), out, 32);
    }
    0
}

/// Generates a fresh Ed25519 signing key seed and its matching public key
/// into `out_secret`/`out_public` (each must point to 32 writable bytes).
///
/// # Safety
/// `out_secret`/`out_public` must be non-null and point to 32 writable
/// bytes each.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tav_gensignkey(out_secret: *mut u8, out_public: *mut u8) -> i32 {
    if out_secret.is_null() || out_public.is_null() {
        set_last_error("null pointer argument");
        return 1;
    }
    let (secret, public) = envelope::generate_signing_key();
    unsafe {
        ptr::copy_nonoverlapping(secret.as_ptr(), out_secret, 32);
        ptr::copy_nonoverlapping(public.as_ptr(), out_public, 32);
    }
    0
}
