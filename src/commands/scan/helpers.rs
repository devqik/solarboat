use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Default)]
pub struct Module {
    path: String,
    depends_on: Vec<String>,
    used_by: Vec<String>,
    is_stateful: bool,
}

pub fn get_changed_modules(root_dir: &str, force: bool) -> Result<Vec<String>, String> {
    let mut modules = HashMap::new();

    discover_modules(root_dir, &mut modules)?;
    build_dependency_graph(&mut modules)?;

    if force {
        // If force is true, return all stateful modules
        let stateful_modules: Vec<String> = modules
            .iter()
            .filter(|(_, module)| module.is_stateful)
            .map(|(path, _)| path.clone())
            .collect();
        return Ok(stateful_modules);
    }

    let changed_files = get_git_changed_files(root_dir)?;
    let affected_modules = process_changed_modules(&changed_files, &mut modules)?;

    Ok(affected_modules)
}

pub fn discover_modules(root_dir: &str, modules: &mut HashMap<String, Module>) -> Result<(), String> {
    for entry in fs::read_dir(root_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.is_dir() {
            let tf_files: Vec<_> = fs::read_dir(&path)
                .map_err(|e| e.to_string())?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "tf"))
                .collect();

            if !tf_files.is_empty() {
                let abs_path = fs::canonicalize(&path).map_err(|e| e.to_string())?;
                let abs_path_str = abs_path.to_str().ok_or("Invalid path")?.to_string();

                modules.entry(abs_path_str.clone()).or_insert(Module {
                    path: abs_path_str,
                    is_stateful: !has_backend_config(&tf_files),
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
    println!("---------------------------------");
    println!("{:?}", modules);
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
    for file in tf_files {
        if let Ok(content) = fs::read_to_string(file.path()) {
            let lines: Vec<&str> = content.lines().collect();
            let mut in_terraform_block = false;

            for line in lines {
                let trimmed_line = line.trim();
                
                if trimmed_line.starts_with("terraform") && trimmed_line.contains("{") {
                    in_terraform_block = true;
                    continue;
                }

                if in_terraform_block {
                    if trimmed_line.starts_with("backend") && trimmed_line.contains("\"") {
                        return true;
                    }
                    if trimmed_line == "}" {
                        in_terraform_block = false;
                    }
                }
            }
        }
    }
    false
}

pub fn get_git_changed_files(root_dir: &str) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(root_dir)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("Failed to get git status".to_string());
    }

    let changed_files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.ends_with(".tf"))
        .map(|line| {
            let file = line[3..].trim();
            fs::canonicalize(Path::new(root_dir).join(file))
                .map_err(|e| e.to_string())
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        })
        .collect();

    Ok(changed_files)
}

pub fn process_changed_modules(changed_files: &[String], modules: &mut HashMap<String, Module>) -> Result<Vec<String>, String> {
    let mut affected_modules = Vec::new();
    let mut processed = HashMap::new();

    let module_dirs: Vec<String> = changed_files.iter()
        .filter_map(|file| Path::new(file).parent().and_then(|p| p.to_str()).map(String::from))
        .collect();

    for module_dir in module_dirs {
        if let Some(module) = modules.get(&module_dir) {
            let path = module.path.clone();
            mark_module_changed(&path, modules, &mut affected_modules, &mut processed);
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
            let user_paths: Vec<String> = module.used_by.clone();
            for user_path in user_paths {
                mark_module_changed(&user_path, all_modules, affected_modules, processed);
            }
        } else {
            affected_modules.push(module_path.to_string());
        }
    }
}
