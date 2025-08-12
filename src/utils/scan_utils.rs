use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Default)]
pub struct Module {
    depends_on: Vec<String>,
    used_by: Vec<String>,
    is_stateful: bool,
}

pub fn get_changed_modules(root_dir: &str, all: bool, default_branch: &str) -> Result<Vec<String>, String> {
    let mut modules = HashMap::new();

    // Always discover modules from the root directory
    discover_modules(root_dir, &mut modules)?;
    build_dependency_graph(&mut modules)?;

    if all {
        // If all is true, return all stateful modules
        let stateful_modules: Vec<String> = modules
            .iter()
            .filter(|(_, module)| module.is_stateful)
            .map(|(path, _)| path.clone())
            .collect();
        return Ok(stateful_modules);
    }

    // Check if we're on the main branch and handle accordingly
    let current_branch = get_current_branch(root_dir)?;
    let is_on_main = current_branch == default_branch;
    
    if is_on_main {
        println!("🔍 Currently on {} branch - using enhanced change detection", current_branch);
        let changed_files = get_main_branch_changes(root_dir)?;
        let affected_modules = process_changed_modules(&changed_files, &mut modules)?;
        
        // If no changes detected on main, provide helpful message
        if affected_modules.is_empty() {
            println!("ℹ️  No changes detected on main branch. This could mean:");
            println!("   • No recent commits with .tf changes");
            println!("   • Changes were already applied");
            println!("   • Use --all flag to process all modules");
        }
        
        return Ok(affected_modules);
    }

    // Regular change detection for non-main branches
    let changed_files = get_git_changed_files(".", default_branch)?;
    let affected_modules = process_changed_modules(&changed_files, &mut modules)?;

    // If root_dir is not ".", filter modules based on the root_dir path
    if root_dir != "." {
        println!("🔍 Filtering modules with path: {}", root_dir);
        
        // Filter the affected modules to only include those matching the path
        let filtered_modules: Vec<String> = affected_modules
            .into_iter()
            .filter(|path| {
                // Check if the path contains the root_dir
                let contains_path = path.contains(&format!("/{}/", root_dir)) || 
                                   path.ends_with(&format!("/{}", root_dir));
                
                // Don't print anything for keeping or filtering modules
                contains_path
            })
            .collect();
            
        return Ok(filtered_modules);
    }
    
    // Otherwise return all affected modules without filtering
    Ok(affected_modules)
}

pub fn discover_modules(root_dir: &str, modules: &mut HashMap<String, Module>) -> Result<(), String> {
    for entry in fs::read_dir(root_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.is_dir() {
            // Recursively search subdirectories
            discover_modules(path.to_str().ok_or("Invalid path")?, modules)?;

            let tf_files: Vec<_> = fs::read_dir(&path)
                .map_err(|e| e.to_string())?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "tf"))
                .collect();

            if !tf_files.is_empty() {
                let abs_path = fs::canonicalize(&path).map_err(|e| e.to_string())?;
                let abs_path_str = abs_path.to_str().ok_or("Invalid path")?.to_string();

                modules.entry(abs_path_str.clone()).or_insert(Module {
                    is_stateful: has_backend_config(&tf_files),
                    ..Default::default()
                });
            }
        }
    }
    Ok(())
}

pub fn build_dependency_graph(modules: &mut HashMap<String, Module>) -> Result<(), String> {
    let dependencies = collect_dependencies(modules)?;

    for (path, dep) in dependencies {
        if let Some(module) = modules.get_mut(&path) {
            module.depends_on.push(dep.clone());
        }
        if let Some(dep_module) = modules.get_mut(&dep) {
            dep_module.used_by.push(path.clone());
        }
    }

    println!("🔗 Building dependency graph...");
    // Don't print the full module details, just the count
    println!("🔍 Found {} modules repo-wide", modules.len());
    Ok(())
}

