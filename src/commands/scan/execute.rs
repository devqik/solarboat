use crate::cli::ScanArgs;
use super::helpers;
use std::io;

pub fn execute(args: ScanArgs) -> Result<(), Box<dyn std::error::Error>> {
    match helpers::get_changed_modules(&args.path, args.force) {
        Ok(modules) => {
            if args.force {
                println!("🔍 Found {} stateful modules", modules.len());
                println!("📦 All stateful modules will be scanned...");
            } else {
                if modules.is_empty() {
                    println!("🎉 No modules were changed!");
                    return Ok(());
                }
                println!("📦 Found {} changed modules:", modules.len());
            }
            println!("---------------------------------");
            for module in modules {
                // Extract just the module name from the full path for cleaner output
                let module_name = module.split('/').last().unwrap_or(&module);
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
