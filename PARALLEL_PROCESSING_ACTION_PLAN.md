# Parallel Processing Action Plan for Module Operations

## Context

The goal is to safely and efficiently process many modules in parallel (for both `plan` and `apply` operations) without overwhelming system resources or causing instability. The system should:

- Allow the user to specify the number of parallel operations (with a maximum of 4).
- Queue additional modules if more than 4 are requested, processing them as threads become available.
- Ensure robust error handling, resource cleanup, and graceful shutdown.
- Provide clear documentation and tests for reliability.

## Current Infrastructure

The codebase already has a robust `BackgroundTerraform` system in `src/utils/terraform_background.rs` that provides:

- `BackgroundTerraform` struct for managing background terraform processes
- `init_background()`, `plan_background()`, and `apply_background()` methods
- `wait_for_completion()` with timeout support
- Status tracking and output collection
- Process cleanup with `kill()` method

## Implementation Strategy

We will extend the existing `terraform_background.rs` utilities to add parallel processing capabilities while keeping the plan and apply helpers clean and simple.

## Action Steps

1. **Extend terraform_background.rs with parallel processing utilities**

   - Create a `ParallelProcessor` struct that manages a thread pool/worker queue
   - Add methods for queuing operations and managing concurrency limits
   - Ensure proper error handling and resource cleanup
   - Keep the existing `BackgroundTerraform` API unchanged

2. **Update CLI and argument parsing to support a `--parallel` argument**

   - Add `--parallel` argument to CLI args, clamp to max 4
   - Pass the parallel value to both plan and apply commands
   - Update help text and documentation

3. **Refactor plan helpers to use the new parallel processing utilities**

   - Keep the existing logic but wrap it with the new `ParallelProcessor`
   - Ensure all existing functionality (workspaces, var files, etc.) is preserved
   - Maintain the same error handling and output format

4. **Refactor apply helpers to use the new parallel processing utilities**

   - Mirror the plan helpers approach using the same `ParallelProcessor`
   - Preserve all existing functionality and error handling
   - Ensure consistency between plan and apply operations

5. **Add tests and documentation for the new parallel processing system**
   - Test edge cases (e.g., >4 modules, error propagation, shutdown)
   - Document the new parallel processing capabilities
   - Ensure the system is reliable and well-documented

## Dependencies

- Step 3 and 4 depend on the parallel processing utilities in Step 1
- Step 5 depends on the completion of Steps 3 and 4

## Design Notes

- The `ParallelProcessor` will be a new struct in `terraform_background.rs`
- It will manage a queue of operations and spawn worker threads as needed
- Each worker thread will use the existing `BackgroundTerraform` functionality
- The plan and apply helpers will remain largely unchanged, just wrapped with the parallel processor
- Error handling will be consistent with the existing patterns

---

This plan will be updated as requirements evolve or as new context is provided.
