use crate::cli::ApplyArgs;
use crate::config::Settings;
use super::helpers;
use std::io;

pub fn execute(args: ApplyArgs, settings: &Settings) -> anyhow::Result<()> {
    println!("🚀 Starting Terraform apply...");
    if args.dry_run {
        println!("🔍 Running in dry-run mode (default) - no changes will be applied");
    } else {
        println!("⚠️  Running in APPLY mode - changes will be applied!");
    }

    match helpers::get_changed_modules(&args.path, args.all) {
        Ok(modules) => {
            if args.all {
                println!("🔍 Found {} stateful modules", modules.len());
                println!("📦 All stateful modules will be applied...");
            } else {
                if modules.is_empty() {
                    println!("🎉 No modules were changed!");
                    return Ok(());
                }
                println!("📦 Found {} changed modules:", modules.len());
            }
            println!("---------------------------------");
            for module in &modules {
                // Extract just the module name from the full path for cleaner output
                let module_name = module.split('/').last().unwrap_or(module);
                println!("  • {}", module_name);
            }
            println!("---------------------------------");
            
            // Filter modules based on the path argument if it's not "."
            let filtered_modules = if args.path != "." {
                println!("🔍 Filtering modules with path: {}", args.path);
                modules.into_iter()
                    .filter(|path| {
                        // Check if the path contains the root_dir
                        let contains_path = path.contains(&format!("/{}/", args.path)) || 
                                           path.ends_with(&format!("/{}", args.path));
                        contains_path
                    })
                    .collect::<Vec<String>>()
            } else {
                modules
            };
            
            if filtered_modules.is_empty() {
                println!("🎉 No modules match the specified path!");
                return Ok(());
            }
            
            println!("📦 Applying {} modules matching path: {}", filtered_modules.len(), args.path);
            println!("---------------------------------");
            for module in &filtered_modules {
                // Extract just the module name from the full path for cleaner output
                let module_name = module.split('/').last().unwrap_or(module);
                println!("  • {}", module_name);
            }
            println!("---------------------------------");

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

            helpers::run_terraform_apply(&filtered_modules, args.dry_run, args.ignore_workspaces.as_deref(), args.var_files.as_deref(), settings.resolver())
                .map_err(|e| anyhow::anyhow!("Terraform apply failed: {}", e))?;
            
            if args.dry_run {
                println!("\n🔍 Dry run completed - no changes were applied");
            } else {
                println!("\n✅ Changes applied successfully!");
            }
        }
        Err(e) => {
            eprintln!("Error getting changed modules: {}", e);
            return Err(anyhow::anyhow!("Failed to get changed modules: {}", e));
        }
    }
    Ok(())
}
