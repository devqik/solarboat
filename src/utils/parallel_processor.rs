use std::sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}};
use std::thread;
use std::time::Duration;
use std::collections::{HashMap, VecDeque};

use crate::utils::terraform_operations::{TerraformOperation, OperationResult};
use crate::utils::error::{SolarboatError, SafeOperations};
use crate::utils::logger;

pub struct ParallelProcessor {
    module_groups: Arc<Mutex<HashMap<String, VecDeque<TerraformOperation>>>>,
    results: Arc<Mutex<Vec<OperationResult>>>,
    total_modules: usize,
    completed_modules: Arc<AtomicUsize>,
    worker_handle: Option<thread::JoinHandle<()>>,
    parallel_limit: usize,
}

impl ParallelProcessor {
    pub fn new(parallel_limit: usize) -> Self {
        Self {
            module_groups: Arc::new(Mutex::new(HashMap::new())),
            results: Arc::new(Mutex::new(Vec::new())),
            total_modules: 0,
            completed_modules: Arc::new(AtomicUsize::new(0)),
            worker_handle: None,
            parallel_limit: parallel_limit.clamp(1, 4),
        }
    }

    pub fn add_operation(&mut self, operation: TerraformOperation) -> Result<(), SolarboatError> {
        let module_path = operation.module_path.clone();
        let workspace = operation.workspace.as_deref().unwrap_or("default");
        
        logger::debug(&format!("Adding operation: module={}, workspace={}", module_path, workspace));
        
        let mut groups = SafeOperations::lock_with_timeout(
            &self.module_groups,
            Duration::from_secs(5),
            "module_groups_add"
        )?;
        
        groups.entry(module_path.clone())
            .or_insert_with(VecDeque::new)
            .push_back(operation);
        
        logger::debug(&format!("Operation added. Total groups: {}, operations in group: {}", 
            groups.len(), 
            groups.get(&module_path).map(|g| g.len()).unwrap_or(0)
        ));
        
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), SolarboatError> {
        let groups = SafeOperations::lock_with_timeout(
            &self.module_groups,
            Duration::from_secs(5),
            "module_groups_count"
        )?;
        
        self.total_modules = groups.len();
        
        if self.total_modules == 0 {
            logger::info("No operations to process");
            return Ok(());
        }
        
        logger::info(&format!("Starting processing of {} modules with {} parallel workers", 
            self.total_modules, self.parallel_limit));
        
        let module_groups = Arc::clone(&self.module_groups);
        let results = Arc::clone(&self.results);
        let completed_modules = Arc::clone(&self.completed_modules);
        let total_modules = self.total_modules;
        let parallel_limit = self.parallel_limit;
        
        let handle = thread::spawn(move || {
            Self::process_modules(
                module_groups,
                results,
                completed_modules,
                total_modules,
                parallel_limit
            );
        });
        
        self.worker_handle = Some(handle);
        Ok(())
    }

