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
                println!("🔍 Found {} changed files", modules.len());
                if modules.is_empty() {
                    println!("🎉 No modules were changed!");
                    return Ok(());
                }
                println!("📦 Changed modules...");
            }
            println!("---------------------------------");
            for module in modules {
                println!("{}", module);
            }
        }
        Err(e) => {
            eprintln!("Error getting changed modules: {}", e);
            return Err(Box::new(io::Error::new(io::ErrorKind::Other, e)));
        }
    }
    Ok(())
}
