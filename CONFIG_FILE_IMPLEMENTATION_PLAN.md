# Solarboat Configuration File Implementation Plan

## Overview

Implement a configuration file system that allows users to specify module-specific and global settings for workspace management and variable files, supporting both JSON and YAML formats.

## Configuration File Structure

### Example `solarboat.json`:

```json
{
  "global": {
    "ignore_workspaces": ["dev", "test"],
    "var_files": ["global.tfvars", "environment.tfvars"],
    "workspace_var_files": {
      "dev": ["dev.tfvars", "dev-secrets.tfvars"],
      "staging": ["staging.tfvars", "staging-secrets.tfvars"],
      "prod": ["prod.tfvars", "prod-secrets.tfvars"]
    }
  },
  "modules": {
    "infrastructure/networking": {
      "ignore_workspaces": ["dev"],
      "var_files": ["networking.tfvars", "vpc.tfvars"],
      "workspace_var_files": {
        "staging": ["networking-staging.tfvars"],
        "prod": ["networking-prod.tfvars", "networking-prod-secrets.tfvars"]
      }
    },
    "infrastructure/compute": {
      "ignore_workspaces": ["staging"],
      "var_files": ["compute.tfvars"],
      "workspace_var_files": {
        "dev": ["compute-dev.tfvars"],
        "prod": ["compute-prod.tfvars", "compute-prod-secrets.tfvars"]
      }
    }
  }
}
```

### Example `solarboat.yml`:

```yaml
global:
  ignore_workspaces:
    - dev
    - test
  var_files:
    - global.tfvars
    - environment.tfvars
  workspace_var_files:
    dev:
      - dev.tfvars
      - dev-secrets.tfvars
    staging:
      - staging.tfvars
      - staging-secrets.tfvars
    prod:
      - prod.tfvars
      - prod-secrets.tfvars

modules:
  infrastructure/networking:
    ignore_workspaces:
      - dev
    var_files:
      - networking.tfvars
      - vpc.tfvars
    workspace_var_files:
      staging:
        - networking-staging.tfvars
      prod:
        - networking-prod.tfvars
        - networking-prod-secrets.tfvars
  infrastructure/compute:
    ignore_workspaces:
      - staging
    var_files:
      - compute.tfvars
    workspace_var_files:
      dev:
        - compute-dev.tfvars
      prod:
        - compute-prod.tfvars
        - compute-prod-secrets.tfvars
```

## Implementation Steps

### Phase 1: Core Configuration Infrastructure

#### Step 1.1: Add Configuration Dependencies

- [x] Add `serde` and `serde_json` dependencies to `Cargo.toml`
- [x] Add `serde_yaml` dependency for YAML support
- [x] Add `anyhow` for better error handling

#### Step 1.2: Define Configuration Data Structures

- [x] Create `src/config/types.rs` with configuration structs:
  - [x] `GlobalConfig` struct for global settings
  - [x] `ModuleConfig` struct for module-specific settings
  - [x] `SolarboatConfig` struct as the root configuration
  - [x] Add `WorkspaceVarFiles` struct for workspace-specific var file mappings
  - [x] Add proper serde derive macros and documentation

#### Step 1.3: Configuration Loading Logic

- [x] Create `src/config/loader.rs` with:
  - [x] `ConfigLoader` struct to handle file discovery and loading
  - [x] Support for both JSON and YAML formats
  - [x] Configuration file discovery (search for `solarboat.json`, `solarboat.yml`, `solarboat.yaml`)
  - [x] Configuration validation and error handling
  - [x] Default configuration fallback

#### Step 1.4: Update Configuration Module

- [x] Update `src/config/mod.rs` to export new modules
- [x] Update `src/config/settings.rs` to use new configuration system
- [x] Add configuration merging logic (CLI args override config file)

### Phase 2: CLI Integration

#### Step 2.1: Add Configuration File CLI Option

- [x] Update `src/cli/args.rs`:
  - [x] Add `--config` option to all commands for explicit config file path
  - [x] Add `--no-config` flag to disable config file loading
  - [x] Update help text and documentation

#### Step 2.2: Configuration Resolution Logic

- [x] Create `src/config/resolver.rs` with:
  - [x] `ConfigResolver` struct to merge CLI args with config file
  - [x] Logic to resolve module-specific vs global settings
  - [x] Priority order: CLI args > module config > global config > defaults
  - [x] Path resolution for var files (relative to config file location)
  - [x] Logic to merge workspace-specific var files with general var files
  - [x] Function to get final var files list for a specific module and workspace

#### Step 2.3: Update Command Handlers

- [x] Update `src/commands/mod.rs` to load configuration
- [x] Modify each command execution to use resolved configuration
- [x] Pass resolved settings to plan and apply helpers

### Phase 3: Helper Function Updates

#### Step 3.1: Update Plan Helpers

