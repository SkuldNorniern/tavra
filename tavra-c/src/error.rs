use std::cell::RefCell;
use std::ffi::CString;
use std::fmt::Display;
use std::os::raw::c_char;
use std::ptr;

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

pub(crate) fn set_last_error(message: impl Display) {
    let msg = message.to_string();
    let c_msg = CString::new(msg).unwrap_or_else(|_| c"error message contained a NUL byte".to_owned());
    LAST_ERROR.with(|cell| *cell.borrow_mut() = Some(c_msg));
}

/// Returns the last error message set on this thread, or NULL if none has
/// been set yet. The returned pointer is valid until the next tavra-c call
/// on this thread — copy the string if it needs to outlive that.
#[unsafe(no_mangle)]
pub extern "C" fn tav_last_error() -> *const c_char {
    LAST_ERROR.with(|cell| match &*cell.borrow() {
        Some(s) => s.as_ptr(),
        None => ptr::null(),
    })
}
