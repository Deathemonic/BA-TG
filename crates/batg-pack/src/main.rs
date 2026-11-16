mod cli {
    pub mod args;
    pub mod parse;
}

mod container;
mod error;

use std::process::ExitCode;

use baad_utils::config::{LoggingConfig, init_logging};
use clap::Parser;

use crate::cli::args::Args;
use crate::cli::parse;

fn main() -> ExitCode {
    if let Err(error) = init_logging(LoggingConfig::default()) {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }

    match parse::run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