- [x] Modify `src/commands/plan/helpers.rs`:
  - [x] Update `run_terraform_plan` to accept resolved configuration
  - [x] Add logic to apply module-specific workspace and var file settings
  - [x] Add logic to apply workspace-specific var files for each workspace
  - [x] Maintain backward compatibility with existing CLI args

#### Step 3.2: Update Apply Helpers

- [x] Modify `src/commands/apply/helpers.rs`:
  - [x] Update `run_terraform_apply` to accept resolved configuration
  - [x] Add logic to apply module-specific workspace and var file settings
  - [x] Add logic to apply workspace-specific var files for each workspace
  - [x] Maintain backward compatibility with existing CLI args

### Phase 4: Advanced Features

#### Step 4.1: Environment-Specific Configuration

- [ ] Add support for environment-specific config files:
  - [ ] `solarboat.dev.json`, `solarboat.prod.yml`, etc.
  - [ ] Environment detection via `SOLARBOAT_ENV` environment variable
  - [ ] Fallback to base config if environment-specific not found

#### Step 4.2: Configuration Validation

- [ ] Add configuration schema validation
- [ ] Validate module paths exist in filesystem
- [ ] Validate var file paths are accessible
- [ ] Add helpful error messages for configuration issues

#### Step 4.3: Configuration Documentation

- [ ] Create comprehensive documentation for configuration options
- [ ] Add examples for common use cases
- [ ] Document configuration precedence and merging rules

### Phase 5: Testing and Validation

#### Step 5.1: Unit Tests

- [ ] Test configuration loading from JSON and YAML
- [ ] Test configuration merging logic
- [ ] Test module-specific vs global settings resolution
- [ ] Test error handling for invalid configurations

#### Step 5.2: Integration Tests

- [ ] Test CLI commands with configuration files
- [ ] Test backward compatibility with existing CLI usage
- [ ] Test configuration file discovery and loading
- [ ] Test var file path resolution

#### Step 5.3: Example Configurations

- [ ] Create example configuration files in project root
- [ ] Add configuration examples to README
- [ ] Create sample configurations for different scenarios

## File Structure After Implementation

```
src/
├── config/
│   ├── mod.rs
│   ├── types.rs          # Configuration data structures
│   ├── loader.rs          # Configuration file loading
│   ├── resolver.rs        # Configuration resolution logic
│   └── settings.rs        # Updated settings module
├── cli/
│   └── args.rs           # Updated with config options
└── commands/
    ├── mod.rs            # Updated to use configuration
    ├── plan/
    │   └── helpers.rs    # Updated to use resolved config
    └── apply/
        └── helpers.rs    # Updated to use resolved config
```

## Configuration Precedence Rules

1. **CLI Arguments** (highest priority)
   - Explicit `--ignore-workspaces` and `--var-files` override all config
2. **Module-Specific Configuration**
   - Settings from `modules.<module_path>` in config file
3. **Global Configuration**
   - Settings from `global` section in config file
4. **Defaults** (lowest priority)
   - Built-in default values

## Workspace-Specific Variable Files

### Var File Resolution Order

For each workspace, var files are resolved in the following order:

1. **General var files** (applied to all workspaces):

   - Module-specific `var_files` (if module config exists)
   - Global `var_files` (fallback)

2. **Workspace-specific var files** (applied only to specific workspace):

   - Module-specific `workspace_var_files.<workspace_name>` (if module config exists)
   - Global `workspace_var_files.<workspace_name>` (fallback)

3. **Final var files list**: General var files + workspace-specific var files

### Example Resolution

For module `infrastructure/networking` and workspace `prod`:

- General var files: `["networking.tfvars", "vpc.tfvars"]` (from module config)
- Workspace-specific var files: `["networking-prod.tfvars", "networking-prod-secrets.tfvars"]` (from module config)
- Final var files: `["networking.tfvars", "vpc.tfvars", "networking-prod.tfvars", "networking-prod-secrets.tfvars"]`

### Use Cases

- **Environment-specific secrets**: Different secret files for dev/staging/prod
- **Environment-specific configurations**: Different variable values per environment
- **Module-specific overrides**: Override global workspace var files for specific modules
- **Layered configuration**: Combine general and workspace-specific settings

## Migration Strategy

- Maintain full backward compatibility with existing CLI usage
- Configuration files are optional - existing commands work unchanged
- Gradual migration path: users can start with global settings, then add module-specific configs
- Clear documentation on how to migrate from CLI-only to configuration-based usage

## Success Criteria

- [ ] Users can specify global workspace and var file settings
- [ ] Users can specify module-specific workspace and var file settings
- [ ] Users can specify workspace-specific var files for different environments
- [ ] Configuration supports both JSON and YAML formats
- [ ] CLI arguments override configuration file settings
- [ ] Var file resolution correctly combines general and workspace-specific files
- [ ] Backward compatibility is maintained
- [ ] Comprehensive error handling and validation
- [ ] Clear documentation and examples provided
