use std::process::{Command, Stdio};
use std::path::Path;
use regex::Regex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents a single terraform operation to be processed
#[derive(Debug, Clone)]
pub struct TerraformOperation {
    pub module_path: String,
    pub workspace: Option<String>,
    pub var_files: Vec<String>,
    pub operation_type: OperationType,
    pub watch: bool,
}

#[derive(Debug, Clone)]
pub enum OperationType {
    Init,
    Plan { plan_dir: Option<String> },
    Apply,
}

/// Result of a terraform operation
#[derive(Debug, Clone)]
pub struct OperationResult {
    pub module_path: String,
    pub workspace: Option<String>,
    pub operation_type: OperationType,
    pub success: bool,
    pub error: Option<String>,
    pub output: Vec<String>,
}

/// Select a terraform workspace
pub fn select_workspace(module_path: &str, workspace: &str) -> Result<(), String> {
    let mut cmd = Command::new("terraform");
    cmd.arg("workspace")
       .arg("select")
       .arg(workspace)
       .current_dir(module_path)
       .stdout(Stdio::null())
       .stderr(Stdio::null());

    let status = cmd.status()
        .map_err(|e| format!("Failed to select workspace {}: {}", workspace, e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Failed to select workspace {}", workspace))
    }
}

/// Save plan output to a markdown file
/// Uses naming convention: {module_name}-{workspace}-{timestamp}.tfplan.md
pub fn save_plan_output(module_path: &str, plan_dir: &str, workspace: Option<&str>, output_lines: &[String]) -> Result<(), String> {
    // Create the plan directory if it doesn't exist
    std::fs::create_dir_all(plan_dir)
        .map_err(|e| format!("Failed to create plan directory: {}", e))?;
        
    if let Some(module_name) = Path::new(module_path).file_name().and_then(|n| n.to_str()) {
        // Get current timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Failed to get timestamp: {}", e))?
            .as_secs();
        
        // Create filename with workspace and timestamp
        let workspace_name = workspace.unwrap_or("default");
        let filename = format!("{}-{}-{}.tfplan.md", module_name, workspace_name, timestamp);
        let plan_file = Path::new(plan_dir).join(filename);
        
        // Format the output
        let mut content = format!("# Terraform Plan Output for {} (workspace: {})\n\n", module_name, workspace_name);
        content.push_str("```\n");
        for line in output_lines {
            content.push_str(&clean_terraform_output(line));
            content.push('\n');
        }
        content.push_str("```\n");
        
        std::fs::write(&plan_file, content)
            .map_err(|e| format!("Failed to write plan file: {}", e))?;
    }

    Ok(())
}

/// Remove ANSI color codes from terraform output
pub fn clean_terraform_output(input: &str) -> String {
    // Remove ANSI color codes
    let re = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(input, "").to_string()
}

/// Run a single terraform plan operation
pub fn run_single_plan(module_path: &str, plan_dir: Option<&str>, workspace: Option<&str>, var_files: Option<&[String]>) -> Result<bool, String> {
    let mut cmd = Command::new("terraform");
    cmd.arg("plan").current_dir(module_path);
    
    if let Some(var_files) = var_files {
        for var_file in var_files {
            cmd.arg("-var-file").arg(var_file);
        }
    }

    let output = cmd.output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        return Ok(false);
    }

    // If plan_dir is specified, save the plan output
    if let Some(plan_dir) = plan_dir {
        let plan_output = String::from_utf8_lossy(&output.stdout).to_string();
        let output_lines: Vec<String> = plan_output.lines().map(|s| s.to_string()).collect();
        if let Err(e) = save_plan_output(module_path, plan_dir, workspace, &output_lines) {
            eprintln!("Warning: Failed to save plan output: {}", e);
        }
    }

    Ok(true)
}

/// Run a single terraform apply operation
pub fn run_single_apply(module_path: &str, var_files: Option<&[String]>) -> Result<bool, String> {
    let mut cmd = Command::new("terraform");
    cmd.arg("apply").arg("-auto-approve").current_dir(module_path);
    
    if let Some(var_files) = var_files {
        for var_file in var_files {
            cmd.arg("-var-file").arg(var_file);
        }
    }

    let status = cmd.status()
        .map_err(|e| e.to_string())?;

    Ok(status.success())
}