    fn process_modules(
        module_groups: Arc<Mutex<HashMap<String, VecDeque<TerraformOperation>>>>,
        results: Arc<Mutex<Vec<OperationResult>>>,
        completed_modules: Arc<AtomicUsize>,
        total_modules: usize,
        parallel_limit: usize,
    ) {
        let active_modules = Arc::new(Mutex::new(HashMap::<String, bool>::new()));
        let start_time = std::time::Instant::now();
        let max_duration = Duration::from_secs(300);
        
        logger::debug(&format!("Worker thread started: processing {} modules with {} parallel limit", 
            total_modules, parallel_limit));
        
        loop {
            if start_time.elapsed() > max_duration {
                logger::warn("Worker thread timeout reached, stopping processing");
                break;
            }
            
            let completed = completed_modules.load(Ordering::Relaxed);
            if completed >= total_modules {
                logger::info(&format!("All {} modules completed successfully", total_modules));
                break;
            }
            
            let can_start_more = {
                let active = match active_modules.lock() {
                    Ok(active) => active,
                    Err(_) => break,
                };
                active.len() < parallel_limit
            };
            
            if can_start_more {
                let module_to_process = {
                    let groups = match SafeOperations::lock_with_timeout(
                        &module_groups,
                        Duration::from_secs(1),
                        "module_groups_process"
                    ) {
                        Ok(groups) => groups,
                        Err(e) => {
                            logger::warn(&format!("Failed to acquire module groups lock: {}", e));
                            break;
                        }
                    };
                    
                    let active = match active_modules.lock() {
                        Ok(active) => active,
                        Err(_) => break,
                    };
                    
                    groups.iter()
                        .find(|(module_path, operations)| {
                            !operations.is_empty() && !active.contains_key(*module_path)
                        })
                        .map(|(module_path, _)| module_path.clone())
                };
                
                if let Some(module_path) = module_to_process {
                    logger::debug(&format!("Starting module: {}", module_path));
                    
                    if let Ok(mut active) = active_modules.lock() {
                        active.insert(module_path.clone(), true);
                    }
                    
                    let module_groups = Arc::clone(&module_groups);
                    let results = Arc::clone(&results);
                    let completed_modules = Arc::clone(&completed_modules);
                    let active_modules_clone = Arc::clone(&active_modules);
                    
                    thread::spawn(move || {
                        Self::process_module_operations(
                            module_path.clone(),
                            module_groups,
                            results,
                            completed_modules,
                            active_modules_clone
                        );
                    });
                }
            }
            
            thread::sleep(Duration::from_millis(100));
        }
        
        logger::debug("Worker thread completed");
    }

    fn process_module_operations(
        module_path: String,
        module_groups: Arc<Mutex<HashMap<String, VecDeque<TerraformOperation>>>>,
        results: Arc<Mutex<Vec<OperationResult>>>,
        completed_modules: Arc<AtomicUsize>,
        active_modules: Arc<Mutex<HashMap<String, bool>>>,
    ) {
        let display_path = format_module_path(&module_path);
        logger::debug(&format!("Processing module: {}", display_path));
        
        let mut operation_count = 0;
        
        loop {
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
                
                if let Some(operations) = groups.get_mut(&module_path) {
                    let op = operations.pop_front();
                    logger::debug(&format!("Module {}: took operation, remaining in group: {}", 
                        display_path, operations.len()));
                    op
                } else {
                    logger::debug(&format!("Module {}: no group found", display_path));
                    None
                }
            };
            
            if let Some(op) = operation {
                operation_count += 1;
                logger::debug(&format!("Module {}: processing operation {} (workspace: {:?})", 
                    display_path, operation_count, op.workspace));
                
                let result = Self::process_single_operation(&op);
                
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
                
                if operation_count > 1 {
                    let workspace_name = op.workspace.as_deref().unwrap_or("default");
                    logger::debug(&format!("Module {}: waiting between workspace operations for '{}'", 
                        display_path, workspace_name));
                    
                    thread::sleep(Duration::from_secs(3));
                }
            } else {
                logger::debug(&format!("Module {}: no more operations, processed {} total", 
                    display_path, operation_count));
                break;
            }
        }
        
        completed_modules.fetch_add(1, Ordering::Relaxed);
        
        if let Ok(mut active) = active_modules.lock() {
            active.remove(&module_path);
            logger::debug(&format!("Module {} removed from active modules", module_path));
        }
        
