use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use crate::commands::scan::helpers;

#[derive(Debug)]
pub struct ModuleError {
    path: String,
    command: String,
    error: String,
}

pub fn get_changed_modules(root_dir: &str) -> Result<Vec<String>, String> {
    let mut modules = HashMap::new();

    helpers::discover_modules(root_dir, &mut modules)?;
    helpers::build_dependency_graph(&mut modules)?;

    let changed_files = helpers::get_git_changed_files(root_dir)?;
    let affected_modules = helpers::process_changed_modules(&changed_files, &mut modules)?;

    Ok(affected_modules)
}

pub fn run_terraform_command(modules: &[String], command: &str, plan_dir: Option<&str>) -> Result<(), String> {
    let mut failed_modules = Vec::new();

    for module in modules {
        println!("\n📦 Processing module: {}", module);

        println!("  🔧 Initializing module...");
        let init_status = Command::new("terraform")
            .arg("init")
            .current_dir(module)
            .status()
            .map_err(|e| e.to_string())?;

        if !init_status.success() {
            println!("  ❌ Initialization failed, skipping module");
            failed_modules.push(ModuleError {
                path: module.clone(),
                command: "init".to_string(),
                error: "Initialization failed".to_string(),
            });
            continue;
        }

        println!("  🚀 Running terraform {}...", command);
        let mut args = vec![command.to_string()];
        if command == "plan" {
            if let Some(plan_dir) = plan_dir {
                if let Some(module_name) = Path::new(module).file_name().and_then(|n| n.to_str()) {
                    let plan_file = Path::new(plan_dir).join(format!("{}.tfplan", module_name));
                    args.push(format!("-out={}", plan_file.to_str().unwrap()));
                }
            }
        }

        let cmd_status = Command::new("terraform")
            .args(&args)
            .current_dir(module)
            .status()
            .map_err(|e| e.to_string())?;

        if !cmd_status.success() {
            failed_modules.push(ModuleError {
                path: module.clone(),
                command: command.to_string(),
                error: "Command failed".to_string(),
            });
            continue;
        }

        println!("  ✅ Module processed successfully");
    }

    if !failed_modules.is_empty() {
        println!("\n⚠️  Some modules failed to process:");
        for failure in &failed_modules {
            println!("  ❌ {}: {} failed - {}", failure.path, failure.command, failure.error);
        }
        return Err(format!("Failed to process {} module(s)", failed_modules.len()));
    }

    Ok(())
}
