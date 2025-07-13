use solarboat::utils::parallel_processor::ParallelProcessor;
use solarboat::utils::terraform_operations::{TerraformOperation, OperationType};
use std::thread;
use std::time::Duration;

// Mock operation for testing
fn create_mock_operation(id: u32) -> TerraformOperation {
    TerraformOperation {
        module_path: format!("test-module-{}", id),
        workspace: Some(format!("workspace-{}", id)),
        var_files: vec![],
        operation_type: OperationType::Plan { plan_dir: None },
        watch: false,
    }
}

#[test]
fn test_parallel_processor_creation_and_clamping() {
    // Test that parallel limit is clamped to 1-4
    let processor = ParallelProcessor::new(0);
    assert_eq!(processor.get_parallel_limit(), 1);
    
    let processor = ParallelProcessor::new(1);
    assert_eq!(processor.get_parallel_limit(), 1);
    
    let processor = ParallelProcessor::new(4);
    assert_eq!(processor.get_parallel_limit(), 4);
    
    let processor = ParallelProcessor::new(10);
    assert_eq!(processor.get_parallel_limit(), 4);
}

#[test]
fn test_parallel_processor_operation_queuing() {
    let mut processor = ParallelProcessor::new(2);
    
    // Add 4 operations
    for i in 1..=4 {
        let operation = create_mock_operation(i);
        processor.add_operation(operation);
    }
    
    // Start processing
    processor.start();
    
    // Wait for completion
    let results = processor.wait_for_completion();
    
    // Verify all operations completed (even if they failed due to missing terraform)
    assert_eq!(results.len(), 4);
}

#[test]
fn test_parallel_processor_empty_queue() {
    let mut processor = ParallelProcessor::new(2);
    
    // Start processing with no operations
    processor.start();
    let results = processor.wait_for_completion();
    
    assert_eq!(results.len(), 0);
}

#[test]
fn test_parallel_processor_mixed_operation_types() {
    let mut processor = ParallelProcessor::new(2);
    
    // Add different operation types
    let plan_op = TerraformOperation {
        module_path: "test-plan".to_string(),
        workspace: None,
        var_files: vec![],
        operation_type: OperationType::Plan { plan_dir: Some("plans".to_string()) },
        watch: false,
    };
    
    let apply_op = TerraformOperation {
        module_path: "test-apply".to_string(),
        workspace: Some("prod".to_string()),
        var_files: vec!["vars.tfvars".to_string()],
        operation_type: OperationType::Apply,
        watch: true,
    };
    
    processor.add_operation(plan_op);
    processor.add_operation(apply_op);
    
    processor.start();
    let results = processor.wait_for_completion();
    
    assert_eq!(results.len(), 2);
    
    // Verify operation types are preserved
    let plan_result = results.iter().find(|r| r.module_path == "test-plan").unwrap();
    let apply_result = results.iter().find(|r| r.module_path == "test-apply").unwrap();
    
    match &plan_result.operation_type {
        OperationType::Plan { plan_dir } => {
            assert_eq!(plan_dir.as_ref().unwrap(), "plans");
        }
        _ => panic!("Expected Plan operation type"),
    }
    
    match &apply_result.operation_type {
        OperationType::Apply => {
            // This is correct
        }
        _ => panic!("Expected Apply operation type"),
    }
}

#[test]
fn test_parallel_processor_workspace_handling() {
    let mut processor = ParallelProcessor::new(2);
    
    // Add operations with different workspace configurations
    let op1 = TerraformOperation {
        module_path: "module1".to_string(),
        workspace: None, // Default workspace
        var_files: vec![],
        operation_type: OperationType::Plan { plan_dir: None },
        watch: false,
    };
    
    let op2 = TerraformOperation {
        module_path: "module2".to_string(),
        workspace: Some("prod".to_string()),
        var_files: vec!["prod.tfvars".to_string()],
        operation_type: OperationType::Apply,
        watch: true,
    };
    
    processor.add_operation(op1);
    processor.add_operation(op2);
    
    processor.start();
    let results = processor.wait_for_completion();
    
    assert_eq!(results.len(), 2);
    
    let default_ws_result = results.iter().find(|r| r.module_path == "module1").unwrap();
    let prod_ws_result = results.iter().find(|r| r.module_path == "module2").unwrap();
    
    assert!(default_ws_result.workspace.is_none());
    assert_eq!(prod_ws_result.workspace.as_ref().unwrap(), "prod");
}

