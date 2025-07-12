use crate::config::types::{GlobalConfig, ModuleConfig, SolarboatConfig, WorkspaceVarFiles};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Resolved configuration for a specific module and workspace
#[derive(Debug, Clone)]
pub struct ResolvedModuleConfig {
    /// Workspaces to ignore for this module
    pub ignore_workspaces: Vec<String>,
    /// Variable files to use for this module and workspace
    pub var_files: Vec<String>,
}

/// Configuration resolver that merges CLI arguments with configuration file settings
pub struct ConfigResolver {
    /// The base configuration loaded from file
    config: Option<SolarboatConfig>,
    /// The directory where the configuration file was loaded from
    config_dir: PathBuf,
}

impl ConfigResolver {
    /// Create a new ConfigResolver with optional configuration
    pub fn new(config: Option<SolarboatConfig>, config_dir: PathBuf) -> Self {
        Self { config, config_dir }
    }
    
    /// Resolve configuration for a specific module
    pub fn resolve_module_config(
        &self,
        module_path: &str,
        cli_ignore_workspaces: Option<&[String]>,
        cli_var_files: Option<&[String]>,
    ) -> ResolvedModuleConfig {
        let mut resolved = ResolvedModuleConfig {
            ignore_workspaces: Vec::new(),
            var_files: Vec::new(),
        };
        
        // Get module-specific and global configurations
        let module_config = self.get_module_config(module_path);
        let global_config = self.get_global_config();
        
        // Resolve ignore workspaces (CLI > module > global)
        resolved.ignore_workspaces = self.resolve_ignore_workspaces(
            cli_ignore_workspaces,
            &module_config.ignore_workspaces,
            &global_config.ignore_workspaces,
        );
        
        // Resolve general var files (CLI > module > global)
        resolved.var_files = self.resolve_var_files(
            cli_var_files,
            &module_config.var_files,
            &global_config.var_files,
        );
        
        resolved
    }
    
    /// Get final var files for a specific module and workspace
    pub fn get_workspace_var_files(
        &self,
        module_path: &str,
        workspace: &str,
        cli_var_files: Option<&[String]>,
    ) -> Vec<String> {
        let mut var_files = Vec::new();
        
        // Start with general var files
        let module_config = self.get_module_config(module_path);
        let global_config = self.get_global_config();
        
        let general_var_files = self.resolve_var_files(
            cli_var_files,
            &module_config.var_files,
            &global_config.var_files,
        );
        var_files.extend(general_var_files);
        
        // Add workspace-specific var files
        let workspace_var_files = self.resolve_workspace_var_files(
            module_path,
            workspace,
        );
        var_files.extend(workspace_var_files);
        
        // Resolve relative paths relative to config file location
        var_files = self.resolve_var_file_paths(&var_files);
        
        var_files
    }
    
    /// Resolve ignore workspaces with proper precedence
    fn resolve_ignore_workspaces(
        &self,
        cli_ignore: Option<&[String]>,
        module_ignore: &[String],
        global_ignore: &[String],
    ) -> Vec<String> {
        // CLI arguments override everything
        if let Some(cli_ignore) = cli_ignore {
            return cli_ignore.to_vec();
        }
        
        // Module-specific overrides global
        if !module_ignore.is_empty() {
            return module_ignore.to_vec();
        }
        
        // Fall back to global
        global_ignore.to_vec()
    }
    
    /// Resolve var files with proper precedence
    fn resolve_var_files(
        &self,
        cli_var_files: Option<&[String]>,
        module_var_files: &[String],
        global_var_files: &[String],
    ) -> Vec<String> {
        // CLI arguments override everything
        if let Some(cli_var_files) = cli_var_files {
            return cli_var_files.to_vec();
        }
        
        // Module-specific overrides global
        if !module_var_files.is_empty() {
            return module_var_files.to_vec();
        }
        
        // Fall back to global
        global_var_files.to_vec()
    }
    
    /// Resolve workspace-specific var files
    fn resolve_workspace_var_files(&self, module_path: &str, workspace: &str) -> Vec<String> {
        let module_config = self.get_module_config(module_path);
        let global_config = self.get_global_config();
        
        // Try module-specific workspace var files first
        if let Some(module_workspace_files) = &module_config.workspace_var_files {
            if module_workspace_files.has_workspace(workspace) {
                return module_workspace_files.get_workspace_files(workspace);
            }
        }
        
        // Fall back to global workspace var files
        if let Some(global_workspace_files) = &global_config.workspace_var_files {
            if global_workspace_files.has_workspace(workspace) {
                return global_workspace_files.get_workspace_files(workspace);
            }
        }
        
        Vec::new()
    }
    