pub fn collect_dependencies(modules: &HashMap<String, Module>) -> Result<Vec<(String, String)>, String> {
    let mut dependencies = Vec::new();

    for (path, _module) in modules {
        let tf_files: Vec<_> = fs::read_dir(path)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "tf"))
            .collect();

        for file in tf_files {
            let content = fs::read_to_string(file.path()).map_err(|e| e.to_string())?;
            let deps = find_module_dependencies(&content, path);

            for dep in deps {
                dependencies.push((path.clone(), dep));
            }
        }
    }

    Ok(dependencies)
}

pub fn find_module_dependencies(content: &str, current_dir: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut in_module_block = false;

    for line in lines {
        let trimmed_line = line.trim();

        if trimmed_line.starts_with("module") && trimmed_line.contains("{") {
            in_module_block = true;
            continue;
        }

        if in_module_block {
            if trimmed_line.contains("source") {
                let parts: Vec<&str> = trimmed_line.split('=').collect();
                if parts.len() == 2 {
                    let source = parts[1].trim().trim_matches(|c| c == '"' || c == '\'');
                    let module_path = Path::new(current_dir).join(source);
                    if let Ok(abs_path) = fs::canonicalize(module_path) {
                        if let Some(abs_path_str) = abs_path.to_str() {
                            deps.push(abs_path_str.to_string());
                        }
                    }
                }
            }
            if trimmed_line.contains("}") {
                in_module_block = false;
            }
        }
    }
    deps
}

pub fn has_backend_config(tf_files: &[fs::DirEntry]) -> bool {
    // Check if this module refers to other modules (has module blocks)
    let has_module_blocks = tf_files.iter().any(|file| {
        if let Ok(content) = fs::read_to_string(file.path()) {
            let lines: Vec<&str> = content.lines().collect();
            for line in lines {
                let trimmed_line = line.trim();
                if trimmed_line.starts_with("module") && trimmed_line.contains("{") {
                    return true;
                }
            }
        }
        false
    });
    
    if has_module_blocks {
        return true; // This module refers to other modules, so it's stateful
    }
    
    // Check if this module has a remote backend or local state files
    for file in tf_files {
        if let Ok(content) = fs::read_to_string(file.path()) {
            let lines: Vec<&str> = content.lines().collect();
            let mut in_terraform_block = false;
            let mut brace_count = 0;
            
            for line in lines {
                let trimmed_line = line.trim();
                
                // Skip empty lines and comments
                if trimmed_line.is_empty() || trimmed_line.starts_with('#') || trimmed_line.starts_with("//") {
                    continue;
                }
                
                // Check for terraform block start
                if trimmed_line.starts_with("terraform") && trimmed_line.contains("{") {
                    in_terraform_block = true;
                    brace_count += 1;
                    continue;
                }
                
                // Check for backend block start while in terraform block
                if in_terraform_block && trimmed_line.starts_with("backend") && trimmed_line.contains("\"") {
                    return true; // Found a backend block, this is a stateful module
                }
                
                // Count braces to track block nesting
                if trimmed_line.contains("{") {
                    brace_count += 1;
                }
                if trimmed_line.contains("}") {
                    brace_count -= 1;
                    if brace_count == 0 {
                        in_terraform_block = false;
                    }
                }
            }
        }
    }
    
    // Check for local state files
    if let Some(first_file) = tf_files.first() {
        if let Some(dir_path) = first_file.path().parent() {
            if let Ok(entries) = fs::read_dir(dir_path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "tfstate") {
                        return true; // Found a local state file, this is a stateful module
                    }
                }
            }
        }
    }
    
    // If we didn't find module blocks, backend blocks, or state files, this is a stateless module
    false
}

/// Get the current branch name
fn get_current_branch(root_dir: &str) -> Result<String, String> {
    // Try to get from environment first (for CI/CD)
    if let Ok(branch) = std::env::var("GITHUB_REF_NAME") {
        return Ok(branch);
    }
    
    // Fallback to git command
    let output = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(root_dir)
        .output()
        .map_err(|e| e.to_string())?;
        
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err("Failed to get current branch".to_string())
    }
}

