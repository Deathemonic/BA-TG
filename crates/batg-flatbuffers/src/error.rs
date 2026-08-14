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
    Json(#[from] serde_json::Error)
}
