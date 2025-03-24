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

    match helpers::get_changed_modules(root_dir, args.force) {
        Ok(modules) => {
            if args.force {
                println!("🔍 Found {} stateful modules", modules.len());
                println!("📦 All stateful modules will be planned...");
            } else {
                println!("🔍 Found {} changed files", modules.len());
                if modules.is_empty() {
                    println!("🎉 No modules were changed!");
                    return Ok(());
                }
                println!("📦 Changed modules...");
            }
            println!("---------------------------------");
            for module in &modules {
                println!("{}", module);
            }
            helpers::run_terraform_plan(&modules, Some(output_dir), ignore_workspaces)?;
        }
        Err(e) => {
            eprintln!("Error getting changed modules: {}", e);
            return Err(Box::new(io::Error::new(io::ErrorKind::Other, e)));
        }
    }
    Ok(())
}
