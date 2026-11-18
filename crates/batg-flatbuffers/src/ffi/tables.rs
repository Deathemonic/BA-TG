use std::ffi::CString;
use std::os::raw::c_char;
use std::{ptr, slice};

use super::strings::{borrow, borrow_bytes, into_raw};
use super::types::FfiResult;
use crate::error::FlatBufferError;
use crate::runtime::sink::{self, Sink, SinkRef};
use crate::runtime::unpack::{
    unpack,
    unpack_decrypted,
    unpack_row,
    unpack_rows,
    visit_decrypted,
    visit_rows,
    visit_table
};
use crate::runtime::{resolve_row, resolve_table};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn baax_resolve_table(filename: *const c_char) -> *mut c_char {
    unsafe { resolve(filename, resolve_table) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn baax_resolve_row(table_name: *const c_char) -> *mut c_char {
    unsafe { resolve(table_name, resolve_row) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn baax_unpack_table(
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

    FfiResult::from(unpack(table, unsafe { slice::from_raw_parts_mut(bytes, len) }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn baax_unpack_table_decrypted(
    table: *const c_char,
    bytes: *const u8,
    len: usize
) -> FfiResult {
    unsafe { decode(table, bytes, len, unpack_decrypted) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn baax_unpack_row(
    row_type: *const c_char,
    bytes: *const u8,
    len: usize
) -> FfiResult {
    unsafe { decode(row_type, bytes, len, unpack_row) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn baax_unpack_rows(
    row_type: *const c_char,
    blobs: *const *const u8,
    lengths: *const usize,
    count: usize
) -> FfiResult {
    let Some(row_type) = (unsafe { borrow(row_type) }) else {
        return FfiResult::err("invalid row type");
    };
    if count == 0 {
        return FfiResult::from(unpack_rows(row_type, []));
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
        Ok(rows) => FfiResult::from(unpack_rows(row_type, rows)),
        Err(error) => FfiResult::err(error)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn baax_sink_version() -> u32 { sink::SINK_VERSION }

#[unsafe(no_mangle)]
pub extern "C" fn baax_sink_size() -> u32 { sink::size() }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn baax_visit_table(
    table: *const c_char,
    bytes: *mut u8,
    len: usize,
    sink: *const Sink
) -> FfiResult {
    let Some(table) = (unsafe { borrow(table) }) else {
        return FfiResult::err("invalid type name");
    };
    if bytes.is_null() {
        return FfiResult::err("invalid buffer");
    }

    let sink = match unsafe { borrow_sink(sink) } {
        Ok(sink) => sink,
        Err(error) => return FfiResult::err(error)
    };

    let bytes = unsafe { slice::from_raw_parts_mut(bytes, len) };
    FfiResult::from_unit(visit_table(table, bytes, &sink))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn baax_visit_table_decrypted(
    table: *const c_char,
    bytes: *const u8,
    len: usize,
    sink: *const Sink
) -> FfiResult {
    let Some(table) = (unsafe { borrow(table) }) else {
        return FfiResult::err("invalid type name");
    };
    let Some(bytes) = (unsafe { borrow_bytes(bytes, len) }) else {
        return FfiResult::err("invalid buffer");
    };

    let sink = match unsafe { borrow_sink(sink) } {
        Ok(sink) => sink,
        Err(error) => return FfiResult::err(error)
    };

    FfiResult::from_unit(visit_decrypted(table, bytes, &sink))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn baax_visit_rows(
    row_type: *const c_char,
    blobs: *const *const u8,
    lengths: *const usize,
    count: usize,
    sink: *const Sink
) -> FfiResult {
    let Some(row_type) = (unsafe { borrow(row_type) }) else {
        return FfiResult::err("invalid row type");
    };

    let sink = match unsafe { borrow_sink(sink) } {
        Ok(sink) => sink,
        Err(error) => return FfiResult::err(error)
    };

    if count == 0 {
        return FfiResult::from_unit(visit_rows(row_type, [], &sink));
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
        Ok(rows) => FfiResult::from_unit(visit_rows(row_type, rows, &sink)),
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

unsafe fn borrow_sink(sink: *const Sink) -> Result<SinkRef, FlatBufferError> {
    unsafe { SinkRef::new(sink) }
}
