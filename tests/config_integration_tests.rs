use solarboat::config::{ConfigLoader, ConfigResolver};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_basic_config_loading() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = r#"{
        "global": {
            "ignore_workspaces": ["dev", "test"],
            "var_files": ["global.tfvars"],
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
    assert_eq!(config.global.var_files, vec!["global.tfvars"]);
    assert_eq!(
        config.global.workspace_var_files.as_ref().unwrap().workspaces["prod"],
        vec!["prod.tfvars"]
    );
}

#[test]
fn test_yaml_config_loading() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = r#"
global:
  ignore_workspaces:
    - dev
    - test
  var_files:
    - global.tfvars
  workspace_var_files:
    prod:
      - prod.tfvars
"#;
    
    let config_path = temp_dir.path().join("solarboat.yml");
    fs::write(&config_path, config_content).unwrap();
    
    let loader = ConfigLoader::new(temp_dir.path());
    let config = loader.load().unwrap().unwrap();
    
    assert_eq!(config.global.ignore_workspaces, vec!["dev", "test"]);
    assert_eq!(config.global.var_files, vec!["global.tfvars"]);
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
            "ignore_workspaces": ["dev"],
            "var_files": ["global.tfvars"]
        },
        "modules": {
            "infrastructure/networking": {
                "ignore_workspaces": ["test"],
                "var_files": ["networking.tfvars"],
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
    assert_eq!(module_config.var_files, vec!["networking.tfvars"]);
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
            "var_files": ["global.tfvars"],
            "workspace_var_files": {
                "prod": ["global-prod.tfvars"]
            }
        },
        "modules": {
            "infrastructure/networking": {
                "ignore_workspaces": ["test"],
                "var_files": ["networking.tfvars"],
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
    let module_settings = resolver.resolve_module_config("infrastructure/networking", None, None);
    assert_eq!(module_settings.ignore_workspaces, vec!["test"]);
    assert_eq!(module_settings.var_files, vec!["networking.tfvars"]);
    
    // Test workspace-specific var files (should include both general and workspace-specific, as absolute paths)
    let workspace_vars = resolver.get_workspace_var_files("infrastructure/networking", "prod", None);
    assert_eq!(
        workspace_vars,
        vec![
            temp_dir.path().join("infrastructure/networking/networking.tfvars").to_string_lossy().to_string(),
            temp_dir.path().join("infrastructure/networking/networking-prod.tfvars").to_string_lossy().to_string()
        ]
    );
    
    // Test fallback to global for non-existent module
    let fallback_settings = resolver.resolve_module_config("other/module", None, None);
    assert_eq!(fallback_settings.ignore_workspaces, vec!["dev"]);
    assert_eq!(fallback_settings.var_files, vec!["global.tfvars"]);
    
    let fallback_vars = resolver.get_workspace_var_files("other/module", "prod", None);
    assert_eq!(
        fallback_vars,
        vec![
            temp_dir.path().join("other/module/global.tfvars").to_string_lossy().to_string(),
            temp_dir.path().join("other/module/global-prod.tfvars").to_string_lossy().to_string()
        ]
    );
}

#[test]
fn test_environment_specific_config() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create environment-specific config
    let dev_config = r#"{
        "global": {
            "ignore_workspaces": ["prod", "staging"],
            "var_files": ["dev.tfvars"]
        }
    }"#;
    
    let prod_config = r#"{
        "global": {
            "ignore_workspaces": ["dev", "test"],
            "var_files": ["prod.tfvars"]
        }
    }"#;
    
    fs::write(temp_dir.path().join("solarboat.dev.json"), dev_config).unwrap();
    fs::write(temp_dir.path().join("solarboat.prod.json"), prod_config).unwrap();
    
    // Test dev environment
    std::env::set_var("SOLARBOAT_ENV", "dev");
    let loader = ConfigLoader::new(temp_dir.path());
    let config = loader.load().unwrap().unwrap();
    assert_eq!(config.global.ignore_workspaces, vec!["prod", "staging"]);
    assert_eq!(config.global.var_files, vec!["dev.tfvars"]);
    
    // Test prod environment
    std::env::set_var("SOLARBOAT_ENV", "prod");
    let loader = ConfigLoader::new(temp_dir.path());
    let config = loader.load().unwrap().unwrap();
    assert_eq!(config.global.ignore_workspaces, vec!["dev", "test"]);
    assert_eq!(config.global.var_files, vec!["prod.tfvars"]);
    
    // Clean up environment variable
    std::env::remove_var("SOLARBOAT_ENV");
}