    /// Resolve var file paths relative to config file location
    fn resolve_var_file_paths(&self, var_files: &[String]) -> Vec<String> {
        var_files
            .iter()
            .map(|var_file| {
                if Path::new(var_file).is_absolute() {
                    var_file.clone()
                } else {
                    // Make path relative to config file location
                    self.config_dir.join(var_file).to_string_lossy().to_string()
                }
            })
            .collect()
    }
    
    /// Get module-specific configuration
    fn get_module_config(&self, module_path: &str) -> ModuleConfig {
        self.config
            .as_ref()
            .and_then(|config| config.modules.get(module_path))
            .cloned()
            .unwrap_or_default()
    }
    
    /// Get global configuration
    fn get_global_config(&self) -> GlobalConfig {
        self.config
            .as_ref()
            .map(|config| config.global.clone())
            .unwrap_or_default()
    }
    
    /// Check if a workspace should be ignored for a module
    pub fn should_ignore_workspace(
        &self,
        module_path: &str,
        workspace: &str,
        cli_ignore_workspaces: Option<&[String]>,
    ) -> bool {
        let resolved_config = self.resolve_module_config(module_path, cli_ignore_workspaces, None);
        resolved_config.ignore_workspaces.contains(&workspace.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::SolarboatConfig;
    
    fn create_test_config() -> SolarboatConfig {
        let mut global_workspace_files = WorkspaceVarFiles {
            workspaces: HashMap::new(),
        };
        global_workspace_files.workspaces.insert("prod".to_string(), vec!["global-prod.tfvars".to_string()]);
        
        let mut module_workspace_files = WorkspaceVarFiles {
            workspaces: HashMap::new(),
        };
        module_workspace_files.workspaces.insert("prod".to_string(), vec!["module-prod.tfvars".to_string()]);
        
        let mut modules = HashMap::new();
        modules.insert(
            "infrastructure/networking".to_string(),
            ModuleConfig {
                ignore_workspaces: vec!["dev".to_string()],
                var_files: vec!["networking.tfvars".to_string()],
                workspace_var_files: Some(module_workspace_files),
            },
        );
        
        SolarboatConfig {
            global: GlobalConfig {
                ignore_workspaces: vec!["test".to_string()],
                var_files: vec!["global.tfvars".to_string()],
                workspace_var_files: Some(global_workspace_files),
            },
            modules,
        }
    }
    
    #[test]
    fn test_resolve_module_config() {
        let config = create_test_config();
        let resolver = ConfigResolver::new(Some(config), PathBuf::from("/tmp"));
        
        let resolved = resolver.resolve_module_config(
            "infrastructure/networking",
            None,
            None,
        );
        
        assert_eq!(resolved.ignore_workspaces, vec!["dev"]);
        assert_eq!(resolved.var_files, vec!["networking.tfvars"]);
    }
    
    #[test]
    fn test_cli_overrides_config() {
        let config = create_test_config();
        let resolver = ConfigResolver::new(Some(config), PathBuf::from("/tmp"));
        
        let resolved = resolver.resolve_module_config(
            "infrastructure/networking",
            Some(&["cli-ignore".to_string()]),
            Some(&["cli-var.tfvars".to_string()]),
        );
        
        assert_eq!(resolved.ignore_workspaces, vec!["cli-ignore"]);
        assert_eq!(resolved.var_files, vec!["cli-var.tfvars"]);
    }
    
    #[test]
    fn test_workspace_var_files() {
        let config = create_test_config();
        let resolver = ConfigResolver::new(Some(config), PathBuf::from("/tmp"));
        
        let var_files = resolver.get_workspace_var_files(
            "infrastructure/networking",
            "prod",
            None,
        );
        
        // Should include both general and workspace-specific files
        assert!(var_files.contains(&"/tmp/networking.tfvars".to_string()));
        assert!(var_files.contains(&"/tmp/module-prod.tfvars".to_string()));
    }
    
    #[test]
    fn test_should_ignore_workspace() {
        let config = create_test_config();
        let resolver = ConfigResolver::new(Some(config), PathBuf::from("/tmp"));
        
        assert!(resolver.should_ignore_workspace("infrastructure/networking", "dev", None));
        assert!(!resolver.should_ignore_workspace("infrastructure/networking", "prod", None));
    }
} 
