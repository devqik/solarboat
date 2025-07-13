# Solar Boat CLI 🚀

[![Release](https://github.com/devqik/solarboat/actions/workflows/release.yml/badge.svg)](https://github.com/devqik/solarboat/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/solarboat)](https://crates.io/crates/solarboat)
[![Website](https://img.shields.io/website?url=https://solarboat.io)](https://solarboat.io)

> Built with love for Rust and infrastructure automation by [devqik](https://devqik.com)

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
cargo install solarboat --version 0.7.1
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

# Scan with custom default branch
solarboat scan --default-branch master
solarboat scan --default-branch develop

# Plan Terraform changes
solarboat plan

# Plan with parallel processing (up to 4 modules at once)
solarboat plan --parallel 4

# Plan and save outputs to a specific directory
solarboat plan --output-dir ./terraform-plans

# Plan changes while ignoring specific workspaces
solarboat plan --ignore-workspaces dev,staging

# Process all stateful modules regardless of changes
solarboat plan --all

# Plan with custom default branch
solarboat plan --default-branch master
solarboat plan --default-branch develop

# Apply Terraform changes (dry-run mode by default)
solarboat apply

# Apply actual Terraform changes
solarboat apply --dry-run=false

# Apply changes while ignoring specific workspaces
solarboat apply --ignore-workspaces prod,staging

# Process all stateful modules regardless of changes
solarboat apply --all

# Apply with custom default branch
solarboat apply --default-branch master
solarboat apply --default-branch develop

# Watch background Terraform operations with real-time output
solarboat plan --watch
solarboat apply --watch

# Combine watch mode with other flags
solarboat plan --all --watch --var-files vars.tfvars
solarboat apply --dry-run=false --watch --ignore-workspaces dev,staging
```

### Command Details

#### Scan

The scan command analyzes your repository for changed Terraform modules and their dependencies. It:

- Detects modified `.tf` files by comparing against a default branch
- Builds a dependency graph
- Identifies affected modules
- Filters modules based on the specified path
- Does not generate any plans or make changes
- Can process all stateful modules with `--all` flag
- Supports custom default branch names with `--default-branch`

**Default Branch Configuration:**

By default, Solar Boat compares changes against the `main` branch. You can specify a different default branch:

```bash
# Use 'master' as the default branch
solarboat scan --default-branch master

# Use 'develop' as the default branch
solarboat scan --default-branch develop

# Use 'main' (default)
solarboat scan --default-branch main
```

This is useful for repositories that use different default branch names (e.g., `master` instead of `main`).

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
- Supports custom default branch names with `--default-branch`

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
- Supports custom default branch names with `--default-branch`

### Background Operations with `--watch`

Solar Boat CLI supports background Terraform operations with real-time status updates using the `--watch` flag.

#### Silent Mode (Default)

By default, Terraform operations run silently in the background:

```bash
# Terraform output is hidden until completion
solarboat plan
solarboat apply
```

**Benefits:**

- Faster execution for CI/CD pipelines
- Clean output focused on results
- Reduced noise in automated environments

#### Watch Mode

When using the `--watch` flag, Terraform operations display real-time output:

```bash
# Real-time Terraform output display
solarboat plan --watch
solarboat apply --watch
```

**Benefits:**

- Real-time progress monitoring
- Immediate feedback on long-running operations
- Useful for debugging and troubleshooting
- See Terraform output as it happens

**Important Note:** When `--watch` is enabled, the `--parallel` argument is automatically forced to 1 to maintain readable real-time output. This ensures that multiple Terraform operations don't interfere with each other's output display.

```bash
# Even if you specify --parallel 4, watch mode will force it to 1
solarboat plan --watch --parallel 4
# This will actually run with --parallel 1 for clean real-time output
```

#### Timeout Handling

Background operations include automatic timeout handling:

- **Initialization**: 5-minute timeout
- **Planning**: 10-minute timeout
- **Application**: 30-minute timeout

#### Combining with Other Flags

The `--watch` flag works seamlessly with all other flags:

```bash
# Watch mode with path filtering
solarboat plan --path ./production --watch

# Watch mode with workspace filtering
solarboat apply --ignore-workspaces dev,staging --watch

# Watch mode with var files
solarboat plan --var-files prod.tfvars --watch

# Watch mode with all modules
solarboat apply --all --watch
```

### Parallel Processing with --parallel

Solar Boat CLI supports safe, robust parallel processing for both `plan` and `apply` commands. You can control the number of modules processed in parallel using the `--parallel` flag:

```bash
# Plan or apply up to 4 modules in parallel
solarboat plan --parallel 4
solarboat apply --parallel 4
```

- The value for `--parallel` is clamped to a maximum of 4 to prevent system overload.
- If you specify more modules than the parallel limit, Solar Boat will queue them and process as threads become available.
- This ensures efficient use of system resources while maintaining safety and reliability.
- The default is `--parallel 1` (sequential processing).

**Example:**

If you have 10 changed modules and run `solarboat plan --parallel 3`, Solar Boat will process 3 modules at a time, automatically queuing the rest and starting new ones as others finish.

**Safety:**

- The parallel system is designed to avoid resource exhaustion and crashing your machine.
- All background processes are managed and cleaned up safely.
- Error propagation and graceful shutdown are built-in.

See the [source code](src/utils/parallel_processor.rs) for implementation details.

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

1. Create a `solarboat.json` in your project root:

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

Solar Boat will look for files like `solarboat.dev.json`, etc. If the environment-specific file is not found, it falls back to the default config file.

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
        uses: devqik/solarboat@v0.7.1
        with:
          command: scan
          github_token: ${{ secrets.GITHUB_TOKEN }}

      - name: Plan Infrastructure Changes
        if: github.event_name == 'pull_request'
        uses: devqik/solarboat@v0.7.1
        with:
          command: plan
          output_dir: terraform-plans
          github_token: ${{ secrets.GITHUB_TOKEN }}

      - name: Apply Infrastructure Changes
        if: github.ref == 'refs/heads/main'
        uses: devqik/solarboat@v0.7.1
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
| `watch`             | Enable real-time Terraform output display          | No       | `false`           |
| `parallel`          | The number of processes to run in parallel         | No       | `1`               |
| `default-branch`    | Change the default git branch                      | No       | `main`            |

#### Workflow Examples

**Basic Scan and Plan:**

```yaml
- name: Scan Changes
  uses: devqik/solarboat@v0.7.1
  with:
    command: scan

- name: Plan Changes
  uses: devqik/solarboat@v0.7.1
  with:
    command: plan
    plan_output_dir: my-plans
```

**Apply with Workspace Filtering:**

```yaml
- name: Apply Changes
  uses: devqik/solarboat@v0.7.1
  with:
    command: apply
    ignore_workspaces: dev,staging,test
    apply_dry_run: true
```

**Targeted Operations with Path Filtering:**

```yaml
- name: Plan Specific Modules
  uses: devqik/solarboat@v0.7.1
  with:
    command: plan
    path: ./terraform-modules/production
    plan_output_dir: prod-plans
```

**Watch Mode for Real-time Output:**

```yaml
- name: Plan with Real-time Output
  uses: devqik/solarboat@v0.7.1
  with:
    command: plan
    watch: true
    plan_output_dir: terraform-plans
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
        uses: devqik/solarboat@v0.7.1
        with:
          command: plan
          plan_output_dir: terraform-plans
          ignore_workspaces: dev,staging

      # Run only on main branch
      - name: Apply Changes
        if: github.ref == 'refs/heads/main'
        uses: devqik/solarboat@v0.7.1
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
