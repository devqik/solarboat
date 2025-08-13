use crate::utils::scan_utils;
use crate::config::ConfigResolver;
use crate::utils::parallel_processor::ParallelProcessor;
use crate::utils::terraform_operations::{TerraformOperation, OperationType, ensure_module_initialized};
use crate::utils::display_utils::{format_module_path, format_workspace_list};

#[derive(Debug)]
pub struct ModuleError {
    path: String,
    command: String,
    error: String,
}

pub fn get_changed_modules(root_dir: &str, force: bool, default_branch: &str, recent_commits: u32) -> Result<Vec<String>, String> {
    scan_utils::get_changed_modules(root_dir, force, default_branch, recent_commits)
}

pub fn run_terraform_plan(
    modules: &[String], 
    plan_dir: Option<&str>,
    ignore_workspaces: Option<&[String]>,
    var_files: Option<&[String]>,
    config_resolver: &ConfigResolver,
    watch: bool,
    parallel: u32,
) -> Result<(), String> {
    // Force parallel to 1 if watch mode is enabled
    let effective_parallel = if watch {
        println!("🔄 Watch mode enabled - forcing parallel processing to 1 for real-time output");
        1
    } else {
        parallel
    };
    
    // Clamp parallel to max 4
    let parallel_limit = effective_parallel.min(4) as usize;
    
    // Create parallel processor
    let mut processor = ParallelProcessor::new(parallel_limit);
    
    // Build operations for all modules and workspaces
    for module in modules {
        let display_path = format_module_path(module);
        println!("\n📦 {}", display_path);
        
        // Ensure module is initialized before trying to list workspaces
        ensure_module_initialized(module)?;
        
        let workspaces = get_workspaces(module)?;
        
        if workspaces.len() <= 1 {
            // Single workspace (default)
            let default_var_files = config_resolver.get_workspace_var_files(module, "default", var_files);
            if !default_var_files.is_empty() {
                println!("  📄 Using {} var files", default_var_files.len());
            }
            
            let operation = TerraformOperation {
                module_path: module.clone(),
                workspace: None, // None means default workspace
                var_files: default_var_files,
                operation_type: OperationType::Plan { 
                    plan_dir: plan_dir.map(|s| s.to_string()) 
                },
                watch,
                skip_init: true, // Already initialized before workspace listing
            };
            processor.add_operation(operation);
        } else {
            // Multiple workspaces
            println!("  🌐 Found workspaces: {}", format_workspace_list(&workspaces));
            
            for workspace in workspaces {
                // Check if workspace should be ignored using config resolver
                if config_resolver.should_ignore_workspace(module, &workspace, ignore_workspaces) {
                    if workspace == "default" {
                        println!("  ⏭️  Skipping: {} (auto-ignored)", workspace);
                        continue;
                    } else {
                        println!("  ⏭️  Skipping: {} (configured)", workspace);
                        continue;
                    }
                }
                
                println!("  🔄 Processing: {}", workspace);
                
                // Get workspace-specific var files
                let workspace_var_files = config_resolver.get_workspace_var_files(module, &workspace, var_files);
                if !workspace_var_files.is_empty() {
                    println!("  📄 Using {} var files", workspace_var_files.len());
                }
                
                let operation = TerraformOperation {
                    module_path: module.clone(),
                    workspace: Some(workspace.clone()),
                    var_files: workspace_var_files,
                    operation_type: OperationType::Plan { 
                        plan_dir: plan_dir.map(|s| s.to_string()) 
                    },
                    watch,
                    skip_init: true, // Already initialized before workspace listing
                };
                processor.add_operation(operation);
            }
        }
    }
    
    // Start processing
    println!("\n🚀 Starting parallel processing with {} workers...", parallel_limit);
    processor.start();
    
    // Wait for completion and collect results
    let results = processor.wait_for_completion();
    
    // Process results and report failures
    let mut failed_modules = Vec::new();
    
    for result in results {
        if !result.success {
            let module_path = match &result.workspace {
                Some(workspace) => format!("{}:{}", result.module_path, workspace),
                None => result.module_path.clone(),
            };
            
            failed_modules.push(ModuleError {
                path: module_path,
                command: "plan".to_string(),
                error: result.error.unwrap_or_else(|| "Unknown error".to_string()),
            });
        }
    }
    
    if !failed_modules.is_empty() {
        println!("\n⚠️  Some modules failed to process:");
        for failure in &failed_modules {
            println!("  ❌ {}: {} failed - {}", failure.path, failure.command, failure.error);
        }
        return Err(format!("Failed to process {} module(s)", failed_modules.len()));
    }
    
    println!("\n✅ All modules processed successfully!");
    Ok(())
}

pub fn get_workspaces(module_path: &str) -> Result<Vec<String>, String> {
    let output = std::process::Command::new("terraform")
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