#[test]
fn test_environment_config_fallback() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create only default config
    let default_config = r#"{
        "global": {
            "ignore_workspaces": ["default"],
            "var_files": ["default.tfvars"]
        }
    }"#;
    
    fs::write(temp_dir.path().join("solarboat.json"), default_config).unwrap();
    
    // Test non-existent environment falls back to default
    std::env::set_var("SOLARBOAT_ENV", "staging");
    let loader = ConfigLoader::new(temp_dir.path());
    let config = loader.load().unwrap().unwrap();
    assert_eq!(config.global.ignore_workspaces, vec!["default"]);
    assert_eq!(config.global.var_files, vec!["default.tfvars"]);
    
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
            "var_files": ["nonexistent.tfvars"],
            "workspace_var_files": {
                "default": ["default.tfvars"]
            }
        },
        "modules": {
            "nonexistent/module": {
                "var_files": ["module.tfvars"]
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
fn test_invalid_yaml_config() {
    let temp_dir = TempDir::new().unwrap();
    let invalid_config = r#"
global:
  ignore_workspaces:
    - dev
  var_files:
    - global.tfvars
  invalid_field: value
"#;
    
    let config_path = temp_dir.path().join("solarboat.yml");
    fs::write(&config_path, invalid_config).unwrap();
    
    let loader = ConfigLoader::new(temp_dir.path());
    let result = loader.load();
    // YAML with unknown fields should still load (serde ignores unknown fields)
    assert!(result.is_ok());
}

#[test]
fn test_config_with_absolute_paths() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = format!(r#"{{
        "global": {{
            "var_files": [
                "relative.tfvars",
                "{}/absolute.tfvars"
            ]
        }}
    }}"#, temp_dir.path().display());
    
    let config_path = temp_dir.path().join("solarboat.json");
    fs::write(&config_path, config_content).unwrap();
    
    // Create the files
    fs::write(temp_dir.path().join("relative.tfvars"), "relative = true").unwrap();
    fs::write(temp_dir.path().join("absolute.tfvars"), "absolute = true").unwrap();
    
    let loader = ConfigLoader::new(temp_dir.path());
    let config = loader.load().unwrap().unwrap();
    
    assert_eq!(config.global.var_files.len(), 2);
    assert!(config.global.var_files.contains(&"relative.tfvars".to_string()));
    assert!(config.global.var_files.contains(&format!("{}/absolute.tfvars", temp_dir.path().display())));
}

#[test]
fn test_workspace_var_files_merging() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = r#"{
        "global": {
            "var_files": ["global.tfvars"],
            "workspace_var_files": {
                "prod": ["global-prod.tfvars"]
            }
        },
        "modules": {
            "infrastructure/networking": {
                "var_files": ["networking.tfvars"],
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
    
    // Test that both general and workspace-specific var files are included (as absolute paths)
    let all_vars = resolver.get_workspace_var_files("infrastructure/networking", "prod", None);
    assert_eq!(
        all_vars,
        vec![
            temp_dir.path().join("infrastructure/networking/networking.tfvars").to_string_lossy().to_string(),
            temp_dir.path().join("infrastructure/networking/networking-prod.tfvars").to_string_lossy().to_string()
        ]
    );
    
    // Test fallback to global
    let fallback_vars = resolver.get_workspace_var_files("other/module", "prod", None);
    assert_eq!(
        fallback_vars,
        vec![
            temp_dir.path().join("other/module/global.tfvars").to_string_lossy().to_string(),
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
    assert!(config.global.var_files.is_empty());
    assert!(config.global.workspace_var_files.is_none());
    assert!(config.modules.is_empty());
} 