/// Get changes specifically for main branch scenarios
fn get_main_branch_changes(root_dir: &str) -> Result<Vec<String>, String> {
    // Strategy 1: Check recent commits (last 5 commits)
    let recent_changes = get_recent_commit_changes(root_dir, 5)?;
    if !recent_changes.is_empty() {
        println!("🔍 Found changes in recent commits");
        return Ok(recent_changes);
    }
    
    // Strategy 2: Check if there are any staged or unstaged changes
    let uncommitted_changes = get_uncommitted_changes(root_dir)?;
    if !uncommitted_changes.is_empty() {
        println!("🔍 Found uncommitted changes");
        return Ok(uncommitted_changes);
    }
    
    // Strategy 3: Compare with a reference point (e.g., last tag or specific commit)
    let reference_changes = get_reference_changes(root_dir)?;
    if !reference_changes.is_empty() {
        println!("🔍 Found changes compared to reference point");
        return Ok(reference_changes);
    }
    
    println!("🔍 No changes detected using any strategy");
    Ok(Vec::new())
}

/// Get changes from recent commits
fn get_recent_commit_changes(root_dir: &str, commit_count: usize) -> Result<Vec<String>, String> {
    let mut changed_files = Vec::new();
    
    // Get the last N commits
    let log_output = Command::new("git")
        .args(&["log", "--oneline", "-n", &commit_count.to_string()])
        .current_dir(root_dir)
        .output()
        .map_err(|e| e.to_string())?;
        
    if !log_output.status.success() {
        return Ok(Vec::new());
    }
    
    let log_output_str = String::from_utf8_lossy(&log_output.stdout);
    let commits: Vec<&str> = log_output_str
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    
    // Check changes in each commit
    for commit in commits {
        let changes = get_changes_between_commits(root_dir, &format!("{}~1", commit), commit)?;
        changed_files.extend(changes);
    }
    
    // Remove duplicates
    changed_files.sort();
    changed_files.dedup();
    
    Ok(changed_files)
}

/// Get uncommitted changes (staged and unstaged)
fn get_uncommitted_changes(root_dir: &str) -> Result<Vec<String>, String> {
    let mut changed_files = Vec::new();
    
    // Get staged changes
    let staged_output = Command::new("git")
        .args(&["diff", "--cached", "--name-only"])
        .current_dir(root_dir)
        .output()
        .map_err(|e| e.to_string())?;
        
    if staged_output.status.success() {
        changed_files.extend(
            String::from_utf8_lossy(&staged_output.stdout)
                .lines()
                .filter(|line| line.ends_with(".tf"))
                .map(|line| Path::new(root_dir).join(line).to_string_lossy().to_string())
        );
    }
    
    // Get unstaged changes
    let unstaged_output = Command::new("git")
        .args(&["diff", "--name-only"])
        .current_dir(root_dir)
        .output()
        .map_err(|e| e.to_string())?;
        
    if unstaged_output.status.success() {
        changed_files.extend(
            String::from_utf8_lossy(&unstaged_output.stdout)
                .lines()
                .filter(|line| line.ends_with(".tf"))
                .map(|line| Path::new(root_dir).join(line).to_string_lossy().to_string())
        );
    }
    
    // Remove duplicates
    changed_files.sort();
    changed_files.dedup();
    
    Ok(changed_files)
}

