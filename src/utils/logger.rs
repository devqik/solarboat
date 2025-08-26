use colored::*;
use std::io::{self, Write};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex, LazyLock};
use std::thread;

/// Log levels for different types of output
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum LogLevel {
    Silent,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Progress indicator for long-running operations
pub struct Progress {
    message: String,
    start_time: Instant,
    is_complete: Arc<Mutex<bool>>,
    spinner_thread: Option<thread::JoinHandle<()>>,
}

impl Progress {
    pub fn new(message: &str) -> Self {
        let message = message.to_string();
        let start_time = Instant::now();
        let is_complete = Arc::new(Mutex::new(false));
        
        // Start spinner thread
        let is_complete_clone = Arc::clone(&is_complete);
        let message_clone = message.clone();
        let spinner_thread = thread::spawn(move || {
            let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut i = 0;
            
            while !*is_complete_clone.lock().unwrap() {
                print!("\r{} {} {}", 
                    spinner[i % spinner.len()].blue(),
                    message_clone.cyan(),
                    "  ".clear()
                );
                io::stdout().flush().ok();
                thread::sleep(Duration::from_millis(100));
                i += 1;
            }
        });
        
        Self {
            message,
            start_time,
            is_complete,
            spinner_thread: Some(spinner_thread),
        }
    }
    
    pub fn complete(mut self, success: bool) {
        *self.is_complete.lock().unwrap() = true;
        
        // Wait for spinner thread to finish
        if let Some(handle) = self.spinner_thread.take() {
            let _ = handle.join();
        }
        
        let duration = self.start_time.elapsed();
        let duration_str = format_duration(duration);
        
        if success {
            println!("\r{} {} {} ({})", 
                "✓".green().bold(),
                self.message.cyan(),
                "completed".green(),
                duration_str.dimmed()
            );
        } else {
            println!("\r{} {} {} ({})", 
                "✗".red().bold(),
                self.message.cyan(),
                "failed".red(),
                duration_str.dimmed()
            );
        }
    }
}

/// Main logger struct
pub struct Logger {
    level: LogLevel,
    quiet: bool,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl Logger {
    pub fn new() -> Self {
        Self {
            level: LogLevel::Info,
            quiet: false,
        }
    }
    
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }
    
    pub fn quiet(mut self) -> Self {
        self.quiet = true;
        self
    }
    
    /// Print a section header with enhanced styling
    pub fn section(&self, title: &str) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        println!("\n{} {}", "▶".blue().bold(), title.cyan().bold());
        println!("{}", "─".repeat(title.len() + 2).blue());
    }
    
    /// Print a subsection with better visual hierarchy
    pub fn subsection(&self, title: &str) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        println!("{} {}", "▸".blue(), title.cyan());
    }
    
    /// Print success message with enhanced styling
    pub fn success(&self, message: &str) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        println!("{} {}", "✓".green().bold(), message.green());
    }
    
    /// Print error message with enhanced styling
    pub fn error(&self, message: &str) {
        if self.quiet || self.level < LogLevel::Error {
            return;
        }
        eprintln!("{} {}", "✗".red().bold(), message.red());
    }
    
    /// Print warning message with enhanced styling
    pub fn warn(&self, message: &str) {
        if self.quiet || self.level < LogLevel::Warn {
            return;
        }
        println!("{} {}", "⚠".yellow().bold(), message.yellow());
    }
    
    /// Print info message with enhanced styling
    pub fn info(&self, message: &str) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        println!("{} {}", "ℹ".blue().bold(), message.blue());
    }
    
    /// Print debug message with enhanced styling
    pub fn debug(&self, message: &str) {
        if self.quiet || self.level < LogLevel::Debug {
            return;
        }
        println!("{} {}", "🔍".dimmed(), message.dimmed());
    }
    
    /// Print a list of items with enhanced styling
    pub fn list(&self, items: &[&str], title: Option<&str>) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        if let Some(title) = title {
            println!("{}", title.cyan().bold());
        }
        
        for item in items {
            println!("  {} {}", "•".blue(), item);
        }
    }
    
    /// Print a table-like structure with enhanced styling
    pub fn table(&self, rows: &[(&str, &str)]) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        let max_key_len = rows.iter().map(|(key, _)| key.len()).max().unwrap_or(0);
        
        for (key, value) in rows {
            println!("  {:<width$} {}", 
                key.cyan(), 
                value,
                width = max_key_len
            );
        }
    }
    
    /// Print a summary box with enhanced styling
    pub fn summary(&self, title: &str, items: &[(&str, &str)]) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        // Calculate the maximum width needed for the box
        let mut max_width = title.len();
        for (key, value) in items {
            let line_width = key.len() + 2 + value.len(); // "key: value" format
            max_width = max_width.max(line_width);
        }
        
        // Ensure minimum width and add padding
        max_width = max_width.max(20);
        let border = "─".repeat(max_width + 2);
        
        println!("\n┌{}┐", border.blue());
        println!("│ {:<width$} │", title.cyan().bold(), width = max_width);
        println!("├{}┤", border.blue());
        
        for (key, value) in items {
            println!("│ {:<key_width$}: {:<value_width$} │", 
                key.cyan(), 
                value,
                key_width = key.len(),
                value_width = max_width - key.len() - 2
            );
        }
        
        println!("└{}┘", border.blue());
    }
    
    /// Start a progress indicator
    pub fn progress(&self, message: &str) -> Option<Progress> {
        if self.quiet || self.level < LogLevel::Info {
            return None;
        }
        Some(Progress::new(message))
    }
    
    /// Print a command being executed with enhanced styling
    pub fn command(&self, cmd: &str, args: &[&str]) {
        if self.quiet || self.level < LogLevel::Debug {
            return;
        }
        let full_cmd = format!("{} {}", cmd, args.join(" "));
        println!("{} {}", "⚡".yellow(), full_cmd.dimmed());
    }
    
    /// Print module processing status with enhanced styling
    pub fn module_status(&self, module: &str, status: &str, workspace: Option<&str>) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        let module_display = format_module_path(module);
        let workspace_display = workspace.map(|w| format!(" ({})", w)).unwrap_or_default();
        
        match status {
            "initializing" => println!("  {} {}{} {}", "🔧".yellow(), module_display.cyan(), workspace_display.dimmed(), "initializing...".yellow()),
            "planning" => println!("  {} {}{} {}", "📋".blue(), module_display.cyan(), workspace_display.dimmed(), "planning...".blue()),
            "applying" => println!("  {} {}{} {}", "🚀".green(), module_display.cyan(), workspace_display.dimmed(), "applying...".green()),
            "success" => println!("  {} {}{} {}", "✅".green(), module_display.cyan(), workspace_display.dimmed(), "completed".green()),
            "failed" => println!("  {} {}{} {}", "❌".red(), module_display.cyan(), workspace_display.dimmed(), "failed".red()),
            _ => println!("  {} {}{} {}", "•".blue(), module_display.cyan(), workspace_display.dimmed(), status),
        }
    }

    /// Print module header with enhanced styling
    pub fn module_header(&self, module: &str) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        let module_display = format_module_path(module);
        println!("\n📦 {}", module_display.cyan().bold());
    }

    /// Print workspace discovery with better formatting
    pub fn workspace_discovery(&self, workspaces: &[String]) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        if workspaces.len() <= 1 {
            // Don't print anything for single workspace
        } else {
            let active_workspaces: Vec<&String> = workspaces.iter().filter(|w| w.as_str() != "default").collect();
            if !active_workspaces.is_empty() {
                println!("  {} Processing {} workspaces: {}", 
                    "🌐".blue(), 
                    active_workspaces.len().to_string().cyan(),
                    active_workspaces.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ").cyan()
                );
            }
        }
    }

    /// Print workspace processing status with better formatting
    pub fn workspace_processing(&self, workspace: &str, _var_files_count: usize) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        println!("  {} Processing workspace: {}", "🔄".blue(), workspace.cyan());
    }

    /// Print workspace skip status
    pub fn workspace_skip(&self, workspace: &str, reason: &str) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        println!("  {} Skipping workspace: {} ({})", "⏭️".yellow(), workspace.cyan(), reason.dimmed());
    }

    /// Print parallel processing start with better formatting
    pub fn parallel_processing_start(&self, worker_count: usize) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        println!("\n🚀 Starting parallel processing with {} worker{}...", 
            worker_count.to_string().cyan().bold(),
            if worker_count == 1 { "" } else { "s" }
        );
    }

    /// Print operation status with better formatting
    pub fn operation_status(&self, operation: &str, workspace: Option<&str>, _var_files_count: usize) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        let workspace_display = workspace.map(|w| format!(" in workspace '{}'", w)).unwrap_or_default();
        println!("  {} Running {} operation{}", "⚡".blue(), operation.cyan(), workspace_display);
    }

    /// Print operation completion with better formatting
    pub fn operation_completion(&self, module: &str, workspace: Option<&str>, success: bool) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        let module_display = format_module_path(module);
        let workspace_display = workspace.map(|w| format!(":{}", w)).unwrap_or_default();
        
        if success {
            println!("✅ {} completed successfully", (module_display + &workspace_display).cyan());
        } else {
            println!("❌ {} failed", (module_display + &workspace_display).red());
        }
    }

    /// Print processing summary with better organization
    pub fn processing_summary(&self, total_modules: usize, successful_modules: usize, failed_modules: usize) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        println!("\n📊 Processing Summary:");
        println!("  {} Total modules: {}", "📦".blue(), total_modules.to_string().cyan());
        println!("  {} Successful: {}", "✅".green(), successful_modules.to_string().green());
        if failed_modules > 0 {
            println!("  {} Failed: {}", "❌".red(), failed_modules.to_string().red());
        }
    }

    /// Print module initialization status (simplified)
    pub fn module_init_status(&self, success: bool) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        if success {
            println!("  {} Module ready", "✅".green());
        } else {
            println!("  {} Module initialization failed", "❌".red());
        }
    }
    
    /// Print change detection results with enhanced styling
    pub fn changes_detected(&self, count: usize, modules: &[String]) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        if count == 0 {
            println!("{}", "🎉 No changes detected".green().bold());
            return;
        }
        
        println!("{} {} changed module{} detected:", 
            "📦".blue().bold(), 
            count.to_string().cyan().bold(),
            if count == 1 { "" } else { "s" }
        );
        
        for module in modules {
            let module_name = module.split('/').last().unwrap_or(module);
            println!("  {} {}", "•".blue(), module_name.cyan());
        }
    }
    
    /// Print pipeline detection info with enhanced styling
    pub fn pipeline_info(&self, pr_number: &str, base: &str, head: &str) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        println!("{} Pipeline environment detected:", "🚀".blue().bold());
        self.table(&[
            ("PR Number", pr_number),
            ("Base Commit", &base[..7.min(base.len())]),
            ("Head Commit", &head[..7.min(head.len())]),
        ]);
    }

    /// Print a step indicator for multi-step processes
    pub fn step(&self, step: usize, total: usize, description: &str) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        println!("{} [{}/{}] {}", "📝".blue(), step, total, description.cyan());
    }

    /// Print a configuration summary
    pub fn config_summary(&self, settings: &[(&str, &str)]) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        self.section("Configuration");
        self.table(settings);
    }

    /// Print a results summary with statistics
    pub fn results_summary(&self, title: &str, stats: &[(&str, &str)]) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        self.section(title);
        self.summary("Results", stats);
    }

    /// Print a warning box for important notices
    pub fn warning_box(&self, title: &str, message: &str) {
        if self.quiet || self.level < LogLevel::Warn {
            return;
        }
        
        const MAX_BOX_WIDTH: usize = 100;
        
        // Prepare wrapped lines
        let mut lines: Vec<String> = Vec::new();
        for raw_line in message.split('\n') {
            let mut line = raw_line.trim_end();
            while line.len() > MAX_BOX_WIDTH {
                let split_at = line.char_indices()
                    .take_while(|(idx, _)| *idx <= MAX_BOX_WIDTH)
                    .map(|(idx, _)| idx)
                    .last()
                    .unwrap_or(MAX_BOX_WIDTH);
                lines.push(line[..split_at].to_string());
                line = &line[split_at..];
            }
            if !line.is_empty() {
                lines.push(line.to_string());
            }
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        
        let content_max = lines.iter().map(|l| l.len()).max().unwrap_or(0);
        let max_width = title.len().max(content_max).max(20).min(MAX_BOX_WIDTH);
        let border = "─".repeat(max_width + 2);
        
        println!("\n┌{}┐", border.yellow());
        println!("│ {:<width$} │", title.yellow().bold(), width = max_width);
        println!("├{}┤", border.yellow());
        for l in &lines {
            println!("│ {:<width$} │", l, width = max_width);
        }
        println!("└{}┘", border.yellow());
    }

    /// Print an error box for detailed error information
    pub fn error_box(&self, title: &str, message: &str) {
        if self.quiet || self.level < LogLevel::Error {
            return;
        }
        
        const MAX_BOX_WIDTH: usize = 100;
        
        // Prepare wrapped lines
        let mut lines: Vec<String> = Vec::new();
        for raw_line in message.split('\n') {
            let mut line = raw_line.trim_end();
            while line.len() > MAX_BOX_WIDTH {
                let split_at = line.char_indices()
                    .take_while(|(idx, _)| *idx <= MAX_BOX_WIDTH)
                    .map(|(idx, _)| idx)
                    .last()
                    .unwrap_or(MAX_BOX_WIDTH);
                lines.push(line[..split_at].to_string());
                line = &line[split_at..];
            }
            if !line.is_empty() {
                lines.push(line.to_string());
            }
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        
        let content_max = lines.iter().map(|l| l.len()).max().unwrap_or(0);
        let max_width = title.len().max(content_max).max(20).min(MAX_BOX_WIDTH);
        let border = "─".repeat(max_width + 2);
        
        eprintln!("\n┌{}┐", border.red());
        eprintln!("│ {:<width$} │", title.red().bold(), width = max_width);
        eprintln!("├{}┤", border.red());
        for l in &lines {
            eprintln!("│ {:<width$} │", l, width = max_width);
        }
        eprintln!("└{}┘", border.red());
    }

    /// Print a success box for completion messages
    pub fn success_box(&self, title: &str, message: &str) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        const MAX_BOX_WIDTH: usize = 100;
        
        // Prepare wrapped lines
        let mut lines: Vec<String> = Vec::new();
        for raw_line in message.split('\n') {
            let mut line = raw_line.trim_end();
            while line.len() > MAX_BOX_WIDTH {
                let split_at = line.char_indices()
                    .take_while(|(idx, _)| *idx <= MAX_BOX_WIDTH)
                    .map(|(idx, _)| idx)
                    .last()
                    .unwrap_or(MAX_BOX_WIDTH);
                lines.push(line[..split_at].to_string());
                line = &line[split_at..];
            }
            if !line.is_empty() {
                lines.push(line.to_string());
            }
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        
        let content_max = lines.iter().map(|l| l.len()).max().unwrap_or(0);
        let max_width = title.len().max(content_max).max(20).min(MAX_BOX_WIDTH);
        let border = "─".repeat(max_width + 2);
        
        println!("\n┌{}┐", border.green());
        println!("│ {:<width$} │", title.green().bold(), width = max_width);
        println!("├{}┤", border.green());
        for l in &lines {
            println!("│ {:<width$} │", l, width = max_width);
        }
        println!("└{}┘", border.green());
    }

    /// Print git change detection progress in a cleaner way
    pub fn git_changes_progress(&self, commit_range: &str, changed_count: usize, total_files: &[String]) {
        if self.quiet || self.level < LogLevel::Debug {
            return;
        }
        
        if changed_count == 0 {
            println!("  {} No changes in {}", "○".dimmed(), commit_range.dimmed());
        } else {
            println!("  {} Found {} changes in {}", "●".blue(), changed_count.to_string().cyan(), commit_range.dimmed());
            
            // Only show file details in trace level
            if self.level >= LogLevel::Trace {
                for file in total_files {
                    let file_name = file.split('/').last().unwrap_or(file);
                    println!("    {} {}", "•".dimmed(), file_name.dimmed());
                }
            }
        }
    }

    /// Print changed files in a beautiful, organized way
    pub fn changed_files_summary(&self, files: &[String]) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        if files.is_empty() {
            return;
        }
        
        // Group files by directory for better organization
        let mut file_groups: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        
        for file_path in files {
            let path = std::path::Path::new(file_path);
            if let Some(parent) = path.parent() {
                let parent_str = parent.to_string_lossy().to_string();
                let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                
                file_groups.entry(parent_str).or_insert_with(Vec::new).push(file_name);
            }
        }
        
        // Sort directories for consistent output
        let mut sorted_dirs: Vec<_> = file_groups.keys().collect();
        sorted_dirs.sort();
        
        println!("  {} Changed files:", "📝".blue());
        
        for dir in sorted_dirs {
            let files_in_dir = &file_groups[dir];
            
            // Get a shorter, more readable directory name
            let short_dir = if dir.contains("/terraform/") {
                if let Some(terraform_part) = dir.split("/terraform/").nth(1) {
                    format!("terraform/{}", terraform_part)
                } else {
                    dir.clone()
                }
            } else {
                dir.clone()
            };
            
            println!("    {} {}", "📁".cyan(), short_dir.cyan().bold());
            
            // Sort files for consistent output
            let mut sorted_files = files_in_dir.clone();
            sorted_files.sort();
            
            for file in sorted_files {
                let file_icon = if file.ends_with(".tf") {
                    "🔧"
                } else if file.ends_with(".tfvars") {
                    "⚙️"
                } else {
                    "📄"
                };
                
                println!("      {} {} {}", file_icon.dimmed(), "•".dimmed(), file.dimmed());
            }
        }
    }

    /// Print a summary of git analysis
    pub fn git_analysis_summary(&self, total_commits: usize, total_changes: usize, modules_found: usize) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        println!("  {} Analyzed {} commits, found {} changes affecting {} modules", 
            "📊".blue(), 
            total_commits.to_string().cyan(),
            total_changes.to_string().cyan(),
            modules_found.to_string().cyan()
        );
    }

    /// Print module discovery progress
    pub fn module_discovery(&self, count: usize, path: &str) {
        if self.quiet || self.level < LogLevel::Debug {
            return;
        }
        
        println!("  {} Found {} modules in {}", "🔍".blue(), count.to_string().cyan(), path.dimmed());
    }

    /// Print dependency graph building progress
    pub fn dependency_graph_progress(&self, stage: &str) {
        if self.quiet || self.level < LogLevel::Debug {
            return;
        }
        
        println!("  {} {}", "🔗".blue(), stage.cyan());
    }

    /// Print environment detection
    pub fn environment_detection(&self, env_type: &str, details: &str) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        match env_type {
            "pipeline" => println!("  {} Pipeline environment: {}", "🚀".blue(), details.cyan()),
            "local" => println!("  {} Local environment: {}", "💻".blue(), details.cyan()),
            "branch" => println!("  {} Branch detection: {}", "🌿".blue(), details.cyan()),
            _ => println!("  {} {}: {}", "ℹ".blue(), env_type.cyan(), details),
        }
    }

    /// Print configuration validation warnings in a cleaner way
    pub fn config_validation_warnings(&self, warnings: &[String]) {
        if self.quiet || self.level < LogLevel::Warn {
            return;
        }
        
        if warnings.is_empty() {
            return;
        }
        
        // Group warnings by type for better organization
        let mut var_file_warnings = Vec::new();
        let mut other_warnings = Vec::new();
        
        for warning in warnings {
            if warning.contains("Var file") && warning.contains("does not exist") {
                var_file_warnings.push(warning);
            } else {
                other_warnings.push(warning);
            }
        }
        
        // Print var file warnings in a structured way
        if !var_file_warnings.is_empty() {
            println!("  {} Missing variable files:", "📄".yellow());
            for warning in &var_file_warnings {
                if let Some(file_name) = warning.split("'").nth(1) {
                    if let Some(workspace) = warning.split("global workspace '").nth(1).and_then(|s| s.split("'").next()) {
                        println!("    {} '{}' for workspace '{}'", "•".yellow(), file_name.cyan(), workspace.cyan());
                    } else if let Some(workspace) = warning.split("workspace '").nth(1).and_then(|s| s.split("'").next()) {
                        println!("    {} '{}' for workspace '{}'", "•".yellow(), file_name.cyan(), workspace.cyan());
                    } else {
                        println!("    {} '{}'", "•".yellow(), file_name.cyan());
                    }
                }
            }
        }
        
        // Print other warnings
        if !other_warnings.is_empty() {
            println!("  {} Other validation issues:", "⚠️".yellow());
            for warning in &other_warnings {
                println!("    {} {}", "•".yellow(), warning.trim());
            }
        }
    }

    /// Print configuration loading status
    pub fn config_loading(&self, config_path: &str) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        // Extract just the filename for cleaner display
        let path = std::path::Path::new(config_path);
        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("config");
        let parent_dir = path.parent().and_then(|p| p.to_str()).unwrap_or("");
        
        // Show a cleaner, more elegant configuration loading message
        println!("  {} Loading configuration", "📄".blue());
        println!("    {} File: {}", "📁".dimmed(), filename.cyan().bold());
        if !parent_dir.is_empty() {
            println!("    {} Path: {}", "📍".dimmed(), parent_dir.dimmed());
        }
    }

    /// Print configuration validation summary
    pub fn config_validation_summary(&self, warning_count: usize, error_count: usize) {
        if self.quiet || self.level < LogLevel::Info {
            return;
        }
        
        if warning_count == 0 && error_count == 0 {
            println!("  {} Configuration validation: {}", "✅".green(), "All checks passed".green());
        } else {
            let mut summary_parts = Vec::new();
            if warning_count > 0 {
                summary_parts.push(format!("{} warnings", warning_count.to_string().yellow()));
            }
            if error_count > 0 {
                summary_parts.push(format!("{} errors", error_count.to_string().red()));
            }
            
            println!("  {} Configuration validation: {}", "⚠️".yellow(), summary_parts.join(", "));
        }
    }

    /// Print a concise error summary for failed operations
    pub fn error_summary(&self, title: &str, failed_count: usize, total_count: usize) {
        if self.quiet || self.level < LogLevel::Error {
            return;
        }
        
        let success_count = total_count - failed_count;
        println!("\n📊 {} Summary:", title);
        println!("  ✅ Successful: {}", success_count);
        println!("  ❌ Failed: {}", failed_count);
        println!("  📦 Total: {}", total_count);
    }
}

