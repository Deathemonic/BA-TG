mod error;
mod format;
mod list;
mod pack;

use std::path::PathBuf;
use std::process::ExitCode;

use baad_utils::config::{LoggingConfig, init_logging};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "batg-pack")]
#[command(about = "Pack platform cdylibs into a .flat container")]
struct Args {
    #[command(subcommand)]
    command: Command
}

#[derive(Subcommand)]
enum Command {
    Pack(PackArgs),
    List(ListArgs)
}

#[derive(Parser)]
struct PackArgs {
    #[arg(short, long)]
    output: PathBuf,

    #[arg(long, num_args = 2, value_names = ["TRIPLE", "PATH"], required = true)]
    add: Vec<String>
}

#[derive(Parser)]
struct ListArgs {
    file: PathBuf
}

fn main() -> ExitCode {
    if let Err(error) = init_logging(LoggingConfig::default()) {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }

    let result = match Args::parse().command {
        Command::Pack(args) => {
            let mut values = args.add.into_iter();
            let mut adds = Vec::with_capacity(values.len() / 2);
            while let Some(triple) = values.next() {
                if let Some(path) = values.next() {
                    adds.push((triple, PathBuf::from(path)));
                }
            }

            pack::pack(&args.output, adds)
        }
        Command::List(args) => list::list(&args.file)
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
