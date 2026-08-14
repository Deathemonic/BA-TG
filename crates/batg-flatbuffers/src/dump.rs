use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::str;

use bacy::crypto::table;
use serde::Serialize;

use crate::error::FlatBufferError;

include!(concat!(env!("OUT_DIR"), "/tables.rs"));

type Dispatch = fn(&str, &[u8], &mut Vec<u8>) -> Option<Result<(), FlatBufferError>>;

fn to_json<'a, T>(bytes: &'a [u8], out: &mut Vec<u8>) -> Result<(), FlatBufferError>
where
    T: flatbuffers::Follow<'a> + flatbuffers::Verifiable + 'a,
    T::Inner: Serialize
{
    let root = flatbuffers::root::<T>(bytes)?;
    Ok(serde_json::to_writer_pretty(out, &root)?)
}

pub fn resolve_table(filename: &str) -> Option<&'static str> {
    let stem =
        filename.rsplit_once('/').map_or(filename, |(_, name)| name).trim_end_matches(".bytes");

    TABLE_TYPES.iter().copied().find(|entry| entry.eq_ignore_ascii_case(stem))
}

pub fn resolve_row(table_name: &str) -> Option<&'static str> {
    let base = table_name.strip_suffix("DBSchema").unwrap_or(table_name).as_bytes();

    ROW_TYPES.iter().copied().find(|entry| {
        entry.as_bytes().split_at_checked(base.len()).is_some_and(|(head, tail)| {
            head.eq_ignore_ascii_case(base) && tail.eq_ignore_ascii_case(b"Excel")
        })
    })
}

fn decode(
    dispatch: Dispatch,
    type_name: &str,
    bytes: &[u8],
    out: &mut Vec<u8>
) -> Result<(), FlatBufferError> {
    let run = AssertUnwindSafe(|| {
        dispatch(type_name, bytes, out)
            .unwrap_or_else(|| Err(FlatBufferError::UnknownTable(type_name.to_owned())))
    });

    catch_unwind(run).unwrap_or_else(|payload| Err(FlatBufferError::Panic(message(&*payload))))
}

fn message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_owned())
}

fn finish(buffer: Vec<u8>) -> Result<String, FlatBufferError> {
    String::from_utf8(buffer).map_err(|error| FlatBufferError::Utf8(error.utf8_error()))
}

fn dump_one(dispatch: Dispatch, type_name: &str, bytes: &[u8]) -> Result<String, FlatBufferError> {
    let mut out = Vec::new();
    decode(dispatch, type_name, bytes, &mut out)?;
    finish(out)
}

pub fn dump_decrypted(table: &str, bytes: &[u8]) -> Result<String, FlatBufferError> {
    dump_one(dispatch_table, table, bytes)
}

pub fn dump(table: &str, bytes: &mut [u8]) -> Result<String, FlatBufferError> {
    table::xor(table, bytes);

    dump_decrypted(table, bytes)
}

pub fn dump_file(
    filename: &str,
    bytes: &mut [u8]
) -> Result<(&'static str, String), FlatBufferError> {
    let table = resolve_table(filename)
        .ok_or_else(|| FlatBufferError::UnknownTable(filename.to_owned()))?;

    let json = dump(table, bytes)?;
    Ok((table, json))
}

pub fn dump_row(row_type: &str, bytes: &[u8]) -> Result<String, FlatBufferError> {
    dump_one(dispatch_row, row_type, bytes)
}

pub fn dump_rows<'a>(
    row_type: &str,
    blobs: impl IntoIterator<Item = &'a [u8]>
) -> Result<String, FlatBufferError> {
    let mut out = String::from("[\n");
    let mut row = Vec::new();
    let mut empty = true;

    for bytes in blobs {
        row.clear();
        decode(dispatch_row, row_type, bytes, &mut row)?;

        if !empty {
            out.push_str(",\n");
        }
        indent(&mut out, str::from_utf8(&row)?);
        empty = false;
    }

    if empty {
        return Ok("[]".into());
    }

    out.push_str("\n]");
    Ok(out)
}

fn indent(out: &mut String, json: &str) {
    for (index, line) in json.lines().enumerate() {
        if index != 0 {
            out.push('\n');
        }
        out.push_str("  ");
        out.push_str(line);
    }
}
