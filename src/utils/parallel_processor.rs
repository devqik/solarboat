use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::collections::{VecDeque, HashMap};

use crate::utils::terraform_operations::{TerraformOperation, OperationType, OperationResult};
use crate::utils::terraform_background::BackgroundTerraform;
use crate::utils::terraform_operations::{save_plan_output, run_single_plan, run_single_apply};
use crate::utils::display_utils::{format_module_path};
use crate::utils::logger;
use crate::utils::error::{SolarboatError, SafeOperations};

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
            parallel_limit: parallel_limit.clamp(1, 4), // Clamp between 1 and 4
            worker_handle: None,
        }
    }

    /// Add an operation to the processing queue.
    /// Operations are automatically grouped by module to prevent state lock conflicts.
    pub fn add_operation(&self, operation: TerraformOperation) -> Result<(), SolarboatError> {
        let mut groups = SafeOperations::lock_with_timeout(
            &self.module_groups,
            Duration::from_secs(5),
            "module_groups"
        )?;
        
        let module_path = operation.module_path.clone();
        
        groups.entry(module_path.clone())
            .or_insert_with(|| ModuleGroup::new(module_path.clone()))
            .add_operation(operation);
        
        Ok(())
    }

    /// Start the worker thread that manages the parallel processing.
    /// This ensures that all workspaces of the same module are processed sequentially
    /// while different modules can run in parallel.
    pub fn start(&mut self) -> Result<(), SolarboatError> {
        let module_groups = Arc::clone(&self.module_groups);
        let results = Arc::clone(&self.results);
        let active_modules = Arc::clone(&self.active_modules);
        let parallel_limit = self.parallel_limit;

        let handle = thread::spawn(move || {
            let start_time = std::time::Instant::now();
            let max_duration = Duration::from_secs(300); // 5 minute timeout
            
            loop {
                // Check for timeout
                if start_time.elapsed() > max_duration {
                    logger::warn("Worker thread timeout reached, stopping processing");
                    break;
                }
                
                // Check if there are any operations to process
                let has_operations = {
                    let groups = match SafeOperations::lock_with_timeout(
                        &module_groups,
                        Duration::from_secs(1),
                        "module_groups_check"
                    ) {
                        Ok(groups) => groups,
                        Err(e) => {
                            logger::warn(&format!("Failed to acquire module groups lock: {}", e));
                            break;
                        }
                    };
                    groups.values().any(|group| !group.is_empty())
                };

                if !has_operations {
                    // Check if all operations are complete
                    let active = match SafeOperations::lock_with_timeout(
                        &active_modules,
                        Duration::from_secs(1),
                        "active_modules_check"
                    ) {
                        Ok(active) => active,
                        Err(e) => {
                            logger::warn(&format!("Failed to acquire active modules lock: {}", e));
                            // If we can't check, assume we're done to prevent hanging
                            break;
                        }
                    };
                    
                    if active.is_empty() {
                        break;
                    }
                    
                    // Check if any active modules still have operations to process
                    let any_remaining_ops = {
                        match SafeOperations::lock_with_timeout(
                            &module_groups,
                            Duration::from_secs(1),
                            "module_groups_check_remaining"
                        ) {
                            Ok(groups) => groups.values().any(|group| !group.is_empty()),
                            Err(_) => {
                                // If we can't check, assume no remaining operations
                                false
                            }
                        }
                    };
                    
                    if !any_remaining_ops {
                        // No more operations to process, we can exit even if cleanup didn't complete
                        logger::info("All operations completed, exiting worker thread");
                        break;
                    }
                    
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                // Try to start a new module processing
                let can_start = {
                    match SafeOperations::lock_with_timeout(
                        &active_modules,
                        Duration::from_secs(1),
                        "active_modules_limit"
                    ) {
                        Ok(active) => active.len() < parallel_limit,
                        Err(_) => {
                            // If we can't acquire the lock, assume we can't start
                            false
                        }
                    }
                };

                if can_start {
                    // Find a module that's not currently being processed
                    let module_to_process = {
                        let groups = match SafeOperations::lock_with_timeout(
                            &module_groups,
                            Duration::from_secs(1),
                            "module_groups_process"
                        ) {
                            Ok(groups) => groups,
                            Err(_) => {
                                // If we can't acquire the lock, no module to process
                                continue;
                            }
                        };
                        
                        let active = match SafeOperations::lock_with_timeout(
                            &active_modules,
                            Duration::from_secs(1),
                            "active_modules_process"
                        ) {
                            Ok(active) => active,
                            Err(_) => {
                                // If we can't acquire the lock, no module to process
                                continue;
                            }
                        };
                        
                        groups.iter()
                            .find(|(module_path, group)| {
                                !group.is_empty() && !active.contains_key(*module_path)
                            })
                            .map(|(module_path, _)| module_path.clone())
                    };

                    if let Some(module_path) = module_to_process {
                        // Mark this module as active
                        let mut active = match SafeOperations::lock_with_timeout(
                            &active_modules,
                            Duration::from_secs(1),
                            "active_modules_mark"
                        ) {
                            Ok(active) => active,
                            Err(e) => {
                                logger::warn(&format!("Failed to acquire active modules lock: {}", e));
                                continue;
                            }
                        };
                        active.insert(module_path.clone(), true);

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
        Ok(())
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
        let _display_path = format_module_path(&module_path);
        
        loop {
            // Get the next operation for this module
            let operation = {
                let mut groups = match SafeOperations::lock_with_timeout(
                    &module_groups,
                    Duration::from_secs(5),
                    "module_groups_take_next"
                ) {
                    Ok(groups) => groups,
                    Err(e) => {
                        logger::warn(&format!("Failed to acquire module groups lock: {}", e));
                        break;
                    }
                };
                
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
                    let mut results = match SafeOperations::lock_with_timeout(
                        &results,
                        Duration::from_secs(5),
                        "results_push"
                    ) {
                        Ok(results) => results,
                        Err(e) => {
                            logger::warn(&format!("Failed to acquire results lock: {}", e));
                            break;
                        }
                    };
                    results.push(result);
                }
            } else {
                break;
            }
        }

        // Mark this module as no longer active (non-blocking cleanup)
        // Use try_lock to avoid blocking, and if it fails, just log and continue
        if let Ok(mut active) = active_modules.try_lock() {
            active.remove(&module_path);
        } else {
            // Cleanup failed, but that's okay - the worker thread will handle it
            logger::debug(&format!("Cleanup for module {} skipped (lock busy)", module_path));
        }
    }

    /// Wait for all operations to complete and return the results.
    pub fn wait_for_completion(mut self) -> Result<Vec<OperationResult>, SolarboatError> {
        // Wait for the worker thread to finish with a timeout
        if let Some(handle) = self.worker_handle.take() {
            // Use a timeout for joining the worker thread
            let start_time = std::time::Instant::now();
            let max_wait_time = Duration::from_secs(60); // 1 minute timeout
            
            while start_time.elapsed() < max_wait_time {
                if handle.is_finished() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            
            // Try to join, but don't block indefinitely
            if !handle.is_finished() {
                logger::warn("Worker thread did not finish within timeout, proceeding with available results");
            } else {
                let _ = handle.join();
            }
        }

        // Return the collected results
        let results = SafeOperations::lock_with_timeout(
            &self.results,
            Duration::from_secs(5),
            "results_clone"
        )?;
        Ok(results.clone())
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
        let _skip_init = operation.skip_init; // No longer used, but kept for compatibility

        // Always ensure module is initialized before operations
        let init_success = if watch {
            let mut background_tf = BackgroundTerraform::new();
            match background_tf.init_background(module_path) {
                Ok(_) => {
                    match background_tf.wait_for_completion(300) {
                        Ok(success) => success,
                        Err(_) => false,
                    }
                }
                Err(_) => false,
            }
        } else {
            // Use the terraform_operations::ensure_module_initialized function
            match crate::utils::terraform_operations::ensure_module_initialized(module_path) {
                Ok(_) => true,
                Err(_) => false,
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
                logger::operation_status("terraform plan", workspace.as_deref(), var_files.len());

                if watch {
                    let mut background_tf = BackgroundTerraform::new();
                    match background_tf.plan_background(module_path, Some(var_files)) {
                        Ok(_) => {
                            match background_tf.wait_for_completion(600) { // 10 minute timeout
                                Ok(success) => {
                                    if success {
                                        logger::operation_completion(module_path, workspace.as_deref(), true);
                                        // Save plan output if plan_dir is specified
                                        if let Some(plan_dir) = plan_dir {
                                            if let Ok(output) = background_tf.get_output() {
                                                if let Err(e) = save_plan_output(module_path, plan_dir, workspace.as_deref(), &output) {
                                                    println!("  ⚠️  Failed to save plan output: {}", e);
                                                }
                                            }
                                        }
                                        let output = background_tf.get_output().unwrap_or_else(|_| Vec::new());
                                        (true, None, output)
                                    } else {
                                        logger::operation_completion(module_path, workspace.as_deref(), false);
                                        let output = background_tf.get_output().unwrap_or_else(|_| Vec::new());
                                        (false, Some("Plan failed".to_string()), output)
                                    }
                                }
                                Err(e) => {
                                    logger::operation_completion(module_path, workspace.as_deref(), false);
                                    let output = background_tf.get_output().unwrap_or_else(|_| Vec::new());
                                    (false, Some(format!("Plan failed: {}", e)), output)
                                }
                            }
                        }
                        Err(e) => {
                            logger::operation_completion(module_path, workspace.as_deref(), false);
                            (false, Some(format!("Failed to start plan: {}", e)), Vec::new())
                        }
                    }
                } else {
                    match run_single_plan(module_path, plan_dir.as_deref(), workspace.as_deref(), Some(var_files)) {
                        Ok(success) => {
                            if success {
                                logger::operation_completion(module_path, workspace.as_deref(), true);
                                (true, None, Vec::new())
                            } else {
                                logger::operation_completion(module_path, workspace.as_deref(), false);
                                (false, Some("Plan failed".to_string()), Vec::new())
                            }
                        }
                        Err(e) => {
                            logger::operation_completion(module_path, workspace.as_deref(), false);
                            (false, Some(format!("Plan failed: {}", e)), Vec::new())
                        }
                    }
                }
            }
            OperationType::Apply => {
                logger::operation_status("terraform apply", workspace.as_deref(), var_files.len());

                if watch {
                    let mut background_tf = BackgroundTerraform::new();
                    match background_tf.apply_background(module_path, Some(var_files)) {
                        Ok(_) => {
                            match background_tf.wait_for_completion(1800) { // 30 minute timeout
                                Ok(success) => {
                                    if success {
                                        logger::operation_completion(module_path, workspace.as_deref(), true);
                                        let output = background_tf.get_output().unwrap_or_else(|_| Vec::new());
                                        (true, None, output)
                                    } else {
                                        logger::operation_completion(module_path, workspace.as_deref(), false);
                                        let output = background_tf.get_output().unwrap_or_else(|_| Vec::new());
                                        (false, Some("Apply failed".to_string()), output)
                                    }
                                }
                                Err(e) => {
                                    logger::operation_completion(module_path, workspace.as_deref(), false);
                                    let output = background_tf.get_output().unwrap_or_else(|_| Vec::new());
                                    (false, Some(format!("Apply failed: {}", e)), output)
                                }
                            }
                        }
                        Err(e) => {
                            logger::operation_completion(module_path, workspace.as_deref(), false);
                            (false, Some(format!("Failed to start apply: {}", e)), Vec::new())
                        }
                    }
                } else {
                    match run_single_apply(module_path, Some(var_files)) {
                        Ok(success) => {
                            if success {
                                logger::operation_completion(module_path, workspace.as_deref(), true);
                                (true, None, Vec::new())
                            } else {
                                logger::operation_completion(module_path, workspace.as_deref(), false);
                                (false, Some("Apply failed".to_string()), Vec::new())
                            }
                        }
                        Err(e) => {
                            logger::operation_completion(module_path, workspace.as_deref(), false);
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


