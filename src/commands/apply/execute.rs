use crate::cli::ApplyArgs;
use super::helpers;
use std::io;

pub fn execute(args: ApplyArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Terraform apply...");
    if args.dry_run {
        println!("🔍 Running in dry-run mode (default) - no changes will be applied");
    } else {
        println!("⚠️  Running in APPLY mode - changes will be applied!");
    }

    let ignore_workspaces = args.ignore_workspaces.as_deref();

    match helpers::get_changed_modules(".") {
        Ok(modules) => {
            println!("🔍 Found {} changed files", modules.len());
            if modules.is_empty() {
                println!("🎉 No modules were changed!");
                return Ok(());
            }
            println!("📦 Changed modules...");
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

            helpers::run_terraform_apply(&modules, args.dry_run, ignore_workspaces)?;
            
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