/// Get changes compared to a reference point (last tag or specific commit)
fn get_reference_changes(root_dir: &str) -> Result<Vec<String>, String> {
    // Try to find the last tag
    let tag_output = Command::new("git")
        .args(&["describe", "--tags", "--abbrev=0"])
        .current_dir(root_dir)
        .output();
        
    if let Ok(output) = tag_output {
        if output.status.success() {
            let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("🔍 Comparing with last tag: {}", tag);
            return get_changes_between_commits(root_dir, &tag, "HEAD");
        }
    }
    
    // Fallback: compare with a commit from 1 day ago
    let date_output = Command::new("git")
        .args(&["rev-list", "-n", "1", "--before=1 day ago", "HEAD"])
        .current_dir(root_dir)
        .output()
        .map_err(|e| e.to_string())?;
        
    if date_output.status.success() {
        let commit = String::from_utf8_lossy(&date_output.stdout).trim().to_string();
        if !commit.is_empty() {
            println!("🔍 Comparing with commit from 1 day ago: {}", commit);
            return get_changes_between_commits(root_dir, &commit, "HEAD");
        }
    }
    
    Ok(Vec::new())
}

/// Get changes between two specific commits
fn get_changes_between_commits(root_dir: &str, from_commit: &str, to_commit: &str) -> Result<Vec<String>, String> {
    let mut changed_files = Vec::new();

    println!("🔍 Getting changes between {} and {}", from_commit, to_commit);
    
    // Get changes between the two commits
    let diff_output = Command::new("git")
        .args(&["diff", "--name-only", from_commit, to_commit])
        .current_dir(root_dir)
        .output()
        .map_err(|e| e.to_string())?;

    if diff_output.status.success() {
        changed_files.extend(
            String::from_utf8_lossy(&diff_output.stdout)
                .lines()
                .filter(|line| line.ends_with(".tf"))
                .map(|line| {
                    // Use a more robust approach to handle paths that might not exist
                    let file_path = Path::new(root_dir).join(line);
                    if file_path.exists() {
                        // If the file exists, canonicalize it
                        fs::canonicalize(file_path)
                            .map_err(|e| e.to_string())
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string()
                    } else {
                        // If the file doesn't exist, use the absolute path from the current directory
                        let current_dir = std::env::current_dir().map_err(|e| e.to_string()).unwrap();
                        current_dir.join(root_dir).join(line)
                            .to_str()
                            .unwrap()
                            .to_string()
                    }
                })
        );
    }

    // Remove duplicates
    changed_files.sort();
    changed_files.dedup();

    if !changed_files.is_empty() {
        println!("🔍 Found {} changed .tf files:", changed_files.len());
        for file in &changed_files {
            println!("   • {}", file);
        }
    } else {
        println!("🔍 No .tf files changed between the commits");
    }

    Ok(changed_files)
}

