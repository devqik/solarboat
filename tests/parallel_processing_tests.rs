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
fn test_empty_processor() {
    let mut processor = ParallelProcessor::new(3);
    processor.start().expect("Failed to start processor");
    let results = processor.wait_for_completion().expect("Failed to wait for completion");
    assert_eq!(results.len(), 0);
}

#[test]
fn test_single_operation() {
    let mut processor = ParallelProcessor::new(3);
    
    let operation = TerraformOperation {
        module_path: "test_module".to_string(),
        workspace: Some("test_workspace".to_string()),
        operation_type: OperationType::Plan { plan_dir: None },
        var_files: vec!["test.tfvars".to_string()],
        watch: false,
        skip_init: true,
    };
    
    processor.add_operation(operation).expect("Failed to add operation");
    processor.start().expect("Failed to start processor");
    let results = processor.wait_for_completion().expect("Failed to wait for completion");
    assert_eq!(results.len(), 1);
    
    let result = &results[0];
    assert_eq!(result.module_path, "test_module");
    assert_eq!(result.workspace, Some("test_workspace".to_string()));
    match &result.operation_type {
        OperationType::Plan { .. } => {},
        _ => panic!("Expected Plan operation"),
    }
}

#[test]
fn test_multiple_operations() {
    let mut processor = ParallelProcessor::new(3);
    
    for i in 0..2 {
        let operation = TerraformOperation {
            module_path: format!("test_module_{}", i),
            workspace: Some(format!("test_workspace_{}", i)),
            operation_type: OperationType::Plan { plan_dir: None },
            var_files: vec!["test.tfvars".to_string()],
            watch: false,
            skip_init: true,
        };
        processor.add_operation(operation).expect("Failed to add operation");
    }
    
    processor.start().expect("Failed to start processor");
    let results = processor.wait_for_completion().expect("Failed to wait for completion");
    assert_eq!(results.len(), 2);
    
    // Since parallel processing doesn't guarantee order, we need to check that both expected results exist
    let expected_modules = vec!["test_module_0", "test_module_1"];
    let expected_workspaces = vec!["test_workspace_0", "test_workspace_1"];
    
    for expected_module in &expected_modules {
        assert!(results.iter().any(|r| r.module_path == *expected_module), 
                "Expected module {} not found in results", expected_module);
    }
    
    for expected_workspace in &expected_workspaces {
        assert!(results.iter().any(|r| r.workspace == Some(expected_workspace.to_string())), 
                "Expected workspace {} not found in results", expected_workspace);
    }
}

#[test]
fn test_parallel_limit() {
    let mut processor = ParallelProcessor::new(2);
    
    for i in 0..3 {
        let operation = TerraformOperation {
            module_path: format!("test_module_{}", i),
            workspace: Some(format!("test_workspace_{}", i)),
            operation_type: OperationType::Plan { plan_dir: None },
            var_files: vec!["test.tfvars".to_string()],
            watch: false,
            skip_init: true,
        };
        processor.add_operation(operation).expect("Failed to add operation");
    }
    
    processor.start().expect("Failed to start processor");
    let results = processor.wait_for_completion().expect("Failed to wait for completion");
    assert_eq!(results.len(), 3);
}

#[test]
fn test_apply_operations() {
    let mut processor = ParallelProcessor::new(3);
    
    for i in 0..3 {
        let operation = TerraformOperation {
            module_path: format!("test_module_{}", i),
            workspace: Some(format!("test_workspace_{}", i)),
            operation_type: OperationType::Apply,
            var_files: vec!["test.tfvars".to_string()],
            watch: false,
            skip_init: true,
        };
        processor.add_operation(operation).expect("Failed to add operation");
    }
    
    processor.start().expect("Failed to start processor");
    let results = processor.wait_for_completion().expect("Failed to wait for completion");
    assert_eq!(results.len(), 3);
    
    for result in results {
        match result.operation_type {
            OperationType::Apply => {},
            _ => panic!("Expected Apply operation"),
        }
    }
}

#[test]
fn test_high_parallel_limit() {
    let mut processor = ParallelProcessor::new(10); // Should be clamped to 4
    assert_eq!(processor.get_parallel_limit(), 4);
    
    for i in 0..5 {
        let operation = TerraformOperation {
            module_path: format!("test_module_{}", i),
            workspace: Some(format!("test_workspace_{}", i)),
            operation_type: OperationType::Plan { plan_dir: None },
            var_files: vec!["test.tfvars".to_string()],
            watch: false,
            skip_init: true,
        };
        processor.add_operation(operation).expect("Failed to add operation");
    }
    
    processor.start().expect("Failed to start processor");
    let results = processor.wait_for_completion().expect("Failed to wait for completion");
    assert_eq!(results.len(), 5);
}

#[test]
fn test_module_grouping() {
    let mut processor = ParallelProcessor::new(3);
    
    // Add operations for the same module but different workspaces
    for workspace in &["dev", "staging", "prod"] {
        let operation = TerraformOperation {
            module_path: "shared_module".to_string(),
            workspace: Some(workspace.to_string()),
            operation_type: OperationType::Plan { plan_dir: None },
            var_files: vec!["test.tfvars".to_string()],
            watch: false,
            skip_init: true,
        };
        processor.add_operation(operation).expect("Failed to add operation");
    }
    
    // Add operations for different modules
    for module in &["other_module", "another_module"] {
        let operation = TerraformOperation {
            module_path: module.to_string(),
            workspace: Some("default".to_string()),
            operation_type: OperationType::Plan { plan_dir: None },
            var_files: vec!["test.tfvars".to_string()],
            watch: false,
            skip_init: true,
        };
        processor.add_operation(operation).expect("Failed to add operation");
    }
    
    processor.start().expect("Failed to start processor");
    let results = processor.wait_for_completion().expect("Failed to wait for completion");
    assert_eq!(results.len(), 5);
    
    // Check that all operations for shared_module are present
    let shared_module_results: Vec<_> = results
        .iter()
        .filter(|r| r.module_path == "shared_module")
        .collect();
    assert_eq!(shared_module_results.len(), 3);
    
    // Check that all workspaces for shared_module are present
    let workspaces: Vec<_> = shared_module_results
        .iter()
        .filter_map(|r| r.workspace.as_ref())
        .collect();
    assert!(workspaces.contains(&&"dev".to_string()));
    assert!(workspaces.contains(&&"staging".to_string()));
    assert!(workspaces.contains(&&"prod".to_string()));
    
    // Check other modules
    let other_module_results: Vec<_> = results
        .iter()
        .filter(|r| r.module_path == "other_module")
        .collect();
    assert_eq!(other_module_results.len(), 1);
    
    let another_module_results: Vec<_> = results
        .iter()
        .filter(|r| r.module_path == "another_module")
        .collect();
    assert_eq!(another_module_results.len(), 1);
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