/// Global logger instance using modern LazyLock
static LOGGER: LazyLock<Mutex<Logger>> = LazyLock::new(|| {
    Mutex::new(Logger::new())
});

/// Initialize the global logger
pub fn init(level: LogLevel, quiet: bool) {
    let mut logger = LOGGER.lock().unwrap();
    let mut new_logger = Logger::new().with_level(level);
    if quiet {
        new_logger = new_logger.quiet();
    }
    *logger = new_logger;
}

/// Get a reference to the global logger
pub fn get() -> std::sync::MutexGuard<'static, Logger> {
    LOGGER.lock().unwrap()
}

/// Helper functions for common logging patterns
pub fn section(title: &str) {
    let logger = get();
    logger.section(title);
}

pub fn success(message: &str) {
    let logger = get();
    logger.success(message);
}

pub fn error(message: &str) {
    let logger = get();
    logger.error(message);
}

pub fn warn(message: &str) {
    let logger = get();
    logger.warn(message);
}

pub fn info(message: &str) {
    let logger = get();
    logger.info(message);
}

pub fn debug(message: &str) {
    let logger = get();
    logger.debug(message);
}

pub fn list(items: &[&str], title: Option<&str>) {
    let logger = get();
    logger.list(items, title);
}

pub fn table(rows: &[(&str, &str)]) {
    let logger = get();
    logger.table(rows);
}

