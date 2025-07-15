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
<img src="./icon.jpg" alt="Solar Boat Logo" width="100%">
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
  - Ephemeral environments on Kubernetes
  - Custom workflow automation

---

## 📦 Installation

**With Cargo (Recommended):**

```bash
cargo install solarboat
# Or install a specific version
cargo install solarboat --version 0.7.2
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
# Scan for changed Terraform modules	solarboat scan
# Scan a specific directory		solarboat scan --path ./terraform-modules
# Scan with custom default branch	solarboat scan --default-branch develop

# Plan Terraform changes		solarboat plan
# Plan in parallel			solarboat plan --parallel 4
# Save plans to directory		solarboat plan --output-dir ./terraform-plans
# Ignore workspaces			solarboat plan --ignore-workspaces dev,staging
# Plan all stateful modules		solarboat plan --all

# Apply changes (dry-run by default)	solarboat apply
# Apply for real			solarboat apply --dry-run=false
# Ignore workspaces			solarboat apply --ignore-workspaces prod,staging
# Apply all stateful modules		solarboat apply --all

# Real-time output			solarboat plan --watch
# Combine flags			solarboat plan --all --watch --var-files vars.tfvars
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
    "workspace_var_files": { "prod": ["prod.tfvars"] }
  },
  "modules": {
    "infrastructure/networking": {
      "ignore_workspaces": ["test"],
      "var_files": ["networking.tfvars"],
      "workspace_var_files": { "prod": ["networking-prod.tfvars"] }
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

**Basic Workflow:**

```yaml
- uses: actions/checkout@v3
  with: { fetch-depth: 0 }
- name: Scan for Changes
  uses: devqik/solarboat@v0.7.2
  with:
    command: scan
    github_token: ${{ secrets.GITHUB_TOKEN }}
- name: Plan Infrastructure Changes
  uses: devqik/solarboat@v0.7.2
  with:
    command: plan
    output_dir: terraform-plans
    github_token: ${{ secrets.GITHUB_TOKEN }}
- name: Apply Infrastructure Changes
  if: github.ref == 'refs/heads/main'
  uses: devqik/solarboat@v0.7.2
  with:
    command: apply
    apply_dry_run: false
    github_token: ${{ secrets.GITHUB_TOKEN }}
```

**Action Inputs:**
| Input | Description | Default |
|---------------------|----------------------------------------------------|-------------------|
| `command` | `scan`, `plan`, or `apply` | - |
| `output-dir` | Directory for plan files | terraform-plans |
| `apply-dryrun` | Run apply in dry-run mode | true |
| `ignore-workspaces` | Comma-separated workspaces to ignore | '' |
| `path` | Directory to scan for modules | . |
| `all` | Process all stateful modules | false |
| `watch` | Real-time output | false |
| `parallel` | Number of parallel processes (max 4) | 1 |
| `default-branch` | Default git branch | main |

**Examples:**

- Plan with workspace filtering:
  ```yaml
  - name: Apply Changes
    uses: devqik/solarboat@v0.7.2
    with:
      command: apply
      ignore_workspaces: dev,staging,test
      apply_dry_run: true
  ```
- Targeted operations:
  ```yaml
  - name: Plan Specific Modules
    uses: devqik/solarboat@v0.7.2
    with:
      command: plan
      path: ./terraform-modules/production
      plan_output_dir: prod-plans
  ```
- Real-time output:
  ```yaml
  - name: Plan with Real-time Output
    uses: devqik/solarboat@v0.7.2
    with:
      command: plan
      watch: true
      plan_output_dir: terraform-plans
  ```

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
