use std::process::Command;
use crate::commands::scan::helpers;
use crate::commands::plan::helpers as plan_helpers;

#[derive(Debug)]
pub struct ModuleError {
    path: String,
    command: String,
    error: String,
}

pub fn get_changed_modules(root_dir: &str, force: bool) -> Result<Vec<String>, String> {
    // Use the scan helpers' get_changed_modules function directly
    helpers::get_changed_modules(root_dir, force)
}

pub fn run_terraform_apply(
    modules: &[String], 
    dry_run: bool,
    ignore_workspaces: Option<&[String]>
) -> Result<(), String> {
    
    if dry_run {
        println!("🔍 Running in dry-run mode - executing plan instead of apply");
        return plan_helpers::run_terraform_plan(modules, None, ignore_workspaces);
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

        let workspaces = plan_helpers::get_workspaces(module)?;
        
        if workspaces.len() <= 1 {
            println!("  🧱 Running terraform apply for default workspace...");
            if !run_single_apply(module)? {
                failed_modules.push(ModuleError {
                    path: module.clone(),
                    command: "apply".to_string(),
                    error: "Apply failed".to_string(),
                });
            }
        } else {
            println!("  🌐 Found multiple workspaces: {:?}", workspaces);
            for workspace in workspaces {
                // Skip ignored workspaces
                if let Some(ignored) = ignore_workspaces {
                    if ignored.contains(&workspace) {
                        println!("  ⏭️  Skipping ignored workspace: {}", workspace);
                        continue;
                    }
                }

                println!("  🔄 Switching to workspace: {}", workspace);
                plan_helpers::select_workspace(module, &workspace)?;
                
                println!("  🧱 Running terraform apply for workspace {}...", workspace);
                if !run_single_apply(module)? {
                    failed_modules.push(ModuleError {
                        path: format!("{}:{}", module, workspace),
                        command: "apply".to_string(),
                        error: format!("Apply failed for workspace {}", workspace),
                    });
                }
            }
        }
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

fn run_single_apply(module: &str) -> Result<bool, String> {
    let cmd_status = Command::new("terraform")
        .args(&["apply", "-auto-approve"])
        .current_dir(module)
        .status()
        .map_err(|e| e.to_string())?;

    Ok(cmd_status.success())
}
