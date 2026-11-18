use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use serde::Serialize;

use super::sink::{SinkRef, visit_row, visit_table as visit};
use crate::error::FlatBufferError;

include!(concat!(env!("OUT_DIR"), "/tables.rs"));

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

pub fn guard<T>(run: impl FnOnce() -> Result<T, FlatBufferError>) -> Result<T, FlatBufferError> {
    catch_unwind(AssertUnwindSafe(run))
        .unwrap_or_else(|payload| Err(FlatBufferError::Panic(message(&*payload))))
}

fn to_json<'a, T>(bytes: &'a [u8], out: &mut Vec<u8>) -> Result<(), FlatBufferError>
where
    T: flatbuffers::Follow<'a> + flatbuffers::Verifiable + 'a,
    T::Inner: Serialize
{
    let root = flatbuffers::root::<T>(bytes)?;
    Ok(serde_json::to_writer_pretty(out, &root)?)
}

fn to_sink_table<'a, T>(bytes: &'a [u8], out: &SinkRef) -> Result<(), FlatBufferError>
where
    T: flatbuffers::Follow<'a> + flatbuffers::Verifiable + 'a,
    T::Inner: Serialize
{
    let root = flatbuffers::root::<T>(bytes)?;
    visit(&root, out)
}

fn to_sink_row<'a, T>(bytes: &'a [u8], out: &SinkRef) -> Result<(), FlatBufferError>
where
    T: flatbuffers::Follow<'a> + flatbuffers::Verifiable + 'a,
    T::Inner: Serialize
{
    let root = flatbuffers::root::<T>(bytes)?;
    visit_row(&root, out)
}

fn message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_owned())
}
