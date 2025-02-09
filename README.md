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
cargo install solarboat --version 0.1.2
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

# Apply Terraform changes
solarboat apply

# Apply Terraform changes in dry-run mode (runs plan instead)
solarboat apply --dry-run
```

### Command Details

#### Scan
The scan command analyzes your repository for changed Terraform modules and their dependencies. It:
- Detects modified `.tf` files
- Builds a dependency graph
- Identifies affected modules
- Does not generate any plans or make changes

#### Plan
The plan command generates Terraform plans for changed modules. It:
- Runs `terraform init` for each module
- Generates detailed plans
- Optionally saves plans to a specified directory
- Shows what changes would be made

#### Apply
The apply command implements the changes to your infrastructure. It:
- Runs `terraform init` for each module
- Supports dry-run mode for safety
- Automatically approves changes in CI/CD
- Shows real-time progress

### Module Types

Solar Boat CLI recognizes two types of Terraform modules:

- **Stateful Modules**: Modules that manage actual infrastructure state (contain backend configuration)
- **Stateless Modules**: Reusable modules without state (no backend configuration)

When changes are detected in stateless modules, the CLI automatically identifies and processes any stateful modules that depend on them.

### GitHub Actions Integration

Solar Boat provides a GitHub Action for seamless integration with your CI/CD pipeline. The action can scan for changes, generate Terraform plans, and automatically comment on pull requests with the results.

#### Basic Usage

```yaml
name: Infrastructure Management

on:
  pull_request:
    branches: [ main ]
  push:
    branches: [ main ]

jobs:
  infrastructure:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0  # Important for detecting changes

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
          apply_dry_run: false  # Set to true for dry-run mode
          github_token: ${{ secrets.GITHUB_TOKEN }}
```

This workflow will:
1. Scan for changes
2. Plan infrastructure changes
3. Comment on the PR with results
4. Apply changes when merged to main

#### Action Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `command` | Command to run (`scan`, `plan`, or `apply`) | No | `scan` |
| `plan_output_dir` | Directory for terraform plan outputs | No | `terraform-plans` |
| `apply_dry_run` | Enable or disable solarboat apply in dry-run mode | No | `true` |
| `github_token` | GitHub token for PR comments | Yes | N/A |

#### Features

- **Automatic Plan Generation**: Generates Terraform plans for changed modules
- **PR Comments**: Automatically comments on pull requests with plan results
- **Artifact Upload**: Uploads plan files as artifacts for review
- **Plan Retention**: Keeps plan artifacts for 5 days
- **Change Detection**: Smart detection of changed Terraform modules

#### PR Comment Example

When a plan is generated, the action will automatically comment on the pull request with:
- Summary of changes detected
- Links to plan artifacts
- Next steps for review
- Retention period information

#### Security Note

The action requires `GITHUB_TOKEN` for commenting on PRs and managing artifacts. This token is automatically provided by GitHub Actions, but you need to pass it explicitly to the action.

## Contributing ��

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.

## License 📄

This project is licensed under the BSD-3-Clause License - see the [LICENSE](LICENSE) file for details.

## Support 💬

- Issues: [GitHub Issues](https://github.com/devqik/solarboat/issues)
- Discussions: [GitHub Discussions](https://github.com/devqik/solarboat/discussions)
- Documentation: [Wiki](https://github.com/devqik/solarboat/wiki)

## Acknowledgments 🙏

Special thanks to all contributors who help make this project better! Whether you're fixing bugs, improving documentation, or suggesting features, your contributions are greatly appreciated.

~ @devqik (Creator)
