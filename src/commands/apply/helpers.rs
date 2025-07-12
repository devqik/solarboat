use std::path::{Path, PathBuf};
use std::process::Command;
use crate::commands::scan::helpers;
use crate::commands::plan::helpers as plan_helpers;
use crate::config::ConfigResolver;

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
    ignore_workspaces: Option<&[String]>,
    var_files: Option<&[String]>,
    config_resolver: &ConfigResolver,
) -> Result<(), String> {
    
    if dry_run {
        println!("🔍 Running in dry-run mode - executing plan instead of apply");
        return plan_helpers::run_terraform_plan(modules, None, ignore_workspaces, var_files, config_resolver);
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
            // Get var files for default workspace
            let default_var_files = config_resolver.get_workspace_var_files(module, "default", var_files);
            if !default_var_files.is_empty() {
                println!("  📄 Using {} var files for default workspace", default_var_files.len());
            }
            if !run_single_apply(module, Some(&default_var_files))? {
                failed_modules.push(ModuleError {
                    path: module.clone(),
                    command: "apply".to_string(),
                    error: "Apply failed".to_string(),
                });
            }
        } else {
            println!("  🌐 Found multiple workspaces: {:?}", workspaces);
            
            for workspace in workspaces {
                // Check if workspace should be ignored using config resolver
                if config_resolver.should_ignore_workspace(module, &workspace, ignore_workspaces) {
                    if workspace == "default" {
                        println!("  ⏭️  Skipping default workspace (auto-ignored when multiple workspaces exist)");
                        continue;
                    } else {
                        println!("  ⏭️  Skipping ignored workspace: {} (from configuration)", workspace);
                        continue;
                    }
                }

                println!("  🔄 Switching to workspace: {}", workspace);
                plan_helpers::select_workspace(module, &workspace)?;
                
                println!("  🧱 Running terraform apply for workspace {}...", workspace);
                
                // Get workspace-specific var files
                let workspace_var_files = config_resolver.get_workspace_var_files(module, &workspace, var_files);
                if !workspace_var_files.is_empty() {
                    println!("  📄 Using {} var files for workspace {}", workspace_var_files.len(), workspace);
                }
                if !run_single_apply(module, Some(&workspace_var_files))? {
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

fn run_single_apply(module: &str, var_files: Option<&[String]>) -> Result<bool, String> {
    let mut terraform_cmd = Command::new("terraform");
    terraform_cmd.arg("apply").arg("-auto-approve").current_dir(module);
    if let Some(var_files) = var_files {
        for var_file in var_files {
            // Resolve var file path relative to module directory
            let var_file_path = if Path::new(var_file).is_absolute() {
                PathBuf::from(var_file)
            } else {
                // Get current working directory
                let current_dir = std::env::current_dir()
                    .map_err(|e| format!("Failed to get current directory: {}", e))?;
                
                // Create absolute path to var file from current directory
                let absolute_var_file = current_dir.join(var_file);
                
                // Create absolute path to module
                let absolute_module = current_dir.join(module);
                
                // Calculate relative path from module to var file
                match absolute_var_file.strip_prefix(&absolute_module) {
                    Ok(relative_path) => {
                        // If var file is within module directory, use relative path
                        relative_path.to_path_buf()
                    }
                    Err(_) => {
                        // If var file is outside module directory, calculate relative path
                        let mut relative_path = PathBuf::new();
                        let module_components: Vec<_> = absolute_module.components().collect();
                        let var_file_components: Vec<_> = absolute_var_file.components().collect();
                        
                        // Find common prefix
                        let mut common_len = 0;
                        for (i, (m, v)) in module_components.iter().zip(var_file_components.iter()).enumerate() {
                            if m == v {
                                common_len = i + 1;
                            } else {
                                break;
                            }
                        }
                        
                        // Add "../" for each component in module path after common prefix
                        for _ in common_len..module_components.len() {
                            relative_path.push("..");
                        }
                        
                        // Add remaining components from var file path
                        for component in &var_file_components[common_len..] {
                            relative_path.push(component);
                        }
                        
                        relative_path
                    }
                }
            };
            
            terraform_cmd.arg("-var-file").arg(&var_file_path);
        }
    }
    let cmd_status = terraform_cmd
        .current_dir(module)
        .status()
        .map_err(|e| e.to_string())?;

    Ok(cmd_status.success())
}
