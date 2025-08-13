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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Workspaces to ignore globally
    #[serde(default)]
    pub ignore_workspaces: Vec<String>,
    /// Global workspace variable file mappings
    pub workspace_var_files: Option<WorkspaceVarFiles>,
}

/// Module-specific configuration settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModuleConfig {
    /// Workspaces to ignore for this module
    #[serde(default)]
    pub ignore_workspaces: Vec<String>,
    /// Module-specific workspace variable file mappings
    pub workspace_var_files: Option<WorkspaceVarFiles>,
}

/// Root configuration structure for solarboat
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SolarboatConfig {
    /// Global configuration settings
    #[serde(default)]
    pub global: GlobalConfig,
    /// Module-specific configurations
    #[serde(default)]
    pub modules: HashMap<String, ModuleConfig>,
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
