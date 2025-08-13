use crate::cli::ApplyArgs;
use crate::config::Settings;
use crate::utils::logger;
use super::helpers;
use std::time::Instant;

pub fn execute(args: ApplyArgs, settings: &Settings) -> anyhow::Result<()> {
    let start_time = Instant::now();
    
    logger::section("Terraform Apply");
    
    let dry_run = args.dry_run.parse::<bool>().unwrap_or_else(|_| {
        logger::warn(&format!("Invalid value for --dry-run: '{}'. Using default (true).", args.dry_run));
        true
    });
    
    let all = match &args.all {
        Some(value) => value.parse::<bool>().unwrap_or_else(|_| {
            logger::warn(&format!("Invalid value for --all: '{}'. Using default (true).", value));
            true
        }),
        None => false,
    };
    
    let watch = match &args.watch {
        Some(value) => value.parse::<bool>().unwrap_or_else(|_| {
            logger::warn(&format!("Invalid value for --watch: '{}'. Using default (true).", value));
            true
        }),
        None => false,
    };

    // Show configuration summary
    logger::config_summary(&[
        ("Apply Path", &args.path),
        ("Default Branch", &args.default_branch),
        ("Recent Commits", &args.recent_commits.to_string()),
        ("Process All", &all.to_string()),
        ("Watch Mode", &watch.to_string()),
        ("Parallel Jobs", &args.parallel.to_string()),
        ("Dry Run", &dry_run.to_string()),
    ]);

    if dry_run {
        logger::info("Running in dry-run mode (default) - no changes will be applied");
    } else {
        logger::warning_box(
            "Live Apply Mode", 
            "Running in APPLY mode - changes will be applied to your infrastructure!"
        );
    }

    // Get changed modules
    logger::step(1, 4, "Detecting changed modules");
    let progress = logger::progress("Analyzing git changes and module dependencies");
    
                match helpers::get_changed_modules(&args.path, all, &args.default_branch, args.recent_commits) {
                Ok(modules) => {
                    if let Some(progress) = progress {
                        progress.complete(true);
                    }
            
            if all {
                logger::info(&format!("Found {} stateful modules", modules.len()));
                logger::warning_box(
                    "Processing All Modules", 
                    "All stateful modules will be applied regardless of changes"
                );
            } else {
                if modules.is_empty() {
                    logger::success_box(
                        "No Changes Detected", 
                        "No modules were changed since the last merge with the default branch"
                    );
                    return Ok(());
                }
                logger::changes_detected(modules.len(), &modules);
            }
            
            // Filter modules based on the path argument if it's not "."
            logger::step(2, 4, "Filtering modules by path");
            let filtered_modules = if args.path != "." {
                logger::info(&format!("Filtering modules with path: {}", args.path));
                modules.into_iter()
                    .filter(|path| {
                        // Check if the path contains the root_dir
                        let contains_path = path.contains(&format!("/{}/", args.path)) || 
                                           path.ends_with(&format!("/{}", args.path));
                        contains_path
                    })
                    .collect::<Vec<String>>()
            } else {
                modules
            };
            
            if filtered_modules.is_empty() {
                logger::warning_box(
                    "No Matching Modules", 
                    &format!("No modules match the specified path: {}", args.path)
                );
                return Ok(());
            }
            
            logger::section("Modules to Apply");
            logger::list(&filtered_modules.iter().map(|s| s.split('/').last().unwrap_or(s)).collect::<Vec<_>>(), None);

            // Run terraform apply
            logger::step(3, 4, "Executing Terraform apply");
            logger::info(&format!("Applying {} modules with {} parallel jobs", filtered_modules.len(), args.parallel));
            
            match helpers::run_terraform_apply(&filtered_modules, dry_run, args.ignore_workspaces.as_deref(), args.var_files.as_deref(), settings.resolver(), watch, args.parallel) {
                Ok(_) => {
                    let duration = start_time.elapsed();
                    
                    if dry_run {
                        logger::success_box(
                            "Dry Run Complete", 
                            &format!("Successfully completed dry run for {} modules in {:.2}s", filtered_modules.len(), duration.as_secs_f64())
                        );
                    } else {
                        logger::success_box(
                            "Apply Complete", 
                            &format!("Successfully applied changes to {} modules in {:.2}s", filtered_modules.len(), duration.as_secs_f64())
                        );
                    }
                    
                    logger::results_summary("Apply Results", &[
                        ("Modules Applied", &filtered_modules.len().to_string()),
                        ("Duration", &format!("{:.2}s", duration.as_secs_f64())),
                        ("Parallel Jobs", &args.parallel.to_string()),
                        ("Mode", if dry_run { "Dry Run" } else { "Live Apply" }),
                    ]);
                }
                Err(e) => {
                    logger::error_box("Apply Failed", &format!("{}", e));
                    return Err(anyhow::anyhow!("{}", e));
                }
            }
        }
        Err(e) => {
            if let Some(progress) = progress {
                progress.complete(false);
            }
            logger::error_box("Module Detection Failed", &format!("Failed to get changed modules: {}", e));
            return Err(anyhow::anyhow!("Failed to get changed modules: {}", e));
        }
    }
    Ok(())
}
