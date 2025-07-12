use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for workspace-specific variable files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceVarFiles {
    /// Mapping of workspace names to their specific variable files
    #[serde(flatten)]
    pub workspaces: HashMap<String, Vec<String>>,
}

/// Global configuration settings applied to all modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Workspace names to ignore across all modules
    #[serde(default)]
    pub ignore_workspaces: Vec<String>,
    
    /// Variable files to use for all modules
    #[serde(default)]
    pub var_files: Vec<String>,
    
    /// Workspace-specific variable files for all modules
    #[serde(default)]
    pub workspace_var_files: Option<WorkspaceVarFiles>,
}

/// Module-specific configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    /// Workspace names to ignore for this specific module
    #[serde(default)]
    pub ignore_workspaces: Vec<String>,
    
    /// Variable files to use for this specific module
    #[serde(default)]
    pub var_files: Vec<String>,
    
    /// Workspace-specific variable files for this module
    #[serde(default)]
    pub workspace_var_files: Option<WorkspaceVarFiles>,
}

/// Root configuration structure for solarboat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarboatConfig {
    /// Global configuration settings
    #[serde(default)]
    pub global: GlobalConfig,
    
    /// Module-specific configuration settings
    #[serde(default)]
    pub modules: HashMap<String, ModuleConfig>,
}

impl Default for SolarboatConfig {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            modules: HashMap::new(),
        }
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            ignore_workspaces: Vec::new(),
            var_files: Vec::new(),
            workspace_var_files: None,
        }
    }
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self {
            ignore_workspaces: Vec::new(),
            var_files: Vec::new(),
            workspace_var_files: None,
        }
    }
}

impl WorkspaceVarFiles {
    /// Get variable files for a specific workspace
    pub fn get_workspace_files(&self, workspace: &str) -> Vec<String> {
        self.workspaces
            .get(workspace)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Check if a workspace has specific variable files
    pub fn has_workspace(&self, workspace: &str) -> bool {
        self.workspaces.contains_key(workspace)
    }
} 
