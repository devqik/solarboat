use crate::config::types::SolarboatConfig;
use anyhow::{Context, Result};
use serde_json;
use serde_yaml;
use std::path::{Path, PathBuf};

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
        for filename in CONFIG_FILE_NAMES {
            let config_path = self.search_dir.join(filename);
            if config_path.exists() {
                return Ok(Some(config_path));
            }
        }
        Ok(None)
    }
    
    /// Validate the loaded configuration
    pub fn validate_config(&self, config: &SolarboatConfig) -> Result<()> {
        // Validate module paths exist
        for module_path in config.modules.keys() {
            let full_path = self.search_dir.join(module_path);
            if !full_path.exists() {
                println!("⚠️  Warning: Module path '{}' does not exist", module_path);
            }
        }
        
        // Validate var file paths (basic check - just warn if they don't exist)
        self.validate_var_files(&config.global.var_files, "global")?;
        
        if let Some(workspace_files) = &config.global.workspace_var_files {
            for (workspace, files) in &workspace_files.workspaces {
                self.validate_var_files(files, &format!("global workspace '{}'", workspace))?;
            }
        }
        
        for (module_path, module_config) in &config.modules {
            self.validate_var_files(&module_config.var_files, &format!("module '{}'", module_path))?;
            
            if let Some(workspace_files) = &module_config.workspace_var_files {
                for (workspace, files) in &workspace_files.workspaces {
                    self.validate_var_files(files, &format!("module '{}' workspace '{}'", module_path, workspace))?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate variable file paths
    fn validate_var_files(&self, var_files: &[String], context: &str) -> Result<()> {
        for var_file in var_files {
            let var_path = if Path::new(var_file).is_absolute() {
                PathBuf::from(var_file)
            } else {
                self.search_dir.join(var_file)
            };
            
            if !var_path.exists() {
                println!("⚠️  Warning: Var file '{}' for {} does not exist", var_file, context);
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
