use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    author, 
    version, 
    about = "A CLI tool for intelligent Terraform operations management",
    long_about = "Solarboat is a command-line interface tool designed for Infrastructure as Code (IaC) \
                  and GitOps workflows. It provides intelligent Terraform operations management with \
                  automatic dependency detection and stateful/stateless module handling."
)]
pub struct Args {
    #[clap(
        long,
        help = "Path to configuration file (solarboat.json)",
        long_help = "Specify a custom path to the configuration file. \
                    If not provided, the tool will search for configuration files \
                    in the current directory and parent directories."
    )]
    pub config: Option<String>,

    #[clap(
        long,
        num_args = 0..=1,
        value_name = "BOOL",
        help = "Disable configuration file loading",
        long_help = "When enabled, this flag will disable loading of configuration files \
                    and use only CLI arguments and defaults. Use --no-config=false to enable config loading."
    )]
    pub no_config: Option<String>,

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
        num_args = 0..=1,
        value_name = "BOOL",
        help = "Process all stateful modules regardless of changes",
        long_help = "When enabled, this flag will process all stateful modules \
                    in the specified directory, regardless of whether they have been changed. \
                    Use --all=false to process only changed modules."
    )]
    pub all: Option<String>,

    #[clap(
        long,
        default_value = "main",
        help = "Default branch to compare against for changes",
        long_help = "Specify the default branch name to compare against when detecting changes. \
                    This is used to determine which modules have been modified since the last \
                    merge with the default branch. Default is 'main'."
    )]
    pub default_branch: String,
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
        num_args = 0..=1,
        value_name = "BOOL",
        help = "Process all stateful modules regardless of changes",
        long_help = "When enabled, this flag will process all stateful modules \
                    in the specified directory, regardless of whether they have been changed. \
                    Use --all=false to process only changed modules."
    )]
    pub all: Option<String>,

    #[clap(
        long,
        help = "Comma-separated list of var files to use",
        long_help = "Specify var files to use during plan operation. \
                    Multiple var files can be provided as comma-separated values. \
                    Example: --var-files var1.tfvars,var2.tfvars"
    )]
    pub var_files: Option<Vec<String>>,

    #[clap(
        long,
        num_args = 0..=1,
        value_name = "BOOL",
        help = "Watch background Terraform operations and display real-time status",
        long_help = "When enabled, Terraform operations will run in the background \
                    and this CLI will display real-time status updates. \
                    Without this flag, Terraform output is hidden until completion. \
                    Use --watch=false to hide real-time output."
    )]
    pub watch: Option<String>,

    /// Number of modules to process in parallel (max 4). Default is 1. This value is clamped to prevent system overload.
    #[clap(
        long,
        default_value = "1",
        help = "Number of parallel module processes (max 4)",
        long_help = "Specify the number of modules to process in parallel. \
                    The value is clamped to a maximum of 4 to prevent system overload. \
                    Default is 1 (sequential processing)."
    )]
    pub parallel: u32,

    #[clap(
        long,
        default_value = "main",
        help = "Default branch to compare against for changes",
        long_help = "Specify the default branch name to compare against when detecting changes. \
                    This is used to determine which modules have been modified since the last \
                    merge with the default branch. Default is 'main'."
    )]
    pub default_branch: String,
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
        value_name = "BOOL",
        help = "Run in dry-run mode (no changes will be applied)",
        long_help = "When enabled (default), this flag will run the apply command in dry-run mode, \
                    showing what changes would be made without actually applying them. \
                    Use --dry-run=false to apply actual changes."
    )]
    pub dry_run: String,

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
        num_args = 0..=1,
        value_name = "BOOL",
        help = "Process all stateful modules regardless of changes",
        long_help = "When enabled, this flag will process all stateful modules \
                    in the specified directory, regardless of whether they have been changed. \
                    Use --all=false to process only changed modules."
    )]
    pub all: Option<String>,

    #[clap(
        long,
        help = "Comma-separated list of var files to use",
        long_help = "Specify var files to use during apply operation. \
                    Multiple var files can be provided as comma-separated values. \
                    Example: --var-files var1.tfvars,var2.tfvars"
    )]
    pub var_files: Option<Vec<String>>,

    #[clap(
        long,
        num_args = 0..=1,
        value_name = "BOOL",
        help = "Watch background Terraform operations and display real-time status",
        long_help = "When enabled, Terraform operations will run in the background \
                    and this CLI will display real-time status updates. \
                    Without this flag, Terraform output is hidden until completion. \
                    Use --watch=false to hide real-time output."
    )]
    pub watch: Option<String>,

    /// Number of modules to process in parallel (max 4). Default is 1. This value is clamped to prevent system overload.
    #[clap(
        long,
        default_value = "1",
        help = "Number of parallel module processes (max 4)",
        long_help = "Specify the number of modules to process in parallel. \
                    The value is clamped to a maximum of 4 to prevent system overload. \
                    Default is 1 (sequential processing)."
    )]
    pub parallel: u32,

    #[clap(
        long,
        default_value = "main",
        help = "Default branch to compare against for changes",
        long_help = "Specify the default branch name to compare against when detecting changes. \
                    This is used to determine which modules have been modified since the last \
                    merge with the default branch. Default is 'main'."
    )]
    pub default_branch: String,
}
