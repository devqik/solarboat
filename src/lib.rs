pub mod cli;
pub mod commands;
pub mod config;
pub mod utils;

use clap::Parser;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli::Args::parse();
    Ok(commands::handle_command(cli)?)
}
