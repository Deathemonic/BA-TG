use std::ffi::{CStr, CString};
use std::fmt::Display;
use std::os::raw::{c_char, c_int};
use std::{ptr, slice};

use crate::dump::{dump, dump_decrypted, dump_row, dump_rows, resolve_row, resolve_table};
use crate::error::FlatBufferError;

#[repr(C)]
pub struct FfiResult {
    pub json: *mut c_char,
    pub error: *mut c_char,
    pub success: c_int
}

impl FfiResult {
    fn ok(json: String) -> Self {
        match CString::new(json) {
            Ok(json) => Self {
                json: json.into_raw(),
                error: ptr::null_mut(),
                success: 1
            },
            Err(error) => Self::err(error)
        }
    }

    fn err(error: impl Display) -> Self {
        Self {
            json: ptr::null_mut(),
            error: into_raw(&error.to_string()),
            success: 0
        }
    }

    fn from(result: Result<String, FlatBufferError>) -> Self {
        result.map_or_else(Self::err, Self::ok)
    }
}

fn into_raw(value: &str) -> *mut c_char {
    CString::new(value).map_or_else(|_| ptr::null_mut(), CString::into_raw)
}

unsafe fn borrow<'a>(value: *const c_char) -> Option<&'a str> {
    (!value.is_null()).then(|| unsafe { CStr::from_ptr(value) }.to_str().ok())?
}

unsafe fn borrow_bytes<'a>(bytes: *const u8, len: usize) -> Option<&'a [u8]> {
    (!bytes.is_null()).then(|| unsafe { slice::from_raw_parts(bytes, len) })
}

unsafe fn resolve(value: *const c_char, resolver: fn(&str) -> Option<&'static str>) -> *mut c_char {
    unsafe { borrow(value) }.and_then(resolver).map_or_else(ptr::null_mut, into_raw)
}

unsafe fn decode(
    type_name: *const c_char,
    bytes: *const u8,
    len: usize,
    decoder: fn(&str, &[u8]) -> Result<String, FlatBufferError>
) -> FfiResult {
    let Some(type_name) = (unsafe { borrow(type_name) }) else {
        return FfiResult::err("invalid type name");
    };
    let Some(bytes) = (unsafe { borrow_bytes(bytes, len) }) else {
        return FfiResult::err("invalid buffer");
    };

    FfiResult::from(decoder(type_name, bytes))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn baax_resolve_table(filename: *const c_char) -> *mut c_char {
    unsafe { resolve(filename, resolve_table) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn baax_resolve_row(table_name: *const c_char) -> *mut c_char {
    unsafe { resolve(table_name, resolve_row) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn baax_dump_table(
    table: *const c_char,
    bytes: *mut u8,
    len: usize
) -> FfiResult {
    let Some(table) = (unsafe { borrow(table) }) else {
        return FfiResult::err("invalid type name");
    };
    if bytes.is_null() {
        return FfiResult::err("invalid buffer");
    }

    FfiResult::from(dump(table, unsafe { slice::from_raw_parts_mut(bytes, len) }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn baax_dump_table_decrypted(
    table: *const c_char,
    bytes: *const u8,
    len: usize
) -> FfiResult {
    unsafe { decode(table, bytes, len, dump_decrypted) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn baax_dump_row(
    row_type: *const c_char,
    bytes: *const u8,
    len: usize
) -> FfiResult {
    unsafe { decode(row_type, bytes, len, dump_row) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn baax_dump_rows(
    row_type: *const c_char,
    blobs: *const *const u8,
    lengths: *const usize,
    count: usize
) -> FfiResult {
    let Some(row_type) = (unsafe { borrow(row_type) }) else {
        return FfiResult::err("invalid row type");
    };
    if count == 0 {
        return FfiResult::from(dump_rows(row_type, []));
    }
    if blobs.is_null() || lengths.is_null() {
        return FfiResult::err("invalid buffers");
    }

    let blobs = unsafe { slice::from_raw_parts(blobs, count) };
    let lengths = unsafe { slice::from_raw_parts(lengths, count) };
    let rows = blobs
        .iter()
        .zip(lengths)
        .map(|(&blob, &len)| unsafe { borrow_bytes(blob, len) }.ok_or("invalid buffer"))
        .collect::<Result<Vec<_>, _>>();

    match rows {
        Ok(rows) => FfiResult::from(dump_rows(row_type, rows)),
        Err(error) => FfiResult::err(error)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn baax_free_string(value: *mut c_char) {
    if !value.is_null() {
        drop(unsafe { CString::from_raw(value) });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn baax_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}
