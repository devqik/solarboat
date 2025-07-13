use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::collections::VecDeque;

use crate::utils::terraform_background::BackgroundTerraform;
use crate::utils::terraform_operations::{
    TerraformOperation, OperationResult, OperationType,
    select_workspace, save_plan_output, run_single_plan, run_single_apply
};

pub struct ParallelProcessor {
    operations: Arc<Mutex<VecDeque<TerraformOperation>>>,
    results: Arc<Mutex<Vec<OperationResult>>>,
    active_count: Arc<Mutex<usize>>,
    parallel_limit: usize,
    worker_handle: Option<thread::JoinHandle<()>>,
}

impl ParallelProcessor {
    /// Create a new ParallelProcessor with the specified concurrency limit (clamped to 1-4).
    pub fn new(parallel_limit: usize) -> Self {
        Self {
            operations: Arc::new(Mutex::new(VecDeque::new())),
            results: Arc::new(Mutex::new(Vec::new())),
            active_count: Arc::new(Mutex::new(0)),
            parallel_limit: parallel_limit.max(1).min(4), // Clamp between 1 and 4
            worker_handle: None,
        }
    }

    /// Add an operation to the processing queue.
    pub fn add_operation(&self, operation: TerraformOperation) {
        let mut ops = self.operations.lock().unwrap();
        ops.push_back(operation);
    }

    /// Start the worker thread that manages the parallel processing.
    pub fn start(&mut self) {
        let operations = Arc::clone(&self.operations);
        let results = Arc::clone(&self.results);
        let active_count = Arc::clone(&self.active_count);
        let parallel_limit = self.parallel_limit;

        let handle = thread::spawn(move || {
            loop {
                // Check if there are operations to process
                let has_operations = {
                    let ops = operations.lock().unwrap();
                    !ops.is_empty()
                };

                if !has_operations {
                    // Check if all operations are complete
                    let active = active_count.lock().unwrap();
                    if *active == 0 {
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                // Try to start a new operation
                let can_start = {
                    let mut active = active_count.lock().unwrap();
                    if *active < parallel_limit {
                        *active += 1;
                        true
                    } else {
                        false
                    }
                };

                if can_start {
                    let operation = {
                        let mut ops = operations.lock().unwrap();
                        ops.pop_front()
                    };

                    if let Some(op) = operation {
                        let results = Arc::clone(&results);
                        let active_count = Arc::clone(&active_count);
                        
                        thread::spawn(move || {
                            let result = Self::process_operation(&op);
                            
                            {
                                let mut results = results.lock().unwrap();
                                results.push(result);
                            }
                            
                            {
                                let mut active = active_count.lock().unwrap();
                                *active = active.saturating_sub(1);
                            }
                        });
                    } else {
                        // No operation available, decrement active count
                        let mut active = active_count.lock().unwrap();
                        *active = active.saturating_sub(1);
                    }
                } else {
                    thread::sleep(Duration::from_millis(100));
                }
            }
        });

        self.worker_handle = Some(handle);
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

        println!("📦 Processing module: {} (workspace: {:?})", 
                module_path, workspace.as_deref().unwrap_or("default"));

        // Initialize module if needed
        let init_success = if watch {
            println!("  🔧 Initializing module in background...");
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
            if let Err(e) = select_workspace(module_path, workspace_name) {
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
                                            if let Err(e) = save_plan_output(module_path, plan_dir, &background_tf.get_output()) {
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
                    match run_single_plan(module_path, plan_dir.as_deref(), Some(var_files)) {
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
