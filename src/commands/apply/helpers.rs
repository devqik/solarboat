use crate::utils::scan_utils;
use crate::commands::plan::helpers as plan_helpers;
use crate::utils::parallel_processor::ParallelProcessor;
use crate::utils::terraform_operations::{TerraformOperation, OperationType, ensure_module_initialized};
use crate::config::ConfigResolver;
use crate::utils::display_utils::{format_module_path, format_workspace_list};

#[derive(Debug)]
pub struct ModuleError {
    path: String,
    command: String,
    error: String,
}

pub fn get_changed_modules(root_dir: &str, force: bool, default_branch: &str) -> Result<Vec<String>, String> {
    scan_utils::get_changed_modules(root_dir, force, default_branch)
}

pub fn run_terraform_apply(
    modules: &[String], 
    dry_run: bool,
    ignore_workspaces: Option<&[String]>,
    var_files: Option<&[String]>,
    config_resolver: &ConfigResolver,
    watch: bool,
    parallel: u32,
) -> Result<(), String> {
    if dry_run {
        println!("🔍 Running in dry-run mode - executing plan instead of apply");
        return plan_helpers::run_terraform_plan(modules, None, ignore_workspaces, var_files, config_resolver, watch, parallel);
    }

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

        ensure_module_initialized(module)?;
        
        let workspaces = plan_helpers::get_workspaces(module)?;
        
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
                operation_type: OperationType::Apply,
                watch,
                skip_init: true, // Already initialized before workspace listing
            };
            processor.add_operation(operation);
        } else {

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
                    operation_type: OperationType::Apply,
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
                command: "apply".to_string(),
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
