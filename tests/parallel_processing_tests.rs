use solarboat::utils::parallel_processor::ParallelProcessor;
use solarboat::utils::terraform_operations::{TerraformOperation, OperationType};

#[test]
fn test_parallel_processor_creation_and_clamping() {
    let processor = ParallelProcessor::new(10);
    assert_eq!(processor.get_parallel_limit(), 4); // Should be clamped to max 4
    
    let processor = ParallelProcessor::new(0);
    assert_eq!(processor.get_parallel_limit(), 1); // Should be clamped to min 1
    
    let processor = ParallelProcessor::new(3);
    assert_eq!(processor.get_parallel_limit(), 3); // Should remain 3
}

#[test]
fn test_parallel_processor_empty_queue() {
    let mut processor = ParallelProcessor::new(2);
    processor.start();
    let results = processor.wait_for_completion();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_parallel_processor_operation_queuing() {
    let mut processor = ParallelProcessor::new(2);
    
    // Add operations for different modules
    processor.add_operation(TerraformOperation {
        module_path: "module1".to_string(),
        workspace: Some("dev".to_string()),
        var_files: vec![],
        operation_type: OperationType::Plan { plan_dir: None },
        watch: false,
        skip_init: false,
    });
    
    processor.add_operation(TerraformOperation {
        module_path: "module2".to_string(),
        workspace: Some("prod".to_string()),
        var_files: vec![],
        operation_type: OperationType::Plan { plan_dir: None },
        watch: false,
        skip_init: false,
    });
    
    processor.start();
    let results = processor.wait_for_completion();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_parallel_processor_varied_operations() {
    let mut processor = ParallelProcessor::new(2);
    
    processor.add_operation(TerraformOperation {
        module_path: "module1".to_string(),
        workspace: None,
        var_files: vec!["vars.tfvars".to_string()],
        operation_type: OperationType::Plan { plan_dir: Some("plans".to_string()) },
        watch: false,
        skip_init: false,
    });
    
    processor.add_operation(TerraformOperation {
        module_path: "module2".to_string(),
        workspace: Some("staging".to_string()),
        var_files: vec![],
        operation_type: OperationType::Apply,
        watch: false,
        skip_init: false,
    });
    
    processor.start();
    let results = processor.wait_for_completion();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_parallel_processor_mixed_operation_types() {
    let mut processor = ParallelProcessor::new(3);
    
    // Add different operation types
    processor.add_operation(TerraformOperation {
        module_path: "module1".to_string(),
        workspace: None,
        var_files: vec![],
        operation_type: OperationType::Plan { plan_dir: None },
        watch: false,
        skip_init: false,
    });
    
    processor.add_operation(TerraformOperation {
        module_path: "module2".to_string(),
        workspace: None,
        var_files: vec![],
        operation_type: OperationType::Apply,
        watch: false,
        skip_init: false,
    });
    
    processor.add_operation(TerraformOperation {
        module_path: "module3".to_string(),
        workspace: None,
        var_files: vec![],
        operation_type: OperationType::Init,
        watch: false,
        skip_init: false,
    });
    
    processor.start();
    let results = processor.wait_for_completion();
    assert_eq!(results.len(), 3);
}

#[test]
fn test_parallel_processor_workspace_handling() {
    let mut processor = ParallelProcessor::new(2);
    
    // Add operations with different workspace configurations
    processor.add_operation(TerraformOperation {
        module_path: "module1".to_string(),
        workspace: None, // Default workspace
        var_files: vec![],
        operation_type: OperationType::Plan { plan_dir: None },
        watch: false,
        skip_init: false,
    });
    
    processor.add_operation(TerraformOperation {
        module_path: "module2".to_string(),
        workspace: Some("dev".to_string()),
        var_files: vec![],
        operation_type: OperationType::Plan { plan_dir: None },
        watch: false,
        skip_init: false,
    });
    
    processor.add_operation(TerraformOperation {
        module_path: "module2".to_string(),
        workspace: Some("prod".to_string()),
        var_files: vec![],
        operation_type: OperationType::Plan { plan_dir: None },
        watch: false,
        skip_init: false,
    });
    
    processor.start();
    let results = processor.wait_for_completion();
    assert_eq!(results.len(), 3);
}

#[test]
fn test_parallel_processor_many_operations() {
    let mut processor = ParallelProcessor::new(4);
    
    // Add many operations to test scalability
    for i in 0..10 {
        processor.add_operation(TerraformOperation {
            module_path: format!("module{}", i),
            workspace: Some(format!("ws{}", i)),
            var_files: vec![],
            operation_type: OperationType::Plan { plan_dir: None },
            watch: false,
            skip_init: false,
        });
    }
    
    processor.start();
    let results = processor.wait_for_completion();
    assert_eq!(results.len(), 10);
}

#[test]
fn test_cli_parallel_argument_parsing() {
    use solarboat::cli::Args;
    use clap::Parser;
    
    // Test that parallel argument is parsed correctly
    let args = Args::try_parse_from(&["solarboat", "plan", "--parallel", "3"]).unwrap();
    if let solarboat::cli::Commands::Plan(plan_args) = args.command {
        assert_eq!(plan_args.parallel, 3);
    } else {
        panic!("Expected Plan command");
    }
    
    // Test default value
    let args = Args::try_parse_from(&["solarboat", "plan"]).unwrap();
    if let solarboat::cli::Commands::Plan(plan_args) = args.command {
        assert_eq!(plan_args.parallel, 1);
    } else {
        panic!("Expected Plan command");
    }
    
    // Test clamping (max 4)
    let args = Args::try_parse_from(&["solarboat", "plan", "--parallel", "10"]).unwrap();
    if let solarboat::cli::Commands::Plan(plan_args) = args.command {
        assert_eq!(plan_args.parallel, 10); // CLI doesn't clamp, but the processor will
    } else {
        panic!("Expected Plan command");
    }
}

#[test]
fn test_module_aware_sequential_processing() {
    let mut processor = ParallelProcessor::new(3);
    
    // Add multiple workspaces for the same module - these should be processed sequentially
    processor.add_operation(TerraformOperation {
        module_path: "shared_module".to_string(),
        workspace: Some("dev".to_string()),
        var_files: vec![],
        operation_type: OperationType::Plan { plan_dir: None },
        watch: false,
        skip_init: false,
    });
    
    processor.add_operation(TerraformOperation {
        module_path: "shared_module".to_string(),
        workspace: Some("staging".to_string()),
        var_files: vec![],
        operation_type: OperationType::Plan { plan_dir: None },
        watch: false,
        skip_init: false,
    });
    
    processor.add_operation(TerraformOperation {
        module_path: "shared_module".to_string(),
        workspace: Some("prod".to_string()),
        var_files: vec![],
        operation_type: OperationType::Plan { plan_dir: None },
        watch: false,
        skip_init: false,
    });
    
    // Add operations for different modules - these can run in parallel
    processor.add_operation(TerraformOperation {
        module_path: "other_module".to_string(),
        workspace: Some("dev".to_string()),
        var_files: vec![],
        operation_type: OperationType::Plan { plan_dir: None },
        watch: false,
        skip_init: false,
    });
    
    processor.add_operation(TerraformOperation {
        module_path: "another_module".to_string(),
        workspace: Some("prod".to_string()),
        var_files: vec![],
        operation_type: OperationType::Apply,
        watch: false,
        skip_init: false,
    });
    
    processor.start();
    let results = processor.wait_for_completion();
    
    // Should have 5 total results
    assert_eq!(results.len(), 5);
    
    // Verify all operations for shared_module are present
    let shared_module_results: Vec<_> = results.iter()
        .filter(|r| r.module_path == "shared_module")
        .collect();
    assert_eq!(shared_module_results.len(), 3);
    
    // Verify all workspaces for shared_module are present
    let workspaces: Vec<_> = shared_module_results.iter()
        .filter_map(|r| r.workspace.as_ref())
        .collect();
    assert!(workspaces.contains(&&"dev".to_string()));
    assert!(workspaces.contains(&&"staging".to_string()));
    assert!(workspaces.contains(&&"prod".to_string()));
    
    // Verify other modules are present
    let other_module_results: Vec<_> = results.iter()
        .filter(|r| r.module_path == "other_module")
        .collect();
    assert_eq!(other_module_results.len(), 1);
    
    let another_module_results: Vec<_> = results.iter()
        .filter(|r| r.module_path == "another_module")
        .collect();
    assert_eq!(another_module_results.len(), 1);
} 
