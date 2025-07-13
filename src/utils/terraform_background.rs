use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum TerraformStatus {
    Initializing,
    Planning,
    Applying,
    Completed { success: bool },
    Failed { error: String },
}

#[derive(Debug)]
pub struct BackgroundTerraform {
    thread_handle: Option<thread::JoinHandle<()>>,
    status: Arc<Mutex<TerraformStatus>>,
    output: Arc<Mutex<Vec<String>>>,
}

impl BackgroundTerraform {
    pub fn new() -> Self {
        Self {
            thread_handle: None,
            status: Arc::new(Mutex::new(TerraformStatus::Initializing)),
            output: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_status(&self) -> TerraformStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn get_output(&self) -> Vec<String> {
        self.output.lock().unwrap().clone()
    }

    pub fn is_running(&mut self) -> bool {
        if let Some(handle) = &mut self.thread_handle {
            !handle.is_finished()
        } else {
            false
        }
    }

    pub fn init_background(&mut self, module_path: &str) -> Result<(), String> {
        let mut cmd = Command::new("terraform");
        cmd.arg("init")
           .current_dir(module_path)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to start terraform init: {}", e))?;

        let status = Arc::clone(&self.status);
        let output = Arc::clone(&self.output);

        // Take stdout and stderr before moving child
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // Spawn a thread to monitor the init process
        let child_handle = thread::spawn(move || {
            let stdout_reader = BufReader::new(stdout);
            let stderr_reader = BufReader::new(stderr);

            // Monitor stdout
            for line in stdout_reader.lines() {
                if let Ok(line) = line {
                    output.lock().unwrap().push(line.clone());
                    println!("  {}", line);
                }
            }

            // Monitor stderr
            for line in stderr_reader.lines() {
                if let Ok(line) = line {
                    output.lock().unwrap().push(format!("ERROR: {}", line));
                    eprintln!("  ERROR: {}", line);
                }
            }

            // Wait for process to complete
            let exit_status = child.wait().unwrap();
            
            if exit_status.success() {
                *status.lock().unwrap() = TerraformStatus::Completed { success: true };
            } else {
                *status.lock().unwrap() = TerraformStatus::Failed { 
                    error: "Terraform init failed".to_string() 
                };
            }
        });

        self.thread_handle = Some(child_handle);
        Ok(())
    }

    pub fn plan_background(&mut self, module_path: &str, var_files: Option<&[String]>) -> Result<(), String> {
        let mut cmd = Command::new("terraform");
        cmd.arg("plan")
           .current_dir(module_path)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        // Add var files if provided
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
                    let absolute_module = current_dir.join(module_path);
                    
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
                
                cmd.arg("-var-file").arg(&var_file_path);
            }
        }

        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to start terraform plan: {}", e))?;

        let status = Arc::clone(&self.status);
        let output = Arc::clone(&self.output);

        // Take stdout and stderr before moving child
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // Spawn a thread to monitor the plan process
        let child_handle = thread::spawn(move || {
            *status.lock().unwrap() = TerraformStatus::Planning;

            let stdout_reader = BufReader::new(stdout);
            let stderr_reader = BufReader::new(stderr);

            // Monitor stdout
            for line in stdout_reader.lines() {
                if let Ok(line) = line {
                    output.lock().unwrap().push(line.clone());
                    println!("  {}", line);
                }
            }

            // Monitor stderr
            for line in stderr_reader.lines() {
                if let Ok(line) = line {
                    output.lock().unwrap().push(format!("ERROR: {}", line));
                    eprintln!("  ERROR: {}", line);
                }
            }

            // Wait for process to complete
            let exit_status = child.wait().unwrap();
            
            if exit_status.success() {
                *status.lock().unwrap() = TerraformStatus::Completed { success: true };
            } else {
                *status.lock().unwrap() = TerraformStatus::Failed { 
                    error: "Terraform plan failed".to_string() 
                };
            }
        });

        // Store the thread handle instead of the child
        self.thread_handle = Some(child_handle);
        Ok(())
    }

    pub fn apply_background(&mut self, module_path: &str, var_files: Option<&[String]>) -> Result<(), String> {
        let mut cmd = Command::new("terraform");
        cmd.arg("apply")
           .arg("-auto-approve")
           .current_dir(module_path)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        // Add var files if provided
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
                    let absolute_module = current_dir.join(module_path);
                    
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
                
                cmd.arg("-var-file").arg(&var_file_path);
            }
        }

        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to start terraform apply: {}", e))?;

        let status = Arc::clone(&self.status);
        let output = Arc::clone(&self.output);

        // Take stdout and stderr before moving child
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // Spawn a thread to monitor the apply process
        let child_handle = thread::spawn(move || {
            *status.lock().unwrap() = TerraformStatus::Applying;

            let stdout_reader = BufReader::new(stdout);
            let stderr_reader = BufReader::new(stderr);

            // Monitor stdout
            for line in stdout_reader.lines() {
                if let Ok(line) = line {
                    output.lock().unwrap().push(line.clone());
                    println!("  {}", line);
                }
            }

            // Monitor stderr
            for line in stderr_reader.lines() {
                if let Ok(line) = line {
                    output.lock().unwrap().push(format!("ERROR: {}", line));
                    eprintln!("  ERROR: {}", line);
                }
            }

            // Wait for process to complete
            let exit_status = child.wait().unwrap();
            
            if exit_status.success() {
                *status.lock().unwrap() = TerraformStatus::Completed { success: true };
            } else {
                *status.lock().unwrap() = TerraformStatus::Failed { 
                    error: "Terraform apply failed".to_string() 
                };
            }
        });

        self.thread_handle = Some(child_handle);
        Ok(())
    }

    pub fn wait_for_completion(&mut self, timeout_seconds: u64) -> Result<bool, String> {
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_seconds);

        while self.is_running() {
            if start_time.elapsed() > timeout {
                return Err("Operation timed out".to_string());
            }
            thread::sleep(Duration::from_millis(100));
        }

        match self.get_status() {
            TerraformStatus::Completed { success } => Ok(success),
            TerraformStatus::Failed { error } => Err(error),
            _ => Err("Operation did not complete properly".to_string()),
        }
    }

    pub fn kill(&mut self) {
        // Note: We can't directly kill the child process anymore since it's in a thread
        // The thread will handle the process lifecycle
        if let Some(handle) = self.thread_handle.take() {
            // The thread will complete naturally when the process finishes
            let _ = handle.join();
        }
    }
}

pub fn run_terraform_silent(
    command: &str,
    args: &[&str],
    module_path: &str,
    var_files: Option<&[String]>,
) -> Result<bool, String> {
    let mut cmd = Command::new("terraform");
    cmd.arg(command)
       .args(args)
       .current_dir(module_path)
       .stdout(Stdio::null())
       .stderr(Stdio::null());

    // Add var files if provided
    if let Some(var_files) = var_files {
        for var_file in var_files {
            cmd.arg("-var-file").arg(var_file);
        }
    }

    let status = cmd.status()
        .map_err(|e| format!("Failed to execute terraform {}: {}", command, e))?;

    Ok(status.success())
} 
