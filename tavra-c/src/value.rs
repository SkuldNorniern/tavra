use tavra::Map;

/// Opaque handle to a parsed/decoded Tavra document root.
pub struct TavValue(pub(crate) Map);

/// Frees a value handle returned by `tav_parse`, `tav_decode`, or
/// `tav_open_*`.
///
/// # Safety
/// `ptr` must be a handle previously returned by tavra-c and not already
/// freed. Passing NULL is a no-op.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tav_value_free(ptr: *mut TavValue) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(ptr));
    }
}
