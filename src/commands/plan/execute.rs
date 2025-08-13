use crate::cli::PlanArgs;
use crate::config::Settings;
use crate::utils::logger;
use super::helpers;
use std::fs;
use std::path::Path;
use std::time::Instant;

pub fn execute(args: PlanArgs, settings: &Settings) -> anyhow::Result<()> {
    let start_time = Instant::now();
    
    // Parse boolean strings
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

    let output_dir = args.output_dir.as_deref().unwrap_or("terraform-plans");
    let output_path = Path::new(output_dir);

    // Show configuration summary
    logger::config_summary(&[
        ("Plan Path", &args.path),
        ("Output Directory", output_dir),
        ("Default Branch", &args.default_branch),
        ("Recent Commits", &args.recent_commits.to_string()),
        ("Process All", &all.to_string()),
        ("Watch Mode", &watch.to_string()),
        ("Parallel Jobs", &args.parallel.to_string()),
    ]);

    // Setup output directory
    logger::step(1, 4, "Setting up output directory");
    if output_path.exists() {
        logger::info(&format!("Using existing output directory: {}", output_dir));
    } else {
        logger::info(&format!("Creating output directory: {}", output_dir));
        fs::create_dir_all(output_dir)?;
    }

    // Get changed modules
    logger::step(2, 4, "Detecting changed modules");
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
                    "All stateful modules will be planned regardless of changes"
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
            logger::step(3, 4, "Filtering modules by path");
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
            
            logger::section("Modules to Plan");
            logger::list(&filtered_modules.iter().map(|s| s.split('/').last().unwrap_or(s)).collect::<Vec<_>>(), None);
            
            // Run terraform plan
            logger::step(4, 4, "Executing Terraform plans");
            logger::info(&format!("Planning {} modules with {} parallel jobs", filtered_modules.len(), args.parallel));
            
            match helpers::run_terraform_plan(&filtered_modules, Some(output_dir), args.ignore_workspaces.as_deref(), args.var_files.as_deref(), settings.resolver(), watch, args.parallel) {
                Ok(_) => {
                    let duration = start_time.elapsed();
                    logger::success_box(
                        "Plan Complete", 
                        &format!("Successfully generated plans for {} modules in {:.2}s", filtered_modules.len(), duration.as_secs_f64())
                    );
                    
                    logger::results_summary("Plan Results", &[
                        ("Modules Planned", &filtered_modules.len().to_string()),
                        ("Output Directory", output_dir),
                        ("Duration", &format!("{:.2}s", duration.as_secs_f64())),
                        ("Parallel Jobs", &args.parallel.to_string()),
                    ]);
                }
                Err(e) => {
                    logger::error_box("Plan Failed", &format!("Terraform plan failed: {}", e));
                    return Err(anyhow::anyhow!("Terraform plan failed: {}", e));
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
