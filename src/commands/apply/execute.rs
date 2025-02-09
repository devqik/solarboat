use crate::cli::ApplyArgs;
use super::helpers;
use std::io;

pub fn execute(args: ApplyArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Terraform apply...");
    if args.dry_run {
        println!("🔍 Running in dry-run mode - no changes will be applied");
    }

    let root_dir = ".";
    match helpers::get_changed_modules(root_dir) {
        Ok(modules) => {
            if modules.is_empty() {
                println!("🎉 No modules to apply!");
                return Ok(());
            }

            println!("📦 Modules to apply:");
            println!("---------------------------------");
            for module in &modules {
                println!("{}", module);
            }

            if !args.dry_run {
                println!("\n⚠️  You are about to apply changes to the above modules.");
                println!("Do you want to continue? [y/N]");
                
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("❌ Apply cancelled by user");
                    return Ok(());
                }
            }

            helpers::run_terraform_apply(&modules, args.dry_run)?;
            
            if args.dry_run {
                println!("\n🔍 Dry run completed - no changes were applied");
            } else {
                println!("\n✅ Changes applied successfully!");
            }
        }
        Err(e) => {
            eprintln!("Error getting changed modules: {}", e);
            return Err(Box::new(io::Error::new(io::ErrorKind::Other, e)));
        }
    }
    Ok(())
}
