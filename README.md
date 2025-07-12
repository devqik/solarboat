# Solar Boat CLI 🚀

[![Release](https://github.com/devqik/solarboat/actions/workflows/release.yml/badge.svg)](https://github.com/devqik/solarboat/actions/workflows/release.yml)

Solar Boat is a command-line interface tool designed for Infrastructure as Code (IaC) and GitOps workflows. It provides intelligent Terraform operations management with automatic dependency detection and stateful/stateless module handling.

<table>
<tr>
<td width="50%">

## Why "Solar Boat"?

Inspired by the Ancient Egyptian Solar Boats that carried Pharaohs through their celestial journey, this CLI tool serves
as a modern vessel that carries developers through the complexities of operations and infrastructure management. Just as
the ancient boats handled the journey through the afterlife so the Pharaoh didn't have to worry about it, Solar Boat CLI
handles the operational journey so developers can focus on what they do best - writing code.

</td>
<td width="50%">
<img src="./icon.jpg" alt="Solar Boat Logo" width="100%">
</td>
</tr>
</table>

## Features ✨

### Current Features

- **Intelligent Terraform Operations**
  - Automatic detection of changed modules
  - Smart handling of stateful and stateless modules
  - Automatic dependency propagation
  - Parallel execution of independent modules
  - Detailed operation reporting
  - Path-based filtering for targeted operations

### Coming Soon

- Self-service ephemeral environments on Kubernetes
- Infrastructure management and deployment
- Custom workflow automation

## Installation 📦

### Using Cargo (Recommended)

```bash
# Install the latest version
cargo install solarboat

# Install a specific version
cargo install solarboat --version 0.5.3
```

### Building from Source

```bash
git clone https://github.com/devqik/solarboat.git
cd solarboat
cargo build
```

## Usage 🛠️

### Basic Commands

```bash
# Scan for changed Terraform modules
solarboat scan

# Scan modules in a specific directory
solarboat scan --path ./terraform-modules

# Plan Terraform changes
solarboat plan

# Plan and save outputs to a specific directory
solarboat plan --output-dir ./terraform-plans

# Plan changes while ignoring specific workspaces
solarboat plan --ignore-workspaces dev,staging

# Process all stateful modules regardless of changes
solarboat plan --all

# Apply Terraform changes (dry-run mode by default)
solarboat apply

# Apply actual Terraform changes
solarboat apply --dry-run=false

# Apply changes while ignoring specific workspaces
solarboat apply --ignore-workspaces prod,staging

# Process all stateful modules regardless of changes
solarboat apply --all
```

### Command Details

#### Scan

The scan command analyzes your repository for changed Terraform modules and their dependencies. It:

- Detects modified `.tf` files
- Builds a dependency graph
- Identifies affected modules
- Filters modules based on the specified path
- Does not generate any plans or make changes
- Can process all stateful modules with `--all` flag

#### Plan

The plan command generates Terraform plans for changed modules. It:

- Runs `terraform init` for each module
- Detects and handles multiple workspaces
- Generates detailed plans for each workspace
- Optionally skips specified workspaces
- Optionally saves plans to a specified directory
- Shows what changes would be made
- Filters modules based on the specified path
- Can process all stateful modules with `--all` flag
- Saves plans as Markdown files for better readability

#### Apply

The apply command implements the changes to your infrastructure. It:

- Runs `terraform init` for each module
- Detects and handles multiple workspaces
- Supports dry-run mode for safety
- Optionally skips specified workspaces
- Automatically approves changes in CI/CD
- Shows real-time progress
- Filters modules based on the specified path
- Can process all stateful modules with `--all` flag

### Module Types

Solar Boat CLI recognizes two types of Terraform modules:

- **Stateful Modules**: Modules that manage actual infrastructure state (contain backend configuration)
- **Stateless Modules**: Reusable modules without state (no backend configuration)

When changes are detected in stateless modules, the CLI automatically identifies and processes any stateful modules that depend on them.

### Workspace Handling

Solar Boat CLI provides intelligent workspace management for Terraform modules:

- **Automatic Detection**: Automatically detects if a module has multiple workspaces
- **Individual Processing**: Processes each workspace separately for both plan and apply operations
- **Workspace Filtering**: Allows skipping specific workspaces using the `--ignore-workspaces` flag
- **Default Workspace**: Handles modules with only the default workspace appropriately

### Path-based Filtering

Solar Boat CLI supports path-based filtering for all commands:

- **Targeted Operations**: Use `--path` to target specific modules or directories
- **Recursive Scanning**: Automatically discovers all modules within the specified path
- **Dependency Awareness**: Maintains dependency relationships even when filtering by path
- **Combined with --all**: Can be used together with `--all` to process all modules in a specific path

### Configuration Files

Solar Boat CLI supports configuration files to manage global and module-specific settings for Terraform workspaces and variable files.

#### Quick Start

1. Create a `solarboat.json` or `solarboat.yml` in your project root:

```json
{
  "global": {
    "ignore_workspaces": ["dev", "test"],
    "var_files": ["global.tfvars"],
    "workspace_var_files": {
      "prod": ["prod.tfvars"]
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
}
```

2. Run Solar Boat as usual - it will automatically load your configuration:

```bash
solarboat plan
solarboat apply
```

#### Environment-Specific Configuration

Use different config files for different environments by setting the `SOLARBOAT_ENV` environment variable:

```bash
SOLARBOAT_ENV=dev solarboat plan
SOLARBOAT_ENV=prod solarboat apply
```

Solar Boat will look for files like `solarboat.dev.json`, `solarboat.prod.yml`, etc. If the environment-specific file is not found, it falls back to the default config file.

#### Configuration Options

- **Use a specific config file**:

  ```bash
  solarboat --config /path/to/solarboat.json plan
  ```

- **Disable config file loading**:

  ```bash
  solarboat --no-config plan
  ```

- **Override config file settings with CLI options**:
  ```bash
  solarboat plan --ignore-workspaces dev,test --var-files custom.tfvars
  ```

#### Configuration Precedence

1. CLI arguments (highest priority)
2. Module-specific config
3. Global config
4. Defaults (lowest priority)

#### Example: Workspace-Specific Variable Files

```json
{
  "global": {
    "workspace_var_files": {
      "dev": ["dev-secrets.tfvars"],
      "prod": ["prod-secrets.tfvars"]
    }
  }
}
```

#### Validation

Solar Boat validates your configuration and warns about:

- Missing module paths
- Missing variable files
- Reserved workspace names (`default`, `terraform`)

Run `solarboat scan` to check your configuration before running plan/apply.

> **Note**: For complete configuration documentation, see [CONFIGURATION.md](./CONFIGURATION.md).

### GitHub Actions Integration

Solar Boat provides a GitHub Action for seamless integration with your CI/CD pipeline. The action can scan for changes, generate Terraform plans, and automatically comment on pull requests with the results.

#### Basic Usage

```yaml
name: Infrastructure Management

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

jobs:
  infrastructure:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0 # Important for detecting changes

      - name: Scan for Changes
        if: github.event_name == 'pull_request'
        uses: devqik/solarboat-action@latest
        with:
          command: scan
          github_token: ${{ secrets.GITHUB_TOKEN }}

      - name: Plan Infrastructure Changes
        if: github.event_name == 'pull_request'
        uses: devqik/solarboat-action@latest
        with:
          command: plan
          output_dir: terraform-plans
          github_token: ${{ secrets.GITHUB_TOKEN }}

      - name: Apply Infrastructure Changes
        if: github.ref == 'refs/heads/main'
        uses: devqik/solarboat-action@latest
        with:
          command: apply
          apply_dry_run: false # Set to true for dry-run mode
          github_token: ${{ secrets.GITHUB_TOKEN }}
```

This workflow will:

1. Scan for changes
2. Plan infrastructure changes
3. Comment on the PR with results
4. Apply changes when merged to main

#### Action Inputs

| Input               | Description                                        | Required | Default           |
| ------------------- | -------------------------------------------------- | -------- | ----------------- |
| `command`           | Command to run (`scan`, `plan`, or `apply`)        | Yes      | -                 |
| `plan_output_dir`   | Directory to save Terraform plan files             | No       | `terraform-plans` |
| `apply_dry_run`     | Run apply in dry-run mode                          | No       | `true`            |
| `ignore_workspaces` | Comma-separated list of workspaces to ignore       | No       | `''`              |
| `path`              | Root directory to scan for Terraform modules       | No       | `'.'`             |
| `all`               | Process all stateful modules regardless of changes | No       | `false`           |

#### Workflow Examples

**Basic Scan and Plan:**

```yaml
- name: Scan Changes
  uses: devqik/solarboat@v0.5.3
  with:
    command: scan

- name: Plan Changes
  uses: devqik/solarboat@v0.5.3
  with:
    command: plan
    plan_output_dir: my-plans
```

**Apply with Workspace Filtering:**

```yaml
- name: Apply Changes
  uses: devqik/solarboat@v0.5.3
  with:
    command: apply
    ignore_workspaces: dev,staging,test
    apply_dry_run: true
```

**Targeted Operations with Path Filtering:**

```yaml
- name: Plan Specific Modules
  uses: devqik/solarboat@v0.5.3
  with:
    command: plan
    path: ./terraform-modules/production
    plan_output_dir: prod-plans
```

**Complete Workflow with Conditions:**

```yaml
jobs:
  terraform:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # Run on all branches
      - name: Plan Changes
        uses: devqik/solarboat@v0.5.3
        with:
          command: plan
          plan_output_dir: terraform-plans
          ignore_workspaces: dev,staging

      # Run only on main branch
      - name: Apply Changes
        if: github.ref == 'refs/heads/main'
        uses: devqik/solarboat@v0.5.3
        with:
          command: apply
          apply_dry_run: false
```

## Contributing 🤝

Contributions are welcome! Please feel free to submit a Pull Request.

## License 📄

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support 💬

- Issues: [GitHub Issues](https://github.com/devqik/solarboat/issues)
- Discussions: [GitHub Discussions](https://github.com/devqik/solarboat/discussions)
- Documentation: [Wiki](https://github.com/devqik/solarboat/wiki)

## Acknowledgments 🙏

This project needs your support! If you find Solar Boat CLI useful, please consider:

- ⭐ Starring the project on GitHub
- 🛠️ Contributing with code, documentation, or bug reports
- 💡 Suggesting new features or improvements
- 🌟 Sharing it with other developers

Your support will help make this project better and encourage its continued development.

~ [@devqik](https://github.com/devqik) (Creator)
