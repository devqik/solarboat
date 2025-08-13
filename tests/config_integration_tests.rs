use solarboat::config::{ConfigLoader, ConfigResolver};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_global_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = r#"{
        "global": {
            "ignore_workspaces": ["dev", "test"],
            "workspace_var_files": {
                "prod": ["prod.tfvars"]
            }
        }
    }"#;
    
    let config_path = temp_dir.path().join("solarboat.json");
    fs::write(&config_path, config_content).unwrap();
    
    let loader = ConfigLoader::new(temp_dir.path());
    let config = loader.load().unwrap().unwrap();
    
    assert_eq!(config.global.ignore_workspaces, vec!["dev", "test"]);
    // Note: var_files field has been removed
    assert_eq!(
        config.global.workspace_var_files.as_ref().unwrap().workspaces["prod"],
        vec!["prod.tfvars"]
    );
}

#[test]
fn test_module_specific_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = r#"{
        "global": {
            "ignore_workspaces": ["dev"]
        },
        "modules": {
            "infrastructure/networking": {
                "ignore_workspaces": ["test"],
                "workspace_var_files": {
                    "prod": ["networking-prod.tfvars"]
                }
            }
        }
    }"#;
    
    let config_path = temp_dir.path().join("solarboat.json");
    fs::write(&config_path, config_content).unwrap();
    
    let loader = ConfigLoader::new(temp_dir.path());
    let config = loader.load().unwrap().unwrap();
    
    let module_config = &config.modules["infrastructure/networking"];
    assert_eq!(module_config.ignore_workspaces, vec!["test"]);
    // Note: var_files field has been removed
    assert_eq!(
        module_config.workspace_var_files.as_ref().unwrap().workspaces["prod"],
        vec!["networking-prod.tfvars"]
    );
}

#[test]
fn test_config_resolver_precedence() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = r#"{
        "global": {
            "ignore_workspaces": ["dev"],
            "workspace_var_files": {
                "prod": ["global-prod.tfvars"]
            }
        },
        "modules": {
            "infrastructure/networking": {
                "ignore_workspaces": ["test"],
                "workspace_var_files": {
                    "prod": ["networking-prod.tfvars"]
                }
            }
        }
    }"#;
    
    let config_path = temp_dir.path().join("solarboat.json");
    fs::write(&config_path, config_content).unwrap();
    
    let loader = ConfigLoader::new(temp_dir.path());
    let config = loader.load().unwrap().unwrap();
    let resolver = ConfigResolver::new(Some(config), temp_dir.path().to_path_buf());
    
    // Test module-specific settings override global
    let module_settings = resolver.resolve_module_config("infrastructure/networking", None);
    assert_eq!(module_settings.ignore_workspaces, vec!["test"]);
    
    // Test workspace-specific var files (should include workspace-specific files, as absolute paths)
    let workspace_vars = resolver.get_workspace_var_files("infrastructure/networking", "prod", None);
    assert_eq!(
        workspace_vars,
        vec![
            temp_dir.path().join("infrastructure/networking/networking-prod.tfvars").to_string_lossy().to_string()
        ]
    );
    
    // Test fallback to global for non-existent module
    let fallback_settings = resolver.resolve_module_config("other/module", None);
    assert_eq!(fallback_settings.ignore_workspaces, vec!["dev"]);
    assert_eq!(fallback_settings.var_files, Vec::<String>::new()); // var_files is now empty
    
    let fallback_vars = resolver.get_workspace_var_files("other/module", "prod", None);
    assert_eq!(
        fallback_vars,
        vec![
            temp_dir.path().join("other/module/global-prod.tfvars").to_string_lossy().to_string()
        ]
    );
}

#[test]
fn test_environment_config() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create environment-specific configs
    let dev_config = r#"{
        "global": {
            "ignore_workspaces": ["prod", "staging"]
        }
    }"#;
    
    let prod_config = r#"{
        "global": {
            "ignore_workspaces": ["dev", "test"]
        }
    }"#;
    
    fs::write(temp_dir.path().join("solarboat.dev.json"), dev_config).unwrap();
    fs::write(temp_dir.path().join("solarboat.prod.json"), prod_config).unwrap();
    
    // Test dev environment
    std::env::set_var("SOLARBOAT_ENV", "dev");
    let loader = ConfigLoader::new(temp_dir.path());
    let config = loader.load().unwrap().unwrap();
    assert_eq!(config.global.ignore_workspaces, vec!["prod", "staging"]);
    // Note: var_files field has been removed
    
    // Test prod environment
    std::env::set_var("SOLARBOAT_ENV", "prod");
    let loader = ConfigLoader::new(temp_dir.path());
    let config = loader.load().unwrap().unwrap();
    assert_eq!(config.global.ignore_workspaces, vec!["dev", "test"]);
    // Note: var_files field has been removed
    
    // Clean up environment variable
    std::env::remove_var("SOLARBOAT_ENV");
}