pub fn summary(title: &str, items: &[(&str, &str)]) {
    let logger = get();
    logger.summary(title, items);
}

pub fn progress(message: &str) -> Option<Progress> {
    let logger = get();
    logger.progress(message)
}

pub fn command(cmd: &str, args: &[&str]) {
    let logger = get();
    logger.command(cmd, args);
}

pub fn module_status(module: &str, status: &str, workspace: Option<&str>) {
    let logger = get();
    logger.module_status(module, status, workspace);
}

pub fn changes_detected(count: usize, modules: &[String]) {
    let logger = get();
    logger.changes_detected(count, modules);
}

pub fn pipeline_info(pr_number: &str, base: &str, head: &str) {
    let logger = get();
    logger.pipeline_info(pr_number, base, head);
}

pub fn step(step: usize, total: usize, description: &str) {
    let logger = get();
    logger.step(step, total, description);
}

pub fn config_summary(settings: &[(&str, &str)]) {
    let logger = get();
    logger.config_summary(settings);
}

pub fn results_summary(title: &str, stats: &[(&str, &str)]) {
    let logger = get();
    logger.results_summary(title, stats);
}

pub fn warning_box(title: &str, message: &str) {
    let logger = get();
    logger.warning_box(title, message);
}

pub fn error_box(title: &str, message: &str) {
    let logger = get();
    logger.error_box(title, message);
}

