use std::collections::HashMap;
use std::process::Command;
use crate::commands::scan::helpers;
use crate::commands::plan::helpers as plan_helpers;

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

pub fn run_terraform_apply(modules: &[String], dry_run: bool) -> Result<(), String> {
    if dry_run {
        println!("🔍 Running in dry-run mode - executing plan instead of apply");
        return plan_helpers::run_terraform_plan(modules, None);
    }

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

        println!("  🚀 Running terraform apply...");
        let cmd_status = Command::new("terraform")
            .args(&["apply", "-auto-approve"])
            .current_dir(module)
            .status()
            .map_err(|e| e.to_string())?;

        if !cmd_status.success() {
            failed_modules.push(ModuleError {
                path: module.clone(),
                command: "apply".to_string(),
                error: "Apply failed".to_string(),
            });
            continue;
        }

        println!("  ✅ Module applied successfully");
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