// Integration test for CLI argument parsing
#[test]
fn test_cli_parallel_argument_parsing() {
    use solarboat::cli::{Args, Commands};
    use clap::Parser;
    
    // Test plan command with parallel argument
    let args = Args::try_parse_from(&["solarboat", "plan", "--parallel", "3"]).unwrap();
    match args.command {
        Commands::Plan(plan_args) => {
            assert_eq!(plan_args.parallel, 3);
        }
        _ => panic!("Expected Plan command"),
    }
    
    // Test apply command with parallel argument
    let args = Args::try_parse_from(&["solarboat", "apply", "--parallel", "4"]).unwrap();
    match args.command {
        Commands::Apply(apply_args) => {
            assert_eq!(apply_args.parallel, 4);
        }
        _ => panic!("Expected Apply command"),
    }
    
    // Test default value
    let args = Args::try_parse_from(&["solarboat", "plan"]).unwrap();
    match args.command {
        Commands::Plan(plan_args) => {
            assert_eq!(plan_args.parallel, 1);
        }
        _ => panic!("Expected Plan command"),
    }
}

// Test that the parallel processor handles many operations correctly
#[test]
fn test_parallel_processor_many_operations() {
    let mut processor = ParallelProcessor::new(4);
    
    // Add many operations to test queue behavior
    for i in 1..=10 {
        let operation = create_mock_operation(i);
        processor.add_operation(operation);
    }
    
    processor.start();
    let results = processor.wait_for_completion();
    
    assert_eq!(results.len(), 10);
    
    // Verify all operations were processed
    for i in 1..=10 {
        let expected_module = format!("test-module-{}", i);
        let result = results.iter().find(|r| r.module_path == expected_module);
        assert!(result.is_some(), "Operation {} was not processed", i);
    }
}

// Test that the parallel processor can handle operations with different configurations
#[test]
fn test_parallel_processor_varied_operations() {
    let mut processor = ParallelProcessor::new(3);
    
    // Add operations with different configurations
    let operations = vec![
        TerraformOperation {
            module_path: "module-a".to_string(),
            workspace: None,
            var_files: vec![],
            operation_type: OperationType::Plan { plan_dir: None },
            watch: false,
        },
        TerraformOperation {
            module_path: "module-b".to_string(),
            workspace: Some("dev".to_string()),
            var_files: vec!["dev.tfvars".to_string()],
            operation_type: OperationType::Plan { plan_dir: Some("plans".to_string()) },
            watch: true,
        },
        TerraformOperation {
            module_path: "module-c".to_string(),
            workspace: Some("prod".to_string()),
            var_files: vec!["prod.tfvars".to_string(), "secrets.tfvars".to_string()],
            operation_type: OperationType::Apply,
            watch: false,
        },
    ];
    
    for operation in operations {
        processor.add_operation(operation);
    }
    
    processor.start();
    let results = processor.wait_for_completion();
    
    assert_eq!(results.len(), 3);
    
    // Verify each operation was processed with correct configuration
    let module_a = results.iter().find(|r| r.module_path == "module-a").unwrap();
    let module_b = results.iter().find(|r| r.module_path == "module-b").unwrap();
    let module_c = results.iter().find(|r| r.module_path == "module-c").unwrap();
    
    assert!(module_a.workspace.is_none());
    assert_eq!(module_b.workspace.as_ref().unwrap(), "dev");
    assert_eq!(module_c.workspace.as_ref().unwrap(), "prod");
    
    match &module_b.operation_type {
        OperationType::Plan { plan_dir } => {
            assert_eq!(plan_dir.as_ref().unwrap(), "plans");
        }
        _ => panic!("Expected Plan operation type"),
    }
    
    match &module_c.operation_type {
        OperationType::Apply => {
            // This is correct
        }
        _ => panic!("Expected Apply operation type"),
    }
} 
