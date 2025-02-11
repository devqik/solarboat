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

pub fn run_terraform_plan(modules: &[String], plan_dir: Option<&str>) -> Result<(), String> {
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

        let workspaces = get_workspaces(module)?;
        
        if workspaces.len() <= 1 {
            println!("  🚀 Running terraform plan for default workspace...");
            if !run_single_plan(module, plan_dir)? {
                failed_modules.push(ModuleError {
                    path: module.clone(),
                    command: "plan".to_string(),
                    error: "Plan failed".to_string(),
                });
            }
        } else {
            println!("  🌐 Found multiple workspaces: {:?}", workspaces);
            for workspace in workspaces {
                println!("  🔄 Switching to workspace: {}", workspace);
                select_workspace(module, &workspace)?;
                
                println!("  🚀 Running terraform plan for workspace {}...", workspace);
                if !run_single_plan(module, plan_dir)? {
                    failed_modules.push(ModuleError {
                        path: format!("{}:{}", module, workspace),
                        command: "plan".to_string(),
                        error: format!("Plan failed for workspace {}", workspace),
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

fn run_single_plan(module: &str, plan_dir: Option<&str>) -> Result<bool, String> {
    let mut terraform_cmd = Command::new("terraform");
    terraform_cmd.arg("plan").current_dir(module);

    if let Some(plan_dir) = plan_dir {
        if let Some(module_name) = Path::new(module).file_name().and_then(|n| n.to_str()) {
            let plan_file = Path::new(plan_dir).join(format!("{}.tfplan", module_name));
            terraform_cmd.arg(format!("-out={}", plan_file.to_str().unwrap()));
        }
    }

    let cmd_status = terraform_cmd
        .status()
        .map_err(|e| e.to_string())?;

    Ok(cmd_status.success())
}

pub fn get_workspaces(module_path: &str) -> Result<Vec<String>, String> {
    let output = Command::new("terraform")
        .arg("workspace")
        .arg("list")
        .current_dir(module_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("Failed to list workspaces".to_string());
    }

    let workspaces: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.trim().trim_start_matches('*').trim().to_string())
        .filter(|ws| !ws.is_empty())
        .collect();

    Ok(workspaces)
}

pub fn select_workspace(module_path: &str, workspace: &str) -> Result<(), String> {
    let output = Command::new("terraform")
        .arg("workspace")
        .arg("select")
        .arg(workspace)
        .current_dir(module_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(format!("Failed to select workspace {}", workspace));
    }

    Ok(())
}
