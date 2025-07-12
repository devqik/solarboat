mod scan;
mod plan;
mod apply;

use crate::cli::{Args, Commands};
use crate::config::Settings;
use anyhow::Result;
use std::path::PathBuf;

pub fn handle_command(args: Args) -> Result<()> {
    // Load configuration based on CLI arguments
    let settings = if args.no_config {
        // Use default settings when config is disabled
        Settings {
            config_resolver: crate::config::ConfigResolver::new(None, PathBuf::from(".")),
        }
    } else if let Some(config_path) = &args.config {
        // Load from specified config file
        Settings::load(config_path)?
    } else {
        // Auto-discover config file from current directory
        Settings::load_from_current_dir()?
    };

    match args.command {
        Commands::Scan(scan_args) => scan::execute(scan_args, &settings),
        Commands::Plan(plan_args) => plan::execute(plan_args, &settings),
        Commands::Apply(apply_args) => apply::execute(apply_args, &settings),
    }
}
