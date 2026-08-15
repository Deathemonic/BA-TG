use std::env::VarError;
use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BuildError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Env(#[from] VarError),

    #[error(transparent)]
    Schema(#[from] cunnybuffers::FlatBufError),

    #[error(transparent)]
    Syn(#[from] syn::Error)
}
