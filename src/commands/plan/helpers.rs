use std::path::{Path, PathBuf};
use std::process::Command;
use crate::commands::scan::helpers;
use crate::config::ConfigResolver;
use crate::utils::terraform_background::{BackgroundTerraform, run_terraform_silent};
use regex::Regex;

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

pub fn run_terraform_plan(
    modules: &[String], 
    plan_dir: Option<&str>,
    ignore_workspaces: Option<&[String]>,
    var_files: Option<&[String]>,
    config_resolver: &ConfigResolver,
    watch: bool,
) -> Result<(), String> {

    let mut failed_modules = Vec::new();

    for module in modules {
        println!("\n📦 Processing module: {}", module);

        if watch {
            println!("  🔧 Initializing module in background...");
            let mut background_tf = BackgroundTerraform::new();
            background_tf.init_background(module)?;
            
            // Wait for initialization to complete
            match background_tf.wait_for_completion(300) { // 5 minute timeout
                Ok(success) => {
                    if !success {
                        println!("  ❌ Initialization failed, skipping module");
                        failed_modules.push(ModuleError {
                            path: module.clone(),
                            command: "init".to_string(),
                            error: "Initialization failed".to_string(),
                        });
                        continue;
                    }
                    println!("  ✅ Initialization completed");
                }
                Err(e) => {
                    println!("  ❌ Initialization failed: {}, skipping module", e);
                    failed_modules.push(ModuleError {
                        path: module.clone(),
                        command: "init".to_string(),
                        error: format!("Initialization failed: {}", e),
                    });
                    continue;
                }
            }
        } else {
            println!("  🔧 Initializing module...");
            let init_success = run_terraform_silent("init", &[], module, None)?;
            if !init_success {
                println!("  ❌ Initialization failed, skipping module");
                failed_modules.push(ModuleError {
                    path: module.clone(),
                    command: "init".to_string(),
                    error: "Initialization failed".to_string(),
                });
                continue;
            }
        }

        let workspaces = get_workspaces(module)?;
        
        if workspaces.len() <= 1 {
            println!("  🚀 Running terraform plan for default workspace...");
            // Get var files for default workspace
            let default_var_files = config_resolver.get_workspace_var_files(module, "default", var_files);
            if !default_var_files.is_empty() {
                println!("  📄 Using {} var files for default workspace", default_var_files.len());
            }
            
            if watch {
                let mut background_tf = BackgroundTerraform::new();
                background_tf.plan_background(module, Some(&default_var_files))?;
                
                // Wait for plan to complete
                match background_tf.wait_for_completion(600) { // 10 minute timeout
                    Ok(success) => {
                        if !success {
                            failed_modules.push(ModuleError {
                                path: module.clone(),
                                command: "plan".to_string(),
                                error: "Plan failed".to_string(),
                            });
                        } else {
                            // Save plan output if plan_dir is specified
                            if let Some(plan_dir) = plan_dir {
                                save_plan_output(module, plan_dir, &background_tf.get_output())?;
                            }
                        }
                    }
                    Err(e) => {
                        failed_modules.push(ModuleError {
                            path: module.clone(),
                            command: "plan".to_string(),
                            error: format!("Plan failed: {}", e),
                        });
                    }
                }
            } else {
                if !run_single_plan(module, plan_dir, Some(&default_var_files))? {
                    failed_modules.push(ModuleError {
                        path: module.clone(),
                        command: "plan".to_string(),
                        error: "Plan failed".to_string(),
                    });
                }
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
                select_workspace(module, &workspace)?;
                
                println!("  🚀 Running terraform plan for workspace {}...", workspace);
                
                // Get workspace-specific var files
                let workspace_var_files = config_resolver.get_workspace_var_files(module, &workspace, var_files);
                if !workspace_var_files.is_empty() {
                    println!("  📄 Using {} var files for workspace {}", workspace_var_files.len(), workspace);
                }
                
                if watch {
                    let mut background_tf = BackgroundTerraform::new();
                    background_tf.plan_background(module, Some(&workspace_var_files))?;
                    
                    // Wait for plan to complete
                    match background_tf.wait_for_completion(600) { // 10 minute timeout
                        Ok(success) => {
                            if !success {
                                failed_modules.push(ModuleError {
                                    path: format!("{}:{}", module, workspace),
                                    command: "plan".to_string(),
                                    error: format!("Plan failed for workspace {}", workspace),
                                });
                            } else {
                                // Save plan output if plan_dir is specified
                                if let Some(plan_dir) = plan_dir {
                                    save_plan_output(module, plan_dir, &background_tf.get_output())?;
                                }
                            }
                        }
                        Err(e) => {
                            failed_modules.push(ModuleError {
                                path: format!("{}:{}", module, workspace),
                                command: "plan".to_string(),
                                error: format!("Plan failed for workspace {}: {}", workspace, e),
                            });
                        }
                    }
                } else {
                    if !run_single_plan(module, plan_dir, Some(&workspace_var_files))? {
                        failed_modules.push(ModuleError {
                            path: format!("{}:{}", module, workspace),
                            command: "plan".to_string(),
                            error: format!("Plan failed for workspace {}", workspace),
                        });
                    }
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

fn run_single_plan(module: &str, plan_dir: Option<&str>, var_files: Option<&[String]>) -> Result<bool, String> {
    let mut terraform_cmd = Command::new("terraform");
    terraform_cmd.arg("plan").current_dir(module);
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

    // Run terraform plan without specifying an output file
    let output = terraform_cmd
        .output()
        .map_err(|e| e.to_string())?;

    // Check if the plan was successful
    if !output.status.success() {
        // Print the error output
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        return Ok(false);
    }

    // If plan_dir is specified, save the plan output to a markdown file
    if let Some(plan_dir) = plan_dir {
        // Create the plan directory if it doesn't exist
        std::fs::create_dir_all(plan_dir)
            .map_err(|e| format!("Failed to create plan directory: {}", e))?;
            
        if let Some(module_name) = Path::new(module).file_name().and_then(|n| n.to_str()) {
            let plan_file = Path::new(plan_dir).join(format!("{}.tfplan.md", module_name));
            
            // Get the plan output as a string
            let plan_output = String::from_utf8_lossy(&output.stdout).to_string();
            
            // Strip ANSI color codes and format the output
            let cleaned_output = clean_terraform_output(&plan_output);
            
            // Create markdown content with the plan inside a code block
            let markdown_content = format!("```terraform\n{}\n```", cleaned_output);
            
            // Write the markdown content to a file
            std::fs::write(&plan_file, markdown_content)
                .map_err(|e| format!("Failed to write plan file: {}", e))?;
                
            println!("  ✅ Plan saved to: {}", plan_file.to_str().unwrap());
        }
    }

    Ok(true)
}

fn save_plan_output(module: &str, plan_dir: &str, output_lines: &[String]) -> Result<(), String> {
    // Create the plan directory if it doesn't exist
    std::fs::create_dir_all(plan_dir)
        .map_err(|e| format!("Failed to create plan directory: {}", e))?;
        
    if let Some(module_name) = Path::new(module).file_name().and_then(|n| n.to_str()) {
        let plan_file = Path::new(plan_dir).join(format!("{}.tfplan.md", module_name));
        
        // Join all output lines
        let plan_output = output_lines.join("\n");
        
        // Strip ANSI color codes and format the output
        let cleaned_output = clean_terraform_output(&plan_output);
        
        // Create markdown content with the plan inside a code block
        let markdown_content = format!("```terraform\n{}\n```", cleaned_output);
        
        // Write the markdown content to a file
        std::fs::write(&plan_file, markdown_content)
            .map_err(|e| format!("Failed to write plan file: {}", e))?;
            
        println!("  ✅ Plan saved to: {}", plan_file.to_str().unwrap());
    }
    
    Ok(())
}

// Helper function to clean Terraform output by removing ANSI codes and formatting
fn clean_terraform_output(input: &str) -> String {
    // Remove ANSI color codes
    let ansi_regex = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    let mut cleaned = ansi_regex.replace_all(input, "").to_string();
    
    // Remove bold formatting
    cleaned = cleaned.replace("[1m", "").replace("[0m", "");
    
    // Clean up extra spaces and formatting
    cleaned = cleaned.replace("  +", "  +");
    cleaned = cleaned.replace("  [0m", "");
    
    // Remove any remaining ANSI codes
    cleaned = cleaned.replace("[32m", "").replace("[0m", "");
    
    // Clean up any double spaces
    while cleaned.contains("  ") {
        cleaned = cleaned.replace("  ", " ");
    }
    
    // Ensure proper indentation
    let lines: Vec<&str> = cleaned.lines().collect();
    let mut formatted_lines = Vec::new();
    
    for line in lines {
        // Skip empty lines
        if line.trim().is_empty() {
            formatted_lines.push(String::new());
            continue;
        }
        
        // Preserve indentation for resource blocks
        if line.contains("resource \"") {
            formatted_lines.push(line.to_string());
        } else if line.contains("=") {
            // Indent attribute lines
            formatted_lines.push(format!("  {}", line.trim()));
        } else {
            formatted_lines.push(line.to_string());
        }
    }
    
    formatted_lines.join("\n")
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
