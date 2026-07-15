use std::slice;

use tavra::binary;

use crate::error::set_last_error;
use crate::util::box_slice_into_raw;
use crate::value::TavValue;

/// Encodes a value as canonical `.tavb` (magic + version + value). Returns
/// 0 on success and writes an owned buffer to `*out_data`/`*out_len` (free
/// with `tav_free_bytes`).
///
/// # Safety
/// `value` must be a valid, non-null handle. `out_data`/`out_len` must be
/// valid, non-null pointers to write to.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tav_encode(value: *const TavValue, out_data: *mut *mut u8, out_len: *mut usize) -> i32 {
    if value.is_null() || out_data.is_null() || out_len.is_null() {
        set_last_error("null pointer argument");
        return 1;
    }
    let root = unsafe { &(*value).0 };
    let encoded = binary::encode(root);
    let (ptr, len) = box_slice_into_raw(encoded);
    unsafe {
        *out_data = ptr;
        *out_len = len;
    }
    0
}

/// Decodes a `.tavb` document, rejecting any non-canonical or malformed
/// input. Returns 0 on success and writes an owned handle to `*out_value`
/// (free with `tav_value_free`); returns nonzero on error (see
/// `tav_last_error`).
///
/// # Safety
/// `data` must point to `len` valid, readable bytes. `out_value` must be a
/// valid, non-null pointer to write to.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tav_decode(data: *const u8, len: usize, out_value: *mut *mut TavValue) -> i32 {
    if data.is_null() || out_value.is_null() {
        set_last_error("null pointer argument");
        return 1;
    }
    let bytes = unsafe { slice::from_raw_parts(data, len) };
    match binary::decode(bytes) {
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
