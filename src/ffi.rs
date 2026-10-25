use std::ffi::{CString, c_char};

#[cfg(feature = "html")]
use crate::{ParseOptions, render_html};
#[cfg(feature = "html")]
use std::ffi::CStr;

/// Parse markdown input and return a heap-allocated HTML string.
///
/// The caller **must** free the returned pointer with [`ironmark_free`].
/// Returns a null pointer if the input is null or contains invalid UTF-8.
///
/// # Safety
///
/// `input` must be a valid, null-terminated C string.
#[cfg(feature = "html")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ironmark_render_html(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: caller guarantees `input` is a valid null-terminated C string.
    let c_str = unsafe { CStr::from_ptr(input) };
    let Ok(markdown) = c_str.to_str() else {
        return std::ptr::null_mut();
    };
    let html = render_html(markdown, &ParseOptions::default());
    match CString::new(html) {
        Ok(c) => c.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a string previously returned by [`ironmark_render_html`].
///
/// # Safety
///
/// `ptr` must be a pointer returned by `ironmark_render_html`, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ironmark_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        // SAFETY: `ptr` was produced by `CString::into_raw` in `ironmark_render_html`.
        drop(unsafe { CString::from_raw(ptr) });
    }
}
