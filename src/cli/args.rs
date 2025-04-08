use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    author, 
    version, 
    about = "A CLI tool for intelligent Terraform operations management",
    long_about = "Solar Boat is a command-line interface tool designed for Infrastructure as Code (IaC) \
                  and GitOps workflows. It provides intelligent Terraform operations management with \
                  automatic dependency detection and stateful/stateless module handling."
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(
        about = "Scan for changed Terraform modules",
        long_about = "Scans the repository for changed Terraform modules and their dependencies. \
                     This command analyzes git changes and builds a dependency graph to identify \
                     affected modules without generating plans."
    )]
    Scan(ScanArgs),

    #[command(
        about = "Generate Terraform plans for changed modules",
        long_about = "Generates Terraform plans for changed modules and their dependencies. \
                     Plans are saved to the specified output directory for review. This command \
                     handles both stateful and stateless modules appropriately."
    )]
    Plan(PlanArgs),

    #[command(
        about = "Apply Terraform changes",
        long_about = "Applies Terraform changes for previously planned modules. \
                     Runs in dry-run mode by default for safety. Use --dry-run=false to apply actual changes."
    )]
    Apply(ApplyArgs),
}

#[derive(Parser)]
pub struct ScanArgs {
    #[clap(
        long,
        default_value = ".",
        help = "Root directory to scan for Terraform modules",
        long_help = "The root directory where the scan will start looking for Terraform modules. \
                    The scan will recursively search for .tf files in this directory and its subdirectories."
    )]
    pub path: String,

    #[clap(
        long,
        help = "Process all stateful modules regardless of changes",
        long_help = "When enabled, this flag will process all stateful modules \
                    in the specified directory, regardless of whether they have been changed."
    )]
    pub all: bool,
}

#[derive(Parser)]
pub struct PlanArgs {
    #[clap(
        long,
        default_value = ".",
        help = "Root directory containing Terraform modules",
        long_help = "The root directory containing Terraform modules to be planned. \
                    The command will recursively search for changed modules in this directory."
    )]
    pub path: String,
    
    #[clap(
        long,
        default_value = "terraform-plans",
        help = "Directory to save generated plan files",
        long_help = "The directory where Terraform plan files will be saved. \
                    Each module's plan will be saved as a separate file in this directory. \
                    The directory will be created if it doesn't exist."
    )]
    pub output_dir: Option<String>,

    #[clap(
        long,
        value_delimiter = ',',
        help = "Comma-separated list of workspace names to ignore",
        long_help = "Specify workspace names to skip during plan operation. \
                    Multiple workspaces can be provided as comma-separated values. \
                    Example: --ignore-workspaces dev,staging"
    )]
    pub ignore_workspaces: Option<Vec<String>>,

    #[clap(
        long,
        help = "Process all stateful modules regardless of changes",
        long_help = "When enabled, this flag will process all stateful modules \
                    in the specified directory, regardless of whether they have been changed."
    )]
    pub all: bool,
}

#[derive(Parser)]
pub struct ApplyArgs {
    #[clap(
        long,
        default_value = ".",
        help = "Root directory containing Terraform modules",
        long_help = "The root directory containing Terraform modules to be applied. \
                    The command will recursively search for changed modules in this directory."
    )]
    pub path: String,

    #[clap(
        long,
        default_value = "true",
        help = "Run in dry-run mode (no changes will be applied)",
        long_help = "When enabled (default), this flag will run the apply command in dry-run mode, \
                    showing what changes would be made without actually applying them."
    )]
    pub dry_run: bool,

    #[clap(
        long,
        value_delimiter = ',',
        help = "Comma-separated list of workspace names to ignore",
        long_help = "Specify workspace names to skip during apply operation. \
                    Multiple workspaces can be provided as comma-separated values. \
                    Example: --ignore-workspaces dev,staging"
    )]
    pub ignore_workspaces: Option<Vec<String>>,

    #[clap(
        long,
        help = "Process all stateful modules regardless of changes",
        long_help = "When enabled, this flag will process all stateful modules \
                    in the specified directory, regardless of whether they have been changed."
    )]
    pub all: bool,
}