#[test]
fn test_environment_config_fallback() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create only default config
    let default_config = r#"{
        "global": {
            "ignore_workspaces": ["default"]
        }
    }"#;
    
    fs::write(temp_dir.path().join("solarboat.json"), default_config).unwrap();
    
    // Test non-existent environment falls back to default
    std::env::set_var("SOLARBOAT_ENV", "staging");
    let loader = ConfigLoader::new(temp_dir.path());
    let config = loader.load().unwrap().unwrap();
    assert_eq!(config.global.ignore_workspaces, vec!["default"]);
    // Note: var_files field has been removed
    
    // Clean up environment variable
    std::env::remove_var("SOLARBOAT_ENV");
}

#[test]
fn test_config_validation() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a config with some issues
    let config_content = r#"{
        "global": {
            "ignore_workspaces": ["dev"],
            "workspace_var_files": {
                "default": ["default.tfvars"]
            }
        },
        "modules": {
            "nonexistent/module": {
                "ignore_workspaces": ["test"]
            }
        }
    }"#;
    
    let config_path = temp_dir.path().join("solarboat.json");
    fs::write(&config_path, config_content).unwrap();
    
    let loader = ConfigLoader::new(temp_dir.path());
    let config = loader.load().unwrap().unwrap();
    
    // Validation should not fail but should produce warnings
    let result = loader.validate_config(&config);
    assert!(result.is_ok());
}

#[test]
fn test_no_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let loader = ConfigLoader::new(temp_dir.path());
    let config = loader.load().unwrap();
    assert!(config.is_none());
}

#[test]
fn test_invalid_json_config() {
    let temp_dir = TempDir::new().unwrap();
    let invalid_config = r#"{
        "global": {
            "ignore_workspaces": ["dev"
        }
    }"#;
    
    let config_path = temp_dir.path().join("solarboat.json");
    fs::write(&config_path, invalid_config).unwrap();
    
    let loader = ConfigLoader::new(temp_dir.path());
    let result = loader.load();
    assert!(result.is_err());
}

#[test]
fn test_config_with_absolute_paths() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = format!(r#"{{
        "global": {{
            "workspace_var_files": {{
                "default": [
                    "relative.tfvars",
                    "{}/absolute.tfvars"
                ]
            }}
        }}
    }}"#, temp_dir.path().display());
    
    let config_path = temp_dir.path().join("solarboat.json");
    fs::write(&config_path, config_content).unwrap();
    
    // Create the files
    fs::write(temp_dir.path().join("relative.tfvars"), "relative = true").unwrap();
    fs::write(temp_dir.path().join("absolute.tfvars"), "absolute = true").unwrap();
    
    let loader = ConfigLoader::new(temp_dir.path());
    let config = loader.load().unwrap().unwrap();
    
    // Note: var_files field has been removed, testing workspace_var_files instead
    assert!(config.global.workspace_var_files.is_some());
    let workspace_files = config.global.workspace_var_files.as_ref().unwrap();
    assert_eq!(workspace_files.workspaces["default"].len(), 2);
    assert!(workspace_files.workspaces["default"].contains(&"relative.tfvars".to_string()));
    assert!(workspace_files.workspaces["default"].contains(&format!("{}/absolute.tfvars", temp_dir.path().display())));
}

#[test]
fn test_workspace_var_files_merging() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = r#"{
        "global": {
            "workspace_var_files": {
                "prod": ["global-prod.tfvars"]
            }
        },
        "modules": {
            "infrastructure/networking": {
                "workspace_var_files": {
                    "prod": ["networking-prod.tfvars"]
                }
            }
        }
    }"#;
    
    let config_path = temp_dir.path().join("solarboat.json");
    fs::write(&config_path, config_content).unwrap();
    
    let loader = ConfigLoader::new(temp_dir.path());
    let config = loader.load().unwrap().unwrap();
    let resolver = ConfigResolver::new(Some(config), temp_dir.path().to_path_buf());
    
    // Test that workspace-specific var files are included (as absolute paths)
    let all_vars = resolver.get_workspace_var_files("infrastructure/networking", "prod", None);
    assert_eq!(
        all_vars,
        vec![
            temp_dir.path().join("infrastructure/networking/networking-prod.tfvars").to_string_lossy().to_string()
        ]
    );
    
    // Test fallback to global
    let fallback_vars = resolver.get_workspace_var_files("other/module", "prod", None);
    assert_eq!(
        fallback_vars,
        vec![
            temp_dir.path().join("other/module/global-prod.tfvars").to_string_lossy().to_string()
        ]
    );
}

#[test]
fn test_empty_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = r#"{}"#;
    
    let config_path = temp_dir.path().join("solarboat.json");
    fs::write(&config_path, config_content).unwrap();
    
    let loader = ConfigLoader::new(temp_dir.path());
    let config = loader.load().unwrap().unwrap();
    
    // Should have default values
    assert!(config.global.ignore_workspaces.is_empty());
    // Note: var_files field has been removed
    assert!(config.global.workspace_var_files.is_none());
    assert!(config.modules.is_empty());
} 