        logger::debug(&format!("Module {} completed", display_path));
    }

    fn process_single_operation(operation: &TerraformOperation) -> OperationResult {
        let module_path = &operation.module_path;
        let workspace = &operation.workspace;
        let var_files = &operation.var_files;
        let operation_type = &operation.operation_type;
        let watch = operation.watch;
        let _skip_init = operation.skip_init;

        let init_success = if watch {
            let mut background_tf = crate::utils::terraform_background::BackgroundTerraform::new();
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

        let (success, error, output) = match operation_type {
            crate::utils::terraform_operations::OperationType::Init => {
                (true, None, Vec::new())
            }
            crate::utils::terraform_operations::OperationType::Plan { plan_dir } => {
                logger::operation_status("terraform plan", workspace.as_deref(), var_files.len());

                if watch {
                    let mut background_tf = crate::utils::terraform_background::BackgroundTerraform::new();
                    match background_tf.plan_background(module_path, Some(var_files)) {
                        Ok(_) => {
                            match background_tf.wait_for_completion(600) {
                                Ok(success) => {
                                    if success {
                                        logger::operation_completion(module_path, workspace.as_deref(), true);
                                        if let Some(plan_dir) = plan_dir {
                                            if let Ok(output) = background_tf.get_output() {
                                                if let Err(e) = crate::utils::terraform_operations::save_plan_output(
                                                    module_path, plan_dir, workspace.as_deref(), &output
                                                ) {
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
                                Err(_) => {
                                    logger::operation_completion(module_path, workspace.as_deref(), false);
                                    (false, Some("Plan timeout".to_string()), Vec::new())
                                }
                            }
                        }
                        Err(_) => {
                            logger::operation_completion(module_path, workspace.as_deref(), false);
                            (false, Some("Failed to start plan".to_string()), Vec::new())
                        }
                    }
                } else {
                    match crate::utils::terraform_operations::run_single_plan(
                        module_path, 
                        plan_dir.as_deref(), 
                        workspace.as_deref(), 
                        Some(var_files)
                    ) {
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
                            (false, Some(format!("Plan error: {}", e)), Vec::new())
                        }
                    }
                }
            }
            crate::utils::terraform_operations::OperationType::Apply => {
                logger::operation_status("terraform apply", workspace.as_deref(), var_files.len());

                if watch {
                    let mut background_tf = crate::utils::terraform_background::BackgroundTerraform::new();
                    match background_tf.apply_background(module_path, Some(var_files)) {
                        Ok(_) => {
                            match background_tf.wait_for_completion(1800) {
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
                                Err(_) => {
                                    logger::operation_completion(module_path, workspace.as_deref(), false);
                                    (false, Some("Apply timeout".to_string()), Vec::new())
                                }
                            }
                        }
                        Err(_) => {
                            logger::operation_completion(module_path, workspace.as_deref(), false);
                            (false, Some("Failed to start apply".to_string()), Vec::new())
                        }
                    }
                } else {
                    match crate::utils::terraform_operations::run_single_apply(module_path, Some(var_files)) {
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
                            (false, Some(format!("Apply error: {}", e)), Vec::new())
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

    pub fn wait_for_completion(mut self) -> Result<Vec<OperationResult>, SolarboatError> {
        if let Some(handle) = self.worker_handle.take() {
            let start_time = std::time::Instant::now();
            let max_wait_time = Duration::from_secs(300);
            
            logger::debug("Waiting for worker thread to complete...");
            
            while start_time.elapsed() < max_wait_time {
                if handle.is_finished() {
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
            
            if !handle.is_finished() {
                logger::warn("Worker thread did not finish within timeout, proceeding with available results");
            } else {
                match handle.join() {
                    Ok(_) => logger::debug("Worker thread completed successfully"),
                    Err(e) => logger::error(&format!("Worker thread panicked: {:?}", e)),
                }
            }
        }
        
        let results = SafeOperations::lock_with_timeout(
            &self.results,
            Duration::from_secs(5),
            "results_clone"
        )?;
        
        Ok(results.clone())
    }

    pub fn get_parallel_limit(&self) -> usize {
        self.parallel_limit
    }
}

fn format_module_path(module_path: &str) -> String {
    if let Some(file_name) = std::path::Path::new(module_path).file_name() {
        if let Some(name) = file_name.to_str() {
            return format!("terraform/projects/{}", name);
        }
    }
    module_path.to_string()
}
