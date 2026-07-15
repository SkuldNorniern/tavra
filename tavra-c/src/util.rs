use std::ptr;
use std::slice;

/// Converts an owned `Vec<u8>` into a raw pointer + length the caller owns.
/// Free the result with `tav_free_bytes`.
pub(crate) fn box_slice_into_raw(data: Vec<u8>) -> (*mut u8, usize) {
    let boxed: Box<[u8]> = data.into_boxed_slice();
    let len = boxed.len();
    let ptr = Box::into_raw(boxed).cast::<u8>();
    (ptr, len)
}

/// Frees a buffer previously returned by a tavra-c function that documents
/// it as an owned buffer (`tav_format`, `tav_encode`, `tav_seal_*`, ...).
///
/// # Safety
/// `ptr`/`len` must be exactly the pointer and length returned together
/// from one such call, and must not have been freed already. Passing NULL
/// is a no-op.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tav_free_bytes(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let data = slice::from_raw_parts_mut(ptr, len);
        drop(Box::from_raw(data as *mut [u8]));
    }
}

/// Reads a caller-supplied 32-byte buffer (key/seed) into an owned array.
///
/// # Safety
/// `ptr` must be non-null and point to 32 readable bytes.
pub(crate) unsafe fn read_32(ptr: *const u8) -> [u8; 32] {
    let mut out = [0u8; 32];
    unsafe {
        ptr::copy_nonoverlapping(ptr, out.as_mut_ptr(), 32);
    }
    out
}
