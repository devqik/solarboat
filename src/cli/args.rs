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
                     Can be run in dry-run mode for validation."
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
}

#[derive(Parser)]
pub struct ApplyArgs {
    #[clap(
        long,
        default_value = "true",
        help = "Run in dry-run mode without applying changes",
        long_help = "When enabled, shows what would be applied without making actual changes."
    )]
    pub dry_run: bool,
}
