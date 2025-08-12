use crate::cli::PlanArgs;
use crate::config::Settings;
use super::helpers;
use std::fs;
use std::path::Path;

pub fn execute(args: PlanArgs, settings: &Settings) -> anyhow::Result<()> {
    // Parse boolean strings
    let all = match &args.all {
        Some(value) => value.parse::<bool>().unwrap_or_else(|_| {
            eprintln!("Warning: Invalid value for --all: '{}'. Using default (true).", value);
            true
        }),
        None => false,
    };
    
    let watch = match &args.watch {
        Some(value) => value.parse::<bool>().unwrap_or_else(|_| {
            eprintln!("Warning: Invalid value for --watch: '{}'. Using default (true).", value);
            true
        }),
        None => false,
    };

    let output_dir = args.output_dir.as_deref().unwrap_or("terraform-plans");
    let output_path = Path::new(output_dir);

    if output_path.exists() {
        println!("📁 Using existing output directory: {}", output_dir);
    } else {
        println!("📁 Creating output directory: {}", output_dir);
        fs::create_dir_all(output_dir)?;
    }

    match helpers::get_changed_modules(&args.path, all, &args.default_branch) {
        Ok(modules) => {
            if all {
                println!("🔍 Found {} stateful modules", modules.len());
                println!("📦 All stateful modules will be planned...");
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
            
            println!("📦 Planning {} modules matching path: {}", filtered_modules.len(), args.path);
            println!("---------------------------------");
            for module in &filtered_modules {
                // Extract just the module name from the full path for cleaner output
                let module_name = module.split('/').last().unwrap_or(module);
                println!("  • {}", module_name);
            }
            println!("---------------------------------");

            helpers::run_terraform_plan(&filtered_modules, Some(output_dir), args.ignore_workspaces.as_deref(), args.var_files.as_deref(), settings.resolver(), watch, args.parallel)
                .map_err(|e| anyhow::anyhow!("Terraform plan failed: {}", e))?;
        }
        Err(e) => {
            eprintln!("Error getting changed modules: {}", e);
            return Err(anyhow::anyhow!("Failed to get changed modules: {}", e));
        }
    }
    Ok(())
}
