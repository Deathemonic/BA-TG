use std::str;

use bacy::crypto::table;

use super::sink::SinkRef;
use super::tables::{
    guard,
    resolve_table,
    unpack_row_type,
    unpack_table_type,
    visit_row_type,
    visit_table_type
};
use crate::error::FlatBufferError;

type Dispatch<O> = fn(&str, &[u8], O) -> Option<Result<(), FlatBufferError>>;
type Json = fn(&str, &[u8], &mut Vec<u8>) -> Option<Result<(), FlatBufferError>>;

pub fn unpack_decrypted(table: &str, bytes: &[u8]) -> Result<String, FlatBufferError> {
    unpack_one(unpack_table_type, table, bytes)
}

pub fn unpack(table: &str, bytes: &mut [u8]) -> Result<String, FlatBufferError> {
    table::xor(table, bytes);

    unpack_decrypted(table, bytes)
}

pub fn unpack_file(
    filename: &str,
    bytes: &mut [u8]
) -> Result<(&'static str, String), FlatBufferError> {
    let table = resolve_table(filename)
        .ok_or_else(|| FlatBufferError::UnknownTable(filename.to_owned()))?;

    let json = unpack(table, bytes)?;
    Ok((table, json))
}

pub fn unpack_row(row_type: &str, bytes: &[u8]) -> Result<String, FlatBufferError> {
    unpack_one(unpack_row_type, row_type, bytes)
}

pub fn unpack_rows<'a>(
    row_type: &str,
    blobs: impl IntoIterator<Item = &'a [u8]>
) -> Result<String, FlatBufferError> {
    guard(|| {
        let mut out = String::from("[\n");
        let mut row = Vec::new();
        let mut empty = true;

        for bytes in blobs {
            row.clear();
            decode(unpack_row_type, row_type, bytes, &mut row)?;

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
    })
}

pub fn visit_decrypted(table: &str, bytes: &[u8], out: &SinkRef) -> Result<(), FlatBufferError> {
    guard(|| decode(visit_table_type, table, bytes, out))
}

pub fn visit_table(table: &str, bytes: &mut [u8], out: &SinkRef) -> Result<(), FlatBufferError> {
    table::xor(table, bytes);

    visit_decrypted(table, bytes, out)
}

pub fn visit_rows<'a>(
    row_type: &str,
    blobs: impl IntoIterator<Item = &'a [u8]>,
    out: &SinkRef
) -> Result<(), FlatBufferError> {
    guard(|| {
        for bytes in blobs {
            decode(visit_row_type, row_type, bytes, out)?;
        }

        Ok(())
    })
}

fn decode<O>(
    dispatch: Dispatch<O>,
    type_name: &str,
    bytes: &[u8],
    out: O
) -> Result<(), FlatBufferError> {
    dispatch(type_name, bytes, out)
        .unwrap_or_else(|| Err(FlatBufferError::UnknownTable(type_name.to_owned())))
}

fn finish(buffer: Vec<u8>) -> Result<String, FlatBufferError> {
    String::from_utf8(buffer).map_err(|error| FlatBufferError::Utf8(error.utf8_error()))
}

fn unpack_one(dispatch: Json, type_name: &str, bytes: &[u8]) -> Result<String, FlatBufferError> {
    guard(|| {
        let mut out = Vec::new();
        decode(dispatch, type_name, bytes, &mut out)?;

        finish(out)
    })
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