pub fn success_box(title: &str, message: &str) {
    let logger = get();
    logger.success_box(title, message);
}

pub fn git_changes_progress(commit_range: &str, changed_count: usize, total_files: &[String]) {
    let logger = get();
    logger.git_changes_progress(commit_range, changed_count, total_files);
}

pub fn changed_files_summary(files: &[String]) {
    let logger = get();
    logger.changed_files_summary(files);
}

pub fn git_analysis_summary(total_commits: usize, total_changes: usize, modules_found: usize) {
    let logger = get();
    logger.git_analysis_summary(total_commits, total_changes, modules_found);
}

pub fn module_discovery(count: usize, path: &str) {
    let logger = get();
    logger.module_discovery(count, path);
}

pub fn dependency_graph_progress(stage: &str) {
    let logger = get();
    logger.dependency_graph_progress(stage);
}

pub fn environment_detection(env_type: &str, details: &str) {
    let logger = get();
    logger.environment_detection(env_type, details);
}

pub fn config_validation_warnings(warnings: &[String]) {
    let logger = get();
    logger.config_validation_warnings(warnings);
}

pub fn config_loading(config_path: &str) {
    let logger = get();
    logger.config_loading(config_path);
}

pub fn config_validation_summary(warning_count: usize, error_count: usize) {
    let logger = get();
    logger.config_validation_summary(warning_count, error_count);
}

