# Solarboat CLI 🚀

[![Release](https://github.com/devqik/solarboat/actions/workflows/release.yml/badge.svg)](https://github.com/devqik/solarboat/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/solarboat)](https://crates.io/crates/solarboat)
[![Website](https://img.shields.io/website?url=https://solarboat.io)](https://solarboat.io)

> Built with love for Rust and infrastructure automation by [devqik](https://devqik.com)

Solarboat is a modern CLI for Infrastructure as Code (IaC) and GitOps workflows, providing intelligent Terraform operations, automatic dependency detection, and seamless stateful/stateless module handling.

<table>
<tr>
<td width="50%">

## Why "Solarboat"?

Inspired by the Ancient Egyptian solar boats that carried Pharaohs through their celestial journey, this CLI tool is your vessel through the complexities of infrastructure management. Let Solarboat handle the operational journey, so you can focus on writing code.

</td>
<td width="50%">
<img src="./icon.jpg" alt="Solarboat Logo" width="100%">
</td>
</tr>
</table>

---

## ✨ Features

- **Intelligent Terraform Operations**
  - Detects changed modules automatically
  - Handles stateful/stateless modules smartly
  - Propagates dependencies
  - Runs modules in parallel (with safety limits)
  - Detailed, readable reporting
  - Path-based filtering for targeted runs
- **Coming Soon**
  - AI agent
  - Automatic state management
  - Custom workflow automation

---

## 📦 Installation

**With Cargo (Recommended):**

```bash
cargo install solarboat
# Or install a specific version
cargo install solarboat --version 0.8.7
```

**From Release Binaries:**

```bash
curl -L https://github.com/devqik/solarboat/releases/latest/download/solarboat-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv solarboat /usr/local/bin/
```

**From Source:**

```bash
git clone https://github.com/devqik/solarboat.git
cd solarboat
cargo build
```

---

## 🛠️ Usage

### Common Commands

```bash
# Scan for changed Terraform modules
solarboat scan

# Scan a specific directory
solarboat scan --path ./terraform-modules

# Scan with custom default branch
solarboat scan --default-branch develop

# Plan Terraform changes
solarboat plan

# Plan in parallel
solarboat plan --parallel 4

# Save plans to directory
solarboat plan --output-dir ./terraform-plans

# Ignore workspaces
solarboat plan --ignore-workspaces dev,staging

# Plan all stateful modules
solarboat plan --all

# Apply changes (dry-run by default)
solarboat apply

# Apply for real
solarboat apply --dry-run=false

# Ignore workspaces
solarboat apply --ignore-workspaces prod,staging

# Apply all stateful modules
solarboat apply --all

# Real-time output
solarboat plan --watch

# Combine flags
solarboat plan --all --watch --var-files vars.tfvars
```

### Command Overview

- **scan**: Analyze repo for changed modules and dependencies. No changes made.
- **plan**: Generate Terraform plans for changed modules. Supports parallelism, workspace filtering, and output directory.
- **apply**: Apply changes to infrastructure. Dry-run by default, supports real-time output and workspace filtering.

#### Default Branch

- Compares changes against `main` by default. Use `--default-branch` to override.

#### Parallel Processing

- Use `--parallel N` (max 4) to process modules in parallel. Ex: `solarboat plan --parallel 3`
- In `--watch` mode, parallelism is forced to 1 for clean output.

#### Watch Mode

- `--watch` streams real-time Terraform output. Great for debugging and monitoring.
- Without `--watch`, operations run silently for CI/CD cleanliness.

#### Timeout Handling

- Initialization: 5 min
- Planning: 10 min
- Apply: 30 min

---

## ⚙️ Configuration

Solarboat supports flexible configuration via JSON files.

**Quick Start:**

1. Create `solarboat.json` in your project root:

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

2. Run Solarboat as usual. It auto-loads your config.

**Environment-Specific Config:**

```bash
SOLARBOAT_ENV=prod solarboat plan
```

Solarboat will look for `solarboat.prod.json` if set.

**Config Precedence:**

1. CLI arguments (highest)
2. Module config
3. Global config
4. Defaults (lowest)

**More:** See [CONFIGURATION.md](./CONFIGURATION.md) for full docs.

---

## 🧑‍💻 GitHub Actions Integration

Solarboat comes with a GitHub Action for CI/CD automation.

### **GitHub Token Requirements**

The GitHub token is **optional** and only needed for:

- 📝 **PR Comments**: Automatic plan summaries posted to pull requests
- 📊 **Enhanced Integration**: Access to GitHub API features

**Most common scenarios:**

- ✅ **Basic Usage**: No token required for core functionality (scan, plan, apply)
- ✅ **PR Comments**: Use `${{ secrets.GITHUB_TOKEN }}` (automatically provided by GitHub)
- ⚠️ **Custom Permissions**: Use custom token only if default permissions aren't sufficient

### **Basic Workflow (Minimal Setup):**

```yaml
name: Infrastructure CI/CD
on: [push, pull_request]

jobs:
  terraform:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Scan for Changes
        uses: devqik/solarboat@v0.8.7
        with:
          command: scan

      - name: Plan Infrastructure
        if: github.event_name == 'pull_request'
        uses: devqik/solarboat@v0.8.7
        with:
          command: plan
          output-dir: terraform-plans
          # Add token for PR comments
          github_token: ${{ secrets.GITHUB_TOKEN }}

      - name: Apply Changes
        if: github.ref == 'refs/heads/main'
        uses: devqik/solarboat@v0.8.7
        with:
          command: apply
          apply-dryrun: false
```

### **Production Workflow (Full Features):**

```yaml
name: Infrastructure Automation
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

permissions:
  contents: read
  pull-requests: write # For PR comments

jobs:
  infrastructure:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Plan Infrastructure Changes
        if: github.event_name == 'pull_request'
        uses: devqik/solarboat@v0.8.7
        with:
          command: plan
          config: ./infrastructure/solarboat.json
          terraform-version: "1.8.0"
          solarboat-version: "v0.8.7"
          parallel: 3
          ignore-workspaces: dev,test
          output-dir: terraform-plans
          github_token: ${{ secrets.GITHUB_TOKEN }}

      - name: Apply Infrastructure Changes
        if: github.ref == 'refs/heads/main'
        uses: devqik/solarboat@v0.8.7
        with:
          command: apply
          apply-dryrun: false
          config: ./infrastructure/solarboat.json
          terraform-version: "1.8.0"
          parallel: 2
          continue-on-error: false
```

### **Pipeline-Supplied Commits (New Feature)**

Solarboat now supports intelligent change detection with pipeline-supplied commit information for more reliable CI/CD workflows.

#### **Automatic Detection (Recommended)**

The GitHub Action automatically detects the appropriate commits based on the event:

- **Pull Requests**: Uses `base.sha` and `head.sha` from the PR
- **Main Branch Pushes**: Uses `before` and `after` commit hashes
- **Local Mode**: Falls back to checking recent commits (configurable)

#### **Manual Commit Specification**

For advanced use cases, you can manually specify commit ranges:

```yaml
- name: Custom Commit Comparison
  uses: devqik/solarboat@v0.8.7
  with:
    command: plan
    base-commit: abc1234
    head-commit: def5678
    base-branch: main
    head-branch: feature/new-module
```

#### **Local Development Mode**

When no commit information is provided, Solarboat runs in local mode:

```yaml
- name: Local Development Mode
  uses: devqik/solarboat@v0.8.7
  with:
    command: plan
    recent-commits: 5 # Check last 5 commits for changes
```

### **Action Inputs:**

| Input               | Description                              | Default           | Required |
| ------------------- | ---------------------------------------- | ----------------- | -------- |
| `command`           | Command to run (`scan`, `plan`, `apply`) | -                 | ✅       |
| `github_token`      | GitHub token for PR comments             | -                 | ❌       |
| `config`            | Path to Solarboat configuration file     | auto-detect       | ❌       |
| `output-dir`        | Directory for plan files                 | `terraform-plans` | ❌       |
| `apply-dryrun`      | Run apply in dry-run mode                | `true`            | ❌       |
| `ignore-workspaces` | Comma-separated workspaces to ignore     | -                 | ❌       |
| `var-files`         | Comma-separated var files to use         | -                 | ❌       |
| `path`              | Directory to scan for modules            | `.`               | ❌       |
| `all`               | Process all stateful modules             | `false`           | ❌       |
| `watch`             | Show real-time output                    | `false`           | ❌       |
| `parallel`          | Number of parallel processes (max 4)     | `1`               | ❌       |
| `default-branch`    | Default git branch for comparisons       | `main`            | ❌       |
| `recent-commits`    | Recent commits to check (local mode)     | `5`               | ❌       |
| `base-commit`       | Base commit SHA for comparison           | auto-detect       | ❌       |
| `head-commit`       | Head commit SHA for comparison           | auto-detect       | ❌       |
| `base-branch`       | Base branch name for comparison          | auto-detect       | ❌       |
| `head-branch`       | Head branch name for comparison          | auto-detect       | ❌       |
| `solarboat-version` | Solarboat CLI version to use             | `latest`          | ❌       |
| `terraform-version` | Terraform version to use                 | `latest`          | ❌       |
| `continue-on-error` | Continue workflow on Solarboat failure   | `false`           | ❌       |

### **Action Outputs:**

| Output            | Description                             |
| ----------------- | --------------------------------------- |
| `result`          | Command result (`success` or `failure`) |
| `plans-path`      | Path to generated Terraform plans       |
| `changed-modules` | Number of changed modules detected      |

### **Advanced Examples:**

**Conditional workflows based on outputs:**

```yaml
- name: Plan Infrastructure
  id: plan
  uses: devqik/solarboat@v0.8.7
  with:
    command: plan
    github_token: ${{ secrets.GITHUB_TOKEN }}

- name: Notify on Changes
  if: steps.plan.outputs.changed-modules != '0'
  run: |
    echo "🚨 ${{ steps.plan.outputs.changed-modules }} modules changed!"
    echo "Plans available at: ${{ steps.plan.outputs.plans-path }}"
```

**Multi-environment with configuration:**

```yaml
- name: Plan Staging
  uses: devqik/solarboat@v0.8.7
  with:
    command: plan
    config: ./configs/solarboat.staging.json
    path: ./environments/staging

- name: Plan Production
  uses: devqik/solarboat@v0.8.7
  with:
    command: plan
    config: ./configs/solarboat.prod.json
    path: ./environments/production
    ignore-workspaces: dev,staging,test
```

**Error handling:**

```yaml
- name: Apply with Error Handling
  uses: devqik/solarboat@v0.8.7
  with:
    command: apply
    apply-dryrun: false
    continue-on-error: true

- name: Handle Failures
  if: failure()
  run: |
    echo "🚨 Infrastructure apply failed!"
    echo "Check logs and retry manually"
```

### **Permissions**

For PR comments, ensure your workflow has the correct permissions:

```yaml
permissions:
  contents: read # Read repository contents
  pull-requests: write # Comment on pull requests
```

**Note**: `${{ secrets.GITHUB_TOKEN }}` is automatically provided by GitHub with repository access. Custom tokens are only needed for cross-repository access or enhanced permissions.

---

## Contributing 🤝

Contributions are welcome! Please feel free to submit a Pull Request.

## License 📄

This project is licensed under the BSD-3-Clause License - see the [LICENSE](LICENSE) file for details.

## Support 💬

- Issues: [GitHub Issues](https://github.com/devqik/solarboat/issues)
- Discussions: [GitHub Discussions](https://github.com/devqik/solarboat/discussions)
- Documentation: [Wiki](https://github.com/devqik/solarboat/wiki)

## Acknowledgments 🙏

This project needs your support! If you find Solarboat CLI useful, please consider:

- ⭐ Starring the project on GitHub
- 🛠️ Contributing with code, documentation, or bug reports
- 💡 Suggesting new features or improvements
- 🌟 Sharing it with other developers

Your support will help make this project better and encourage its continued development.

~ [@devqik](https://devqik.com) (Creator)
