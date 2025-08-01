# Solarboat Configuration Guide

Solarboat supports configuration files to customize workspace management and variable file handling across your Terraform modules. This guide covers all configuration options and their usage.

## Quick Start

1. Create a configuration file in your project root:

   - `solarboat.json` (JSON format)

2. Configure your settings and run solarboat commands as usual.

## Configuration File Structure

### Basic Configuration

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
    }
  }
}
```

## Configuration Sections

### Global Configuration

The `global` section applies to all modules unless overridden by module-specific settings.

#### `ignore_workspaces`

- **Type**: Array of strings
- **Description**: Workspace names to skip during plan and apply operations
- **Example**: `["dev", "test"]`

#### `var_files`

- **Type**: Array of strings
- **Description**: Variable files to use for all workspaces
- **Example**: `["global.tfvars", "environment.tfvars"]`

#### `workspace_var_files`

- **Type**: Object mapping workspace names to arrays of variable files
- **Description**: Workspace-specific variable files
- **Example**:
  ```json
  {
    "dev": ["dev.tfvars", "dev-secrets.tfvars"],
    "prod": ["prod.tfvars", "prod-secrets.tfvars"]
  }
  ```

### Module-Specific Configuration

The `modules` section allows you to override global settings for specific modules.

#### Module Path

- **Type**: String (key in the modules object)
- **Description**: Path to the Terraform module. Can be relative to the configuration file location or absolute path. Solarboat automatically normalizes paths for lookup.
- **Examples**:
  - `"infrastructure/networking"` (relative path)
  - `"terraform/projects/webapp"` (relative path)
  - `"/absolute/path/to/module"` (absolute path - normalized to relative internally)

**Note**: Module paths are automatically normalized to be relative to the configuration file location for consistent lookup, regardless of whether you specify them as absolute or relative paths.

#### Module Settings

Each module can have the same settings as the global configuration:

- `ignore_workspaces`: Override global ignore settings for this module
- `var_files`: Override global var files for this module
- `workspace_var_files`: Override global workspace var files for this module

## Environment-Specific Configuration

Solarboat supports environment-specific configuration files using the `SOLARBOAT_ENV` environment variable.

### Usage

```bash
# Use development configuration
SOLARBOAT_ENV=dev solarboat plan

# Use production configuration
SOLARBOAT_ENV=prod solarboat apply

# Use staging configuration
SOLARBOAT_ENV=staging solarboat scan
```

### File Naming Convention

When `SOLARBOAT_ENV` is set, solarboat looks for these files in order:

1. `solarboat.<env>.json`
2. `solarboat.json` (fallback)

### Example Environment Files

**solarboat.dev.json:**

```json
{
  "global": {
    "ignore_workspaces": ["prod", "staging"],
    "var_files": ["dev.tfvars"],
    "workspace_var_files": {
      "dev": ["dev-secrets.tfvars"]
    }
  }
}
```

**solarboat.prod.json:**

```json
{
  "global": {
    "ignore_workspaces": ["dev", "test"],
    "var_files": ["prod.tfvars"],
    "workspace_var_files": {
      "prod": ["prod-secrets.tfvars"]
    }
  }
}
```

## Configuration Precedence

Settings are resolved in the following order (highest to lowest priority):

1. **CLI Arguments**: `--ignore-workspaces`, `--var-files`
2. **Module-Specific Configuration**: Settings from `modules.<module_path>`
3. **Global Configuration**: Settings from the `global` section
4. **Defaults**: Built-in default values

### Example Precedence

Given this configuration:

```json
{
  "global": {
    "ignore_workspaces": ["test"],
    "var_files": ["global.tfvars"]
  },
  "modules": {
    "infrastructure/networking": {
      "ignore_workspaces": ["dev"],
      "var_files": ["networking.tfvars"]
    }
  }
}
```

For module `infrastructure/networking`:

- **Ignore workspaces**: `["dev"]` (module-specific overrides global)
- **Var files**: `["networking.tfvars"]` (module-specific overrides global)

If you run: `solarboat plan --ignore-workspaces prod,staging`

- **Ignore workspaces**: `["prod", "staging"]` (CLI overrides all config)

## Variable File Resolution

### General Variable Files

Variable files specified in `var_files` are applied to all workspaces.

### Workspace-Specific Variable Files

Variable files specified in `workspace_var_files.<workspace_name>` are applied only to that specific workspace.

### Final Variable File List

For each workspace, the final variable file list is:

1. General var files (module-specific → global fallback)
2. Workspace-specific var files (module-specific → global fallback)

### Example Resolution

For module `infrastructure/networking` and workspace `prod`:

- **General var files**: `["networking.tfvars", "vpc.tfvars"]` (from module config)
- **Workspace-specific var files**: `["networking-prod.tfvars", "networking-prod-secrets.tfvars"]` (from module config)
- **Final var files**: `["networking.tfvars", "vpc.tfvars", "networking-prod.tfvars", "networking-prod-secrets.tfvars"]`

## Path Resolution

### Module Paths

Module paths (keys in the `modules` object) are automatically normalized to be relative to the configuration file location. This ensures consistent lookup regardless of how you specify the path:

- **Relative paths**: Used as-is (e.g., `"terraform/projects/webapp"`)
- **Absolute paths**: Automatically converted to relative paths (e.g., `/full/path/to/terraform/projects/webapp` → `terraform/projects/webapp`)

This normalization happens internally and is transparent to users.

**Example**: If your configuration file is at `/home/user/project/solarboat.json` and you have a module at `/home/user/project/terraform/projects/webapp`, you can specify it in your configuration as:

```json
{
  "modules": {
    "terraform/projects/webapp": {
      "ignore_workspaces": ["dev"]
    }
  }
}
```

Solarboat will automatically match this configuration to the discovered module path, regardless of whether the module was found using its absolute or relative path.

### Variable File Paths

Variable file paths are resolved relative to the configuration file location.

### Relative Paths

Variable file paths are resolved relative to the configuration file location.

### Absolute Paths

Absolute paths are used as-is.

### Examples

```json
{
  "global": {
    "var_files": [
      "vars/global.tfvars", // Relative to config file
      "/absolute/path/secrets.tfvars" // Absolute path
    ]
  }
}
```

## CLI Integration

### Configuration File Options

```bash
# Use specific configuration file
solarboat --config /path/to/solarboat.json plan

