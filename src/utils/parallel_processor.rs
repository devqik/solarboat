use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::collections::{VecDeque, HashMap};

use crate::utils::terraform_operations::{TerraformOperation, OperationType, OperationResult};
use crate::utils::terraform_background::BackgroundTerraform;
use crate::utils::terraform_operations::{save_plan_output, run_single_plan, run_single_apply};
use crate::utils::display_utils::{format_module_path};

/// Groups operations by module to prevent Terraform state lock conflicts
#[derive(Debug)]
struct ModuleGroup {
    operations: VecDeque<TerraformOperation>,
}

impl ModuleGroup {
    fn new(_module_path: String) -> Self {
        Self {
            operations: VecDeque::new(),
        }
    }

    fn add_operation(&mut self, operation: TerraformOperation) {
        self.operations.push_back(operation);
    }

    fn take_next_operation(&mut self) -> Option<TerraformOperation> {
        self.operations.pop_front()
    }

    fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

pub struct ParallelProcessor {
    module_groups: Arc<Mutex<HashMap<String, ModuleGroup>>>,
    results: Arc<Mutex<Vec<OperationResult>>>,
    active_modules: Arc<Mutex<HashMap<String, bool>>>,
    parallel_limit: usize,
    worker_handle: Option<thread::JoinHandle<()>>,
}

impl ParallelProcessor {
    /// Create a new ParallelProcessor with the specified concurrency limit (clamped to 1-4).
    /// This processor groups operations by module to prevent Terraform state lock conflicts.
    pub fn new(parallel_limit: usize) -> Self {
        Self {
            module_groups: Arc::new(Mutex::new(HashMap::new())),
            results: Arc::new(Mutex::new(Vec::new())),
            active_modules: Arc::new(Mutex::new(HashMap::new())),
            parallel_limit: parallel_limit.max(1).min(4), // Clamp between 1 and 4
            worker_handle: None,
        }
    }

    /// Add an operation to the processing queue.
    /// Operations are automatically grouped by module to prevent state lock conflicts.
    pub fn add_operation(&self, operation: TerraformOperation) {
        let mut groups = self.module_groups.lock().unwrap();
        let module_path = operation.module_path.clone();
        
        groups.entry(module_path.clone())
            .or_insert_with(|| ModuleGroup::new(module_path))
            .add_operation(operation);
    }

    /// Start the worker thread that manages the parallel processing.
    /// This ensures that all workspaces of the same module are processed sequentially
    /// while different modules can run in parallel.
    pub fn start(&mut self) {
        let module_groups = Arc::clone(&self.module_groups);
        let results = Arc::clone(&self.results);
        let active_modules = Arc::clone(&self.active_modules);
        let parallel_limit = self.parallel_limit;

        let handle = thread::spawn(move || {
            loop {
                // Check if there are any operations to process
                let has_operations = {
                    let groups = module_groups.lock().unwrap();
                    groups.values().any(|group| !group.is_empty())
                };

                if !has_operations {
                    // Check if all operations are complete
                    let active = active_modules.lock().unwrap();
                    if active.is_empty() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                // Try to start a new module processing
                let can_start = {
                    let active = active_modules.lock().unwrap();
                    active.len() < parallel_limit
                };

                if can_start {
                    // Find a module that's not currently being processed
                    let module_to_process = {
                        let groups = module_groups.lock().unwrap();
                        let active = active_modules.lock().unwrap();
                        
                        groups.iter()
                            .filter(|(module_path, group)| {
                                !group.is_empty() && !active.contains_key(*module_path)
                            })
                            .next()
                            .map(|(module_path, _)| module_path.clone())
                    };

                    if let Some(module_path) = module_to_process {
                        // Mark this module as active
                        {
                            let mut active = active_modules.lock().unwrap();
                            active.insert(module_path.clone(), true);
                        }

                        let module_groups = Arc::clone(&module_groups);
                        let results = Arc::clone(&results);
                        let active_modules = Arc::clone(&active_modules);
                        
                        thread::spawn(move || {
                            // Process all operations for this module sequentially
                            Self::process_module_operations(
                                module_path.clone(),
                                module_groups,
                                results,
                                active_modules
                            );
                        });
                    } else {
                        thread::sleep(Duration::from_millis(100));
                    }
                } else {
                    thread::sleep(Duration::from_millis(100));
                }
            }
        });

        self.worker_handle = Some(handle);
    }

    /// Process all operations for a specific module sequentially.
    /// This prevents Terraform state lock conflicts by ensuring all workspaces
    /// of the same module are processed one after another.
    fn process_module_operations(
        module_path: String,
        module_groups: Arc<Mutex<HashMap<String, ModuleGroup>>>,
        results: Arc<Mutex<Vec<OperationResult>>>,
        active_modules: Arc<Mutex<HashMap<String, bool>>>,
    ) {
        let display_path = format_module_path(&module_path);
        
        loop {
            // Get the next operation for this module
            let operation = {
                let mut groups = module_groups.lock().unwrap();
                if let Some(group) = groups.get_mut(&module_path) {
                    group.take_next_operation()
                } else {
                    None
                }
            };

            if let Some(op) = operation {
                // Process the operation
                let result = Self::process_operation(&op);
                
                // Store the result
                {
                    let mut results = results.lock().unwrap();
                    results.push(result);
                }
            } else {
                // No more operations for this module
                break;
            }
        }

        // Mark this module as no longer active
        {
            let mut active = active_modules.lock().unwrap();
            active.remove(&module_path);
        }
        
        println!("✅ Completed: {}", display_path);
    }

    /// Wait for all operations to complete and return the results.
    pub fn wait_for_completion(mut self) -> Vec<OperationResult> {
        // Wait for the worker thread to finish
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }

        // Return the collected results
        let results = self.results.lock().unwrap();
        results.clone()
    }