pub fn error_summary(title: &str, failed_count: usize, total_count: usize) {
    let logger = get();
    logger.error_summary(title, failed_count, total_count);
}

/// Format duration for display
fn format_duration(duration: Duration) -> String {
    if duration.as_secs() < 1 {
        format!("{}ms", duration.as_millis())
    } else if duration.as_secs() < 60 {
        format!("{:.1}s", duration.as_secs_f64())
    } else {
        let minutes = duration.as_secs() / 60;
        let seconds = duration.as_secs() % 60;
        format!("{}m {}s", minutes, seconds)
    }
}

/// Format module path for display (reuse from display_utils)
fn format_module_path(module_path: &str) -> String {
    use crate::utils::display_utils::format_module_path;
    format_module_path(module_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(Duration::from_secs(1)), "1.0s");
        assert_eq!(format_duration(Duration::from_secs(65)), "1m 5s");
    }
    
    #[test]
    fn test_logger_levels() {
        let logger = Logger::new().with_level(LogLevel::Warn);
        // These should not print anything in tests
        logger.info("This should not appear");
        logger.debug("This should not appear");
        
        // These should work
        logger.warn("This should appear");
        logger.error("This should appear");
    }
}

// New helper functions for improved output
pub fn module_header(module: &str) {
    get().module_header(module);
}

pub fn workspace_discovery(workspaces: &[String]) {
    get().workspace_discovery(workspaces);
}

pub fn workspace_processing(workspace: &str, var_files_count: usize) {
    get().workspace_processing(workspace, var_files_count);
}

pub fn workspace_skip(workspace: &str, reason: &str) {
    get().workspace_skip(workspace, reason);
}

pub fn parallel_processing_start(worker_count: usize) {
    get().parallel_processing_start(worker_count);
}

pub fn operation_status(operation: &str, workspace: Option<&str>, var_files_count: usize) {
    get().operation_status(operation, workspace, var_files_count);
}

pub fn operation_completion(module: &str, workspace: Option<&str>, success: bool) {
    get().operation_completion(module, workspace, success);
}

pub fn processing_summary(total_modules: usize, successful_modules: usize, failed_modules: usize) {
    get().processing_summary(total_modules, successful_modules, failed_modules);
}

pub fn module_init_status(success: bool) {
    get().module_init_status(success);
}
