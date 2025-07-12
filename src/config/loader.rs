use crate::config::types::SolarboatConfig;
use anyhow::{Context, Result};
use serde_json;
use serde_yaml;
use std::path::{Path, PathBuf};
use std::env;

/// Configuration file names to search for
const CONFIG_FILE_NAMES: &[&str] = &[
    "solarboat.json",
    "solarboat.yml", 
    "solarboat.yaml",
];

/// Loader for solarboat configuration files
pub struct ConfigLoader {
    /// The directory where configuration files are searched
    pub search_dir: PathBuf,
}

impl ConfigLoader {
    /// Create a new ConfigLoader for the given directory
    pub fn new<P: AsRef<Path>>(search_dir: P) -> Self {
        Self {
            search_dir: search_dir.as_ref().to_path_buf(),
        }
    }
    
    /// Create a ConfigLoader for the current working directory
    pub fn from_current_dir() -> Result<Self> {
        let current_dir = std::env::current_dir()
            .context("Failed to get current working directory")?;
        Ok(Self::new(current_dir))
    }
    
    /// Find and load the configuration file
    pub fn load(&self) -> Result<Option<SolarboatConfig>> {
        let config_path = self.find_config_file()?;
        
        match config_path {
            Some(path) => {
                println!("📄 Loading configuration from: {}", path.display());
                let config = self.load_from_path(&path)?;
                Ok(Some(config))
            }
            None => {
                println!("ℹ️  No configuration file found, using defaults");
                Ok(None)
            }
        }
    }
    
