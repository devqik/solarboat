use crate::cli::ScanArgs;
use super::helpers;
use std::io;
use std::collections::HashSet;

pub fn execute(args: ScanArgs) -> Result<(), Box<dyn std::error::Error>> {
    match helpers::get_changed_modules(&args.path, args.all) {
        Ok(modules) => {
            // Use a HashSet to deduplicate modules based on their names
            let mut unique_module_names = HashSet::new();
            let unique_modules: Vec<_> = modules.iter()
                .filter(|module| {
                    let module_name = module.split('/').last().unwrap_or(module);
                    unique_module_names.insert(module_name.to_string())
                })
                .collect();
            
            if args.all {
                println!("🔍 Found {} stateful modules", unique_modules.len());
                println!("📦 All stateful modules will be scanned...");
            } else {
                if unique_modules.is_empty() {
                    println!("🎉 No modules were changed!");
                    return Ok(());
                }
                println!("📦 Found {} changed modules:", unique_modules.len());
            }
            println!("---------------------------------");
            
            // Sort module names for consistent output
            let mut sorted_module_names: Vec<_> = unique_module_names.into_iter().collect();
            sorted_module_names.sort();
            
            for module_name in sorted_module_names {
                println!("  • {}", module_name);
            }
            println!("---------------------------------");
        }
        Err(e) => {
            eprintln!("Error getting changed modules: {}", e);
            return Err(Box::new(io::Error::new(io::ErrorKind::Other, e)));
        }
    }
    Ok(())
}
