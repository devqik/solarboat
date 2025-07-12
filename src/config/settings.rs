use crate::config::{ConfigLoader, ConfigResolver};
use anyhow::Result;

/// Application settings that can be loaded from configuration files
pub struct Settings {
    /// The resolved configuration for the application
    pub config_resolver: ConfigResolver,
}

impl Settings {
    /// Load settings from configuration file
    pub fn load<P: AsRef<std::path::Path>>(config_dir: P) -> Result<Self> {
        let config_dir = config_dir.as_ref().to_path_buf();
        let loader = ConfigLoader::new(&config_dir);
        
        // Load configuration file
        let config = loader.load()?;
        
        // Validate configuration if loaded
        if let Some(ref config_data) = config {
            loader.validate_config(config_data)?;
        }
        
        // Create resolver
        let config_resolver = ConfigResolver::new(config, config_dir);
        
        Ok(Self { config_resolver })
    }
    
    /// Load settings from current working directory
    pub fn load_from_current_dir() -> Result<Self> {
        let loader = ConfigLoader::from_current_dir()?;
        let config_dir = loader.search_dir.clone();
        
        // Load configuration file
        let config = loader.load()?;
        
        // Validate configuration if loaded
        if let Some(ref config_data) = config {
            loader.validate_config(config_data)?;
        }
        
        // Create resolver
        let config_resolver = ConfigResolver::new(config, config_dir);
        
        Ok(Self { config_resolver })
    }
    
    /// Get the configuration resolver
    pub fn resolver(&self) -> &ConfigResolver {
        &self.config_resolver
    }
}
