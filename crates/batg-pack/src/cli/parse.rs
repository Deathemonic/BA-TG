use std::path::PathBuf;

use crate::cli::args::{Args, Commands};
use crate::container;
use crate::error::PackError;

pub fn run(args: Args) -> Result<(), PackError> {
    match args.command {
        Commands::Pack(args) => {
            let mut values = args.add.into_iter();
            let mut adds = Vec::with_capacity(values.len() / 2);

            while let Some(triple) = values.next() {
                if let Some(path) = values.next() {
                    adds.push((triple, PathBuf::from(path)));
                }
            }

            container::pack(&args.output, adds)
        }
        Commands::List(args) => container::list(&args.file)
    }
}
