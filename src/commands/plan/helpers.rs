use std::path::Path;
use std::process::Command;
use crate::commands::scan::helpers;
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
    ignore_workspaces: Option<&[String]>
) -> Result<(), String> {

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
                if let Some(ignored) = ignore_workspaces {
                    if ignored.contains(&workspace) {
                        println!("  ⏭️  Skipping ignored workspace: {}", workspace);
                        continue;
                    }
                }

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
