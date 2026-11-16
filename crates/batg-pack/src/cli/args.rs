use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Commands {
    Pack(PackArgs),
    List(ListArgs)
}

#[derive(Parser)]
#[command(name = "batg-pack")]
#[command(about = "Pack platform cdylibs into a .flat container")]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands
}

#[derive(Parser)]
pub struct PackArgs {
    #[arg(short, long)]
    pub output: PathBuf,

    #[arg(long, num_args = 2, value_names = ["TRIPLE", "PATH"], required = true)]
    pub add: Vec<String>
}

#[derive(Parser)]
pub struct ListArgs {
    pub file: PathBuf
}