pub fn get_git_changed_files(root_dir: &str, default_branch: &str) -> Result<Vec<String>, String> {
    // First, try to get the merge-base with origin/{default_branch}
    let merge_base_output = Command::new("git")
        .args(&["merge-base", &format!("origin/{}", default_branch), "HEAD"])
        .current_dir(root_dir)
        .output()
        .map_err(|e| e.to_string())?;

    let merge_base = if merge_base_output.status.success() {
        String::from_utf8_lossy(&merge_base_output.stdout).trim().to_string()
    } else {
        // If origin/{default_branch} is not available, try with local {default_branch}
        let local_merge_base = Command::new("git")
            .args(&["merge-base", default_branch, "HEAD"])
            .current_dir(root_dir)
            .output()
            .map_err(|e| e.to_string())?;
            
        if !local_merge_base.status.success() {
            // If we can't find a merge base, return an empty list
            return Ok(Vec::new());
        }
        String::from_utf8_lossy(&local_merge_base.stdout).trim().to_string()
    };

    // Get both staged and unstaged changes
    let mut changed_files = Vec::new();

    // Get uncommitted changes
    let status_output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(root_dir)
        .output()
        .map_err(|e| e.to_string())?;

    if status_output.status.success() {
        changed_files.extend(
            String::from_utf8_lossy(&status_output.stdout)
                .lines()
                .filter(|line| line.ends_with(".tf"))
                .map(|line| {
                    let file = line[3..].trim();
                    // Use a more robust approach to handle paths that might not exist
                    let file_path = Path::new(root_dir).join(file);
                    if file_path.exists() {
                        // If the file exists, canonicalize it
                        fs::canonicalize(file_path)
                            .map_err(|e| e.to_string())
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string()
                    } else {
                        // If the file doesn't exist, use the absolute path from the current directory
                        let current_dir = std::env::current_dir().map_err(|e| e.to_string()).unwrap();
                        current_dir.join(root_dir).join(file)
                            .to_str()
                            .unwrap()
                            .to_string()
                    }
                })
        );
    }

    // Get changes between current branch and merge-base
    let diff_output = Command::new("git")
        .args(&["diff", "--name-only", &merge_base])
        .current_dir(root_dir)
        .output()
        .map_err(|e| e.to_string())?;

    if diff_output.status.success() {
        changed_files.extend(
            String::from_utf8_lossy(&diff_output.stdout)
                .lines()
                .filter(|line| line.ends_with(".tf"))
                .map(|line| {
                    // Use a more robust approach to handle paths that might not exist
                    let file_path = Path::new(root_dir).join(line);
                    if file_path.exists() {
                        // If the file exists, canonicalize it
                        fs::canonicalize(file_path)
                            .map_err(|e| e.to_string())
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string()
                    } else {
                        // If the file doesn't exist, use the absolute path from the current directory
                        let current_dir = std::env::current_dir().map_err(|e| e.to_string()).unwrap();
                        current_dir.join(root_dir).join(line)
                            .to_str()
                            .unwrap()
                            .to_string()
                    }
                })
        );
    }

    // Remove duplicates
    changed_files.sort();
    changed_files.dedup();

    Ok(changed_files)
}

pub fn process_changed_modules(changed_files: &[String], modules: &mut HashMap<String, Module>) -> Result<Vec<String>, String> {
    let mut affected_modules = Vec::new();
    let mut processed = HashMap::new();

    // Collect all module paths first
    let module_paths: Vec<String> = modules.keys().cloned().collect();

    // For each changed file, find the module it belongs to
    for file in changed_files {
        let file_path = Path::new(file);
        
        // Find the module this file belongs to
        for module_path in &module_paths {
            let module_path = Path::new(module_path);
            
            // Check if the file is in this module or a subdirectory of it
            if file_path.starts_with(module_path) {
                mark_module_changed(module_path.to_str().unwrap(), modules, &mut affected_modules, &mut processed);
                break;
            }
        }
    }

    Ok(affected_modules)
}

pub fn mark_module_changed(module_path: &str, all_modules: &mut HashMap<String, Module>, affected_modules: &mut Vec<String>, processed: &mut HashMap<String, bool>) {
    if *processed.get(module_path).unwrap_or(&false) {
        return;
    }
    processed.insert(module_path.to_string(), true);

    if let Some(module) = all_modules.get(module_path) {
        if module.is_stateful {
            // Add this stateful module to affected modules if not already added
            if !affected_modules.contains(&module_path.to_string()) {
                affected_modules.push(module_path.to_string());
            }
            
            // We no longer mark dependents as changed
            // This ensures only directly changed modules are included
        } else {
            // For stateless modules, we need to check if they are used by any stateful modules
            // If so, we mark those stateful modules as changed as well
            if !module.used_by.is_empty() {
                println!("🔄 Stateless module with changes: {}", module_path.split('/').last().unwrap_or(module_path));
                
                // Check all modules that use this stateless module
                for user_module_path in &module.used_by {
                    if let Some(user_module) = all_modules.get(user_module_path) {
                        if user_module.is_stateful {
                            // Mark this stateful module as affected since it uses a changed stateless module
                            // Only add and print if not already in the list
                            if !affected_modules.contains(user_module_path) {
                                println!("🔄 Adding stateful module that uses changed stateless module: {}", 
                                         user_module_path.split('/').last().unwrap_or(user_module_path));
                                affected_modules.push(user_module_path.clone());
                            }
                        }
                    }
                }
            }
        }
    }
}
