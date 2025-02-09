mod scan;
mod plan;

use crate::cli::{Args, Commands};

pub fn execute(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        Commands::Scan(args) => scan::execute(args),
        Commands::Plan(args) => plan::execute(args),
    }
}