    /// Get the parallel limit (for testing purposes).
    pub fn get_parallel_limit(&self) -> usize {
        self.parallel_limit
    }

    /// Process a single terraform operation (internal).
    fn process_operation(operation: &TerraformOperation) -> OperationResult {
        let module_path = &operation.module_path;
        let workspace = &operation.workspace;
        let var_files = &operation.var_files;
        let operation_type = &operation.operation_type;
        let watch = operation.watch;

        // Initialize module if needed
        let init_success = if watch {
            println!("  🔧 Initializing module...");
            let mut background_tf = BackgroundTerraform::new();
            match background_tf.init_background(module_path) {
                Ok(_) => {
                    match background_tf.wait_for_completion(300) {
                        Ok(success) => {
                            if success {
                                println!("  ✅ Initialization completed");
                                true
                            } else {
                                println!("  ❌ Initialization failed");
                                false
                            }
                        }
                        Err(e) => {
                            println!("  ❌ Initialization failed: {}", e);
                            false
                        }
                    }
                }
                Err(e) => {
                    println!("  ❌ Failed to start initialization: {}", e);
                    false
                }
            }
        } else {
            println!("  🔧 Initializing module...");
            match crate::utils::terraform_background::run_terraform_silent("init", &[], module_path, None) {
                Ok(success) => {
                    if success {
                        println!("  ✅ Initialization completed");
                        true
                    } else {
                        println!("  ❌ Initialization failed");
                        false
                    }
                }
                Err(e) => {
                    println!("  ❌ Initialization failed: {}", e);
                    false
                }
            }
        };

        if !init_success {
            return OperationResult {
                module_path: module_path.clone(),
                workspace: workspace.clone(),
                operation_type: operation_type.clone(),
                success: false,
                error: Some("Initialization failed".to_string()),
                output: Vec::new(),
            };
        }

        // Select workspace if specified
        if let Some(ref workspace_name) = workspace {
            println!("  🔄 Switching to workspace: {}", workspace_name);
            if let Err(e) = crate::utils::terraform_operations::select_workspace(module_path, workspace_name) {
                return OperationResult {
                    module_path: module_path.clone(),
                    workspace: workspace.clone(),
                    operation_type: operation_type.clone(),
                    success: false,
                    error: Some(format!("Failed to select workspace {}: {}", workspace_name, e)),
                    output: Vec::new(),
                };
            }
        }

        // Execute the main operation
        let (success, error, output) = match operation_type {
            OperationType::Init => {
                // Init is already done above, this shouldn't happen
                (true, None, Vec::new())
            }
            OperationType::Plan { plan_dir } => {
                println!("  🚀 Running terraform plan...");
                if !var_files.is_empty() {
                    println!("  📄 Using {} var files", var_files.len());
                }

                if watch {
                    let mut background_tf = BackgroundTerraform::new();
                    match background_tf.plan_background(module_path, Some(var_files)) {
                        Ok(_) => {
                            match background_tf.wait_for_completion(600) { // 10 minute timeout
                                Ok(success) => {
                                    if success {
                                        println!("  ✅ Plan completed successfully");
                                        // Save plan output if plan_dir is specified
                                        if let Some(plan_dir) = plan_dir {
                                            if let Err(e) = save_plan_output(module_path, plan_dir, workspace.as_deref(), &background_tf.get_output()) {
                                                println!("  ⚠️  Failed to save plan output: {}", e);
                                            }
                                        }
                                        (true, None, background_tf.get_output())
                                    } else {
                                        println!("  ❌ Plan failed");
                                        (false, Some("Plan failed".to_string()), background_tf.get_output())
                                    }
                                }
                                Err(e) => {
                                    println!("  ❌ Plan failed: {}", e);
                                    (false, Some(format!("Plan failed: {}", e)), background_tf.get_output())
                                }
                            }
                        }
                        Err(e) => {
                            println!("  ❌ Failed to start plan: {}", e);
                            (false, Some(format!("Failed to start plan: {}", e)), Vec::new())
                        }
                    }
                } else {
                    match run_single_plan(module_path, plan_dir.as_deref(), workspace.as_deref(), Some(var_files)) {
                        Ok(success) => {
                            if success {
                                println!("  ✅ Plan completed successfully");
                                (true, None, Vec::new())
                            } else {
                                println!("  ❌ Plan failed");
                                (false, Some("Plan failed".to_string()), Vec::new())
                            }
                        }
                        Err(e) => {
                            println!("  ❌ Plan failed: {}", e);
                            (false, Some(format!("Plan failed: {}", e)), Vec::new())
                        }
                    }
                }
            }
            OperationType::Apply => {
                println!("  🧱 Running terraform apply...");
                if !var_files.is_empty() {
                    println!("  📄 Using {} var files", var_files.len());
                }

                if watch {
                    let mut background_tf = BackgroundTerraform::new();
                    match background_tf.apply_background(module_path, Some(var_files)) {
                        Ok(_) => {
                            match background_tf.wait_for_completion(1800) { // 30 minute timeout
                                Ok(success) => {
                                    if success {
                                        println!("  ✅ Apply completed successfully");
                                        (true, None, background_tf.get_output())
                                    } else {
                                        println!("  ❌ Apply failed");
                                        (false, Some("Apply failed".to_string()), background_tf.get_output())
                                    }
                                }
                                Err(e) => {
                                    println!("  ❌ Apply failed: {}", e);
                                    (false, Some(format!("Apply failed: {}", e)), background_tf.get_output())
                                }
                            }
                        }
                        Err(e) => {
                            println!("  ❌ Failed to start apply: {}", e);
                            (false, Some(format!("Failed to start apply: {}", e)), Vec::new())
                        }
                    }
                } else {
                    match run_single_apply(module_path, Some(var_files)) {
                        Ok(success) => {
                            if success {
                                println!("  ✅ Apply completed successfully");
                                (true, None, Vec::new())
                            } else {
                                println!("  ❌ Apply failed");
                                (false, Some("Apply failed".to_string()), Vec::new())
                            }
                        }
                        Err(e) => {
                            println!("  ❌ Apply failed: {}", e);
                            (false, Some(format!("Apply failed: {}", e)), Vec::new())
                        }
                    }
                }
            }
        };

        OperationResult {
            module_path: module_path.clone(),
            workspace: workspace.clone(),
            operation_type: operation_type.clone(),
            success,
            error,
            output,
        }
    }
} 
