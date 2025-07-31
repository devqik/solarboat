use std::path::{Path, PathBuf};
use std::env;

/// Convert an absolute module path to a relative path for display purposes
/// This makes CLI output cleaner by showing paths relative to the current working directory
pub fn format_module_path(module_path: &str) -> String {
    let path = Path::new(module_path);
    
    // Try to get current working directory
    if let Ok(current_dir) = env::current_dir() {
        // If the module path is under the current directory, make it relative
        if let Ok(relative_path) = path.strip_prefix(&current_dir) {
            return relative_path.to_string_lossy().to_string();
        }
    }
    
    // If we can't make it relative, try to show just the meaningful part
    // by removing common prefixes like /Users/username/...
    let path_str = path.to_string_lossy();
    
    // Find the last occurrence of common project indicators
    for indicator in &["terraform", "infrastructure", "modules"] {
        if let Some(pos) = path_str.rfind(indicator) {
            return path_str[pos..].to_string();
        }
    }
    
    // Fall back to just the last few components
    let components: Vec<_> = path.components().collect();
    if components.len() > 3 {
        let last_three: PathBuf = components[components.len()-3..].iter().collect();
        return last_three.to_string_lossy().to_string();
    }
    
    // Last resort: return the original path
    module_path.to_string()
}

/// Format workspace name for display
pub fn format_workspace(workspace: Option<&str>) -> String {
    workspace.unwrap_or("default").to_string()
}

/// Format a list of workspaces for display
pub fn format_workspace_list(workspaces: &[String]) -> String {
    if workspaces.len() <= 3 {
        format!("{:?}", workspaces)
    } else {
        format!("{:?} (+{} more)", &workspaces[..3], workspaces.len() - 3)
    }
}

/// Create a compact status line for module processing
pub fn format_module_status(module_path: &str, workspace: Option<&str>, status: &str) -> String {
    let display_path = format_module_path(module_path);
    let display_workspace = format_workspace(workspace);
    
    if workspace.is_some() && workspace != Some("default") {
        format!("📦 {} ({}): {}", display_path, display_workspace, status)
    } else {
        format!("📦 {}: {}", display_path, status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_format_module_path_relative() {
        // This test will work when run from the project directory
        let current_dir = env::current_dir().unwrap();
        let test_path = current_dir.join("terraform/projects/test");
        let formatted = format_module_path(&test_path.to_string_lossy());
        assert!(formatted.contains("terraform/projects/test"));
    }

    #[test]
    fn test_format_workspace() {
        assert_eq!(format_workspace(Some("staging")), "staging");
        assert_eq!(format_workspace(None), "default");
    }

    #[test]
    fn test_format_workspace_list() {
        assert_eq!(format_workspace_list(&["default".to_string()]), "[\"default\"]");
        let many: Vec<String> = (0..5).map(|i| format!("workspace{}", i)).collect();
        let result = format_workspace_list(&many);
        assert!(result.contains("(+2 more)"));
    }
} 
