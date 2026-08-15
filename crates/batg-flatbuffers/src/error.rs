use std::fmt::Display;
use std::os::raw::c_int;
use std::str::Utf8Error;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FlatBufferError {
    #[error("Unknown flatbuffer table: {0}")]
    UnknownTable(String),

    #[error("Panicked while decoding: {0}")]
    Panic(String),

    #[error(transparent)]
    InvalidFlatbuffer(#[from] flatbuffers::InvalidFlatbuffer),

    #[error(transparent)]
    Utf8(#[from] Utf8Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("Unsupported sink version: {0}")]
    SinkVersion(u32),

    #[error("Unexpected sink size: {0}")]
    SinkSize(u32),

    #[error("Sink is missing the {0} callback")]
    SinkHook(&'static str),

    #[error("Sink rejected {0} with code {1}")]
    SinkFailed(&'static str, c_int),

    #[error("Unsupported flatbuffer value: {0}")]
    Unsupported(&'static str),

    #[error("Sink error: {0}")]
    Sink(String)
}

impl serde::ser::Error for FlatBufferError {
    fn custom<T: Display>(message: T) -> Self { Self::Sink(message.to_string()) }
}
