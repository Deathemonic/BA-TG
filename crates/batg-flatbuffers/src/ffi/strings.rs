use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::{ptr, slice};

pub fn into_raw(value: &str) -> *mut c_char {
    CString::new(value).map_or(ptr::null_mut(), CString::into_raw)
}

pub unsafe fn borrow<'a>(value: *const c_char) -> Option<&'a str> {
    (!value.is_null()).then(|| unsafe { CStr::from_ptr(value) }.to_str().ok())?
}

pub unsafe fn borrow_bytes<'a>(bytes: *const u8, len: usize) -> Option<&'a [u8]> {
    (!bytes.is_null() && len <= isize::MAX as usize)
        .then(|| unsafe { slice::from_raw_parts(bytes, len) })
}
