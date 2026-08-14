use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PackError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Invalid .flat container")]
    Invalid,

    #[error("Unsupported .flat version: {0}")]
    UnsupportedVersion(u16),

    #[error("Target triple is empty, contains NUL, or is longer than 31 bytes")]
    InvalidTriple,

    #[error("Duplicate target triple: {0}")]
    DuplicateTriple(Box<str>),

    #[error("No entries to pack")]
    Empty,

    #[error("Plugin not found: {0}")]
    NotFound(PathBuf)
}