    /// Load configuration from a specific file path
    pub fn load_from_path<P: AsRef<Path>>(&self, path: P) -> Result<SolarboatConfig> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read configuration file: {}", path.display()))?;
        
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("json") => {
                serde_json::from_str(&content)
                    .with_context(|| format!("Failed to parse JSON configuration: {}", path.display()))
            }
            Some("yml") | Some("yaml") => {
                serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML configuration: {}", path.display()))
            }
            _ => {
                // Try to detect format by content
                if content.trim().starts_with('{') {
                    serde_json::from_str(&content)
                        .with_context(|| format!("Failed to parse JSON configuration: {}", path.display()))
                } else {
                    serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML configuration: {}", path.display()))
                }
            }
        }
    }
    
    /// Find the first available configuration file
    fn find_config_file(&self) -> Result<Option<PathBuf>> {
        // Check for SOLARBOAT_ENV
        let mut search_order = Vec::new();
        if let Ok(env) = env::var("SOLARBOAT_ENV") {
            if !env.trim().is_empty() {
                search_order.push(format!("solarboat.{}.json", env));
                search_order.push(format!("solarboat.{}.yml", env));
                search_order.push(format!("solarboat.{}.yaml", env));
            }
        }
        // Add default config file names
        for &filename in CONFIG_FILE_NAMES {
            search_order.push(filename.to_string());
        }
        for filename in search_order {
            let config_path = self.search_dir.join(&filename);
            if config_path.exists() {
                if let Ok(env) = env::var("SOLARBOAT_ENV") {
                    if !env.trim().is_empty() && filename.contains(&env) {
                        println!("📄 Detected SOLARBOAT_ENV='{}', loading environment-specific config: {}", env, config_path.display());
                    } else {
                        println!("📄 Loading configuration from: {}", config_path.display());
                    }
                } else {
                    println!("📄 Loading configuration from: {}", config_path.display());
                }
                return Ok(Some(config_path));
            }
        }
        Ok(None)
    }
    
    /// Validate the loaded configuration
    pub fn validate_config(&self, config: &SolarboatConfig) -> Result<()> {
        let validation_errors: Vec<String> = Vec::new();
        let mut validation_warnings: Vec<String> = Vec::new();
        
        // Validate module paths exist
        for module_path in config.modules.keys() {
            let full_path = self.search_dir.join(module_path);
            if !full_path.exists() {
                validation_warnings.push(format!("Module path '{}' does not exist (checked: {})", 
                    module_path, full_path.display()));
            }
        }
        
        // Validate var file paths
        self.validate_var_files(&config.global.var_files, "global", &mut validation_warnings)?;
        
        if let Some(workspace_files) = &config.global.workspace_var_files {
            for (workspace, files) in &workspace_files.workspaces {
                self.validate_var_files(files, &format!("global workspace '{}'", workspace), &mut validation_warnings)?;
            }
        }
        
        for (module_path, module_config) in &config.modules {
            self.validate_var_files(&module_config.var_files, &format!("module '{}'", module_path), &mut validation_warnings)?;
            
            if let Some(workspace_files) = &module_config.workspace_var_files {
                for (workspace, files) in &workspace_files.workspaces {
                    self.validate_var_files(files, &format!("module '{}' workspace '{}'", module_path, workspace), &mut validation_warnings)?;
                }
            }
        }
        
        // Validate workspace names (basic sanity check)
        self.validate_workspace_names(config, &mut validation_warnings)?;
        
        // Print warnings
        if !validation_warnings.is_empty() {
            println!("⚠️  Configuration validation warnings:");
            for warning in validation_warnings {
                println!("   • {}", warning);
            }
        }
        
        // Print errors and return error if any
        if !validation_errors.is_empty() {
            eprintln!("❌ Configuration validation errors:");
            for error in &validation_errors {
                eprintln!("   • {}", error);
            }
            return Err(anyhow::anyhow!("Configuration validation failed with {} error(s)", validation_errors.len()));
        }
        
        Ok(())
    }
    
    /// Validate variable file paths
    fn validate_var_files(&self, var_files: &[String], context: &str, warnings: &mut Vec<String>) -> Result<()> {
        for var_file in var_files {
            let var_path = if Path::new(var_file).is_absolute() {
                PathBuf::from(var_file)
            } else {
                // All var files (both global and module-specific) are checked relative to the module directory
                if context.starts_with("module") {
                    // Extract module path from context (e.g., "module 'infrastructure/networking' workspace 'develop'")
                    let module_path = context.split("'").nth(1).unwrap_or("");
                    let module_dir = self.search_dir.join(module_path);
                    module_dir.join(var_file)
                } else {
                    // For global config and other contexts, we'll check relative to config directory for validation
                    // The actual resolution will be done relative to the module when the module is processed
                    self.search_dir.join(var_file)
                }
            };
            
            if !var_path.exists() {
                warnings.push(format!("Var file '{}' for {} does not exist (checked: {})", 
                    var_file, context, var_path.display()));
            }
        }
        Ok(())
    }
    
    /// Validate workspace names for basic sanity
    fn validate_workspace_names(&self, config: &SolarboatConfig, warnings: &mut Vec<String>) -> Result<()> {
        let reserved_names = ["default", "terraform"];
        
        // Check global workspace var files
        if let Some(workspace_files) = &config.global.workspace_var_files {
            for workspace in workspace_files.workspaces.keys() {
                if reserved_names.contains(&workspace.as_str()) {
                    warnings.push(format!("Workspace name '{}' is reserved and may cause issues", workspace));
                }
            }
        }
        
        // Check module workspace var files
        for (module_path, module_config) in &config.modules {
            if let Some(workspace_files) = &module_config.workspace_var_files {
                for workspace in workspace_files.workspaces.keys() {
                    if reserved_names.contains(&workspace.as_str()) {
                        warnings.push(format!("Workspace name '{}' in module '{}' is reserved and may cause issues", 
                            workspace, module_path));
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_load_json_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"{
            "global": {
                "ignore_workspaces": ["dev", "test"],
                "var_files": ["global.tfvars"]
            },
            "modules": {
                "infrastructure/networking": {
                    "ignore_workspaces": ["dev"],
                    "var_files": ["networking.tfvars"]
                }
            }
        }"#;
        
        fs::write(temp_dir.path().join("solarboat.json"), config_content).unwrap();
        
        let loader = ConfigLoader::new(temp_dir.path());
        let config = loader.load().unwrap().unwrap();
        
        assert_eq!(config.global.ignore_workspaces, vec!["dev", "test"]);
        assert_eq!(config.global.var_files, vec!["global.tfvars"]);
        assert!(config.modules.contains_key("infrastructure/networking"));
    }
    
    #[test]
    fn test_load_yaml_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
global:
  ignore_workspaces:
    - dev
    - test
  var_files:
    - global.tfvars
modules:
  infrastructure/networking:
    ignore_workspaces:
      - dev
    var_files:
      - networking.tfvars
"#;
        
        fs::write(temp_dir.path().join("solarboat.yml"), config_content).unwrap();
        
        let loader = ConfigLoader::new(temp_dir.path());
        let config = loader.load().unwrap().unwrap();
        
        assert_eq!(config.global.ignore_workspaces, vec!["dev", "test"]);
        assert_eq!(config.global.var_files, vec!["global.tfvars"]);
        assert!(config.modules.contains_key("infrastructure/networking"));
    }
    
    #[test]
    fn test_no_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::new(temp_dir.path());
        let config = loader.load().unwrap();
        
        assert!(config.is_none());
    }
} 
