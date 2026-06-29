mod cli;
mod commands {
    pub mod commit;
}

use crate::cli::{Cli, Command};
use clap::Parser;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Commit => commands::commit::run(),
    }

    println!("End");
}
