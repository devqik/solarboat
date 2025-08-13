use crate::cli::ScanArgs;
use crate::config::Settings;
use crate::utils::scan_utils;
use crate::utils::logger;
use std::collections::HashSet;
use std::process::Command;
use std::time::Instant;

pub fn execute(args: ScanArgs, _settings: &Settings) -> anyhow::Result<()> {
    let start_time = Instant::now();
    
    // Parse all string as boolean
    let all = match &args.all {
        Some(value) => value.parse::<bool>().unwrap_or_else(|_| {
            logger::warn(&format!("Invalid value for --all: '{}'. Using default (true).", value));
            true
        }),
        None => false, // Flag not provided
    };

    // Show configuration summary
    logger::config_summary(&[
        ("Scan Path", &args.path),
        ("Default Branch", &args.default_branch),
        ("Recent Commits", &args.recent_commits.to_string()),
        ("Process All", &all.to_string()),
    ]);

    // Check if the specified path is a git repository
    logger::step(1, 3, "Checking git repository");
    let git_check = Command::new("git")
        .args(&["rev-parse", "--is-inside-work-tree"])
        .current_dir(&args.path)
        .output();

    match git_check {
        Ok(output) if output.status.success() => {
            logger::success("Git repository found");
            
            // Scan for changed modules
            logger::step(2, 3, "Scanning for changed modules");
            let progress = logger::progress("Analyzing git changes and module dependencies");
            
            match scan_utils::get_changed_modules_clean(&args.path, all, &args.default_branch, args.recent_commits) {
                Ok(modules) => {
                    if let Some(progress) = progress {
                        progress.complete(true);
                    }
                    
                    // Use a HashSet to deduplicate modules based on their names
                    let mut unique_module_names = HashSet::new();
                    let unique_modules: Vec<_> = modules.iter()
                        .filter(|module| {
                            let module_name = module.split('/').last().unwrap_or(module);
                            unique_module_names.insert(module_name.to_string())
                        })
                        .collect();
                    
                    logger::step(3, 3, "Processing results");
                    
                    if all {
                        logger::info(&format!("Found {} stateful modules", unique_modules.len()));
                        logger::warning_box(
                            "Processing All Modules", 
                            "All stateful modules will be processed regardless of changes"
                        );
                    } else {
                        if unique_modules.is_empty() {
                            logger::success_box(
                                "No Changes Detected", 
                                "No modules were changed since the last merge with the default branch"
                            );
                            return Ok(());
                        }
                        logger::changes_detected(unique_modules.len(), &modules);
                    }
                    
                    // Sort module names for consistent output
                    let mut sorted_module_names: Vec<_> = unique_module_names.into_iter().collect();
                    sorted_module_names.sort();
                    
                    logger::section("Modules to Process");
                    logger::list(&sorted_module_names.iter().map(|s| s.as_str()).collect::<Vec<_>>(), None);
                    
                    // Show results summary
                    let duration = start_time.elapsed();
                    logger::results_summary("Scan Complete", &[
                        ("Total Modules", &unique_modules.len().to_string()),
                        ("Scan Path", &args.path),
                        ("Duration", &format!("{:.2}s", duration.as_secs_f64())),
                    ]);
                }
                Err(e) => {
                    if let Some(progress) = progress {
                        progress.complete(false);
                    }
                    logger::error_box("Scan Failed", &format!("Failed to get changed modules: {}", e));
                    return Err(anyhow::anyhow!("Failed to get changed modules: {}", e));
                }
            }
        }
        _ => {
            logger::error_box(
                "Git Repository Not Found", 
                &format!("Path '{}' is not a git repository. Please specify a path that is within a git repository.", args.path)
            );
            return Err(anyhow::anyhow!("Path '{}' is not a git repository", args.path));
        }
    }
    Ok(())
}