# Disable configuration file loading
solarboat --no-config plan

# Auto-discover configuration file
solarboat plan
```

### Backward Compatibility

All existing CLI options continue to work:

- `--ignore-workspaces` overrides configuration file settings
- `--var-files` overrides configuration file settings
- Configuration files are optional

## Validation and Error Handling

### Configuration Validation

Solarboat validates your configuration and provides helpful warnings for:

- Missing module paths
- Missing variable files
- Reserved workspace names (`default`, `terraform`)

### Example Validation Output

```
⚠️  Configuration validation warnings:
   • Module path 'infrastructure/networking' does not exist (checked: /path/to/infrastructure/networking)
   • Var file 'dev.tfvars' for global does not exist (checked: /path/to/dev.tfvars)
   • Workspace name 'default' is reserved and may cause issues
```

### Error Handling

- Invalid JSON syntax results in immediate error
- Missing required fields use sensible defaults
- Configuration validation warnings don't prevent execution

## Best Practices

### 1. Use Environment-Specific Configurations

```bash
# Development
SOLARBOAT_ENV=dev solarboat plan

# Production
SOLARBOAT_ENV=prod solarboat apply
```

### 2. Organize Variable Files

```
project/
├── solarboat.json
├── vars/
│   ├── global.tfvars
│   ├── dev.tfvars
│   ├── dev-secrets.tfvars
│   ├── prod.tfvars
│   └── prod-secrets.tfvars
└── infrastructure/
    └── networking/
        ├── main.tf
        └── networking.tfvars
```

### 3. Use Module-Specific Overrides Sparingly

Only override global settings when necessary to avoid configuration duplication.

### 4. Validate Your Configuration

Run `solarboat scan` to validate your configuration before using plan/apply commands.

### 5. Keep Secrets Separate

Use workspace-specific variable files for sensitive information:

```json
{
  "workspace_var_files": {
    "prod": ["prod-secrets.tfvars"],
    "dev": ["dev-secrets.tfvars"]
  }
}
```

## Migration from CLI-Only Usage

### Before (CLI-only)

```bash
solarboat plan --ignore-workspaces dev,test --var-files global.tfvars,env.tfvars
solarboat apply --ignore-workspaces dev,test --var-files global.tfvars,env.tfvars
```

### After (Configuration-based)

```json
{
  "global": {
    "ignore_workspaces": ["dev", "test"],
    "var_files": ["global.tfvars", "env.tfvars"]
  }
}
```

```bash
solarboat plan
solarboat apply
```

## Troubleshooting

### Configuration Not Loading

- Check file permissions
- Verify file path is correct
- Ensure JSON syntax is valid

### Module Configuration Not Applied

If your module-specific settings (like `ignore_workspaces` or `workspace_var_files`) are not being applied:

- Ensure you're using `"modules"` as the top-level key (not `"projects"`)
- Verify the module path matches your directory structure relative to the config file
- Use relative paths like `"terraform/projects/webapp"` rather than absolute paths in your configuration
- Check that the module exists and contains `.tf` files

**Note**: As of version 0.8.3+, path normalization automatically handles absolute vs relative path mismatches.

### Unexpected Variable Files

- Check configuration precedence
- Verify workspace-specific settings
- Review CLI argument overrides

### Validation Warnings

- Create missing variable files
- Fix module paths
- Review workspace names

### Environment-Specific Config Not Found

- Verify `SOLARBOAT_ENV` is set correctly
- Check file naming convention
- Ensure files exist in the expected location
