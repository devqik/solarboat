mod scan;
mod plan;
mod apply;

use crate::cli::{Args, Commands};

pub fn handle_command(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        Commands::Scan(args) => scan::execute(args),
        Commands::Plan(args) => plan::execute(args),
        Commands::Apply(args) => apply::execute(args),
    }
}
