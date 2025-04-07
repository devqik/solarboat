use crate::cli::PlanArgs;
use super::helpers;
use std::io;
use std::fs;
use std::path::Path;

pub fn execute(args: PlanArgs) -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = args.output_dir.as_deref().unwrap_or("terraform-plans");
    let output_path = Path::new(output_dir);

    if output_path.exists() {
        println!("📁 Using existing output directory: {}", output_dir);
    } else {
        println!("📁 Creating output directory: {}", output_dir);
        fs::create_dir_all(output_dir)?;
    }

    let root_dir = &args.path;
    let ignore_workspaces = args.ignore_workspaces.as_deref();

    match helpers::get_changed_modules(&args.path, args.force) {
        Ok(modules) => {
            if args.force {
                println!("🔍 Found {} stateful modules", modules.len());
                println!("📦 Planning all stateful modules...");
            } else {
                println!("🔍 Found {} changed modules", modules.len());
                if modules.is_empty() {
                    println!("🎉 No modules were changed!");
                    return Ok(());
                }
                println!("📦 Planning changed modules:");
            }
            println!("---------------------------------");
            for module in &modules {
                println!("  • {}", module);
            }
            println!("---------------------------------");
            
            // Filter modules based on the path argument if it's not "."
            let filtered_modules = if root_dir != "." {
                println!("🔍 Filtering modules with path: {}", root_dir);
                modules.into_iter()
                    .filter(|path| {
                        // Check if the path contains the root_dir
                        let contains_path = path.contains(&format!("/{}/", root_dir)) || 
                                           path.ends_with(&format!("/{}", root_dir));
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
            
            println!("📦 Planning {} modules matching path: {}", filtered_modules.len(), root_dir);
            println!("---------------------------------");
            for module in &filtered_modules {
                println!("  • {}", module);
            }
            println!("---------------------------------");
            
            helpers::run_terraform_plan(&filtered_modules, Some(output_dir), ignore_workspaces)?;
        }
        Err(e) => {
            eprintln!("Error getting changed modules: {}", e);
            return Err(Box::new(io::Error::new(io::ErrorKind::Other, e)));
        }
    }
    Ok(())
}
