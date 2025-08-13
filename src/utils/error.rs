use std::fmt;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::sync::LazyLock;

/// Custom error types for Solarboat
#[derive(Debug, Clone)]
pub enum SolarboatError {
    /// File system related errors
    FileSystem {
        operation: String,
        path: String,
        cause: String,
    },
    /// Terraform operation errors
    Terraform {
        operation: String,
        module: String,
        workspace: Option<String>,
        cause: String,
        is_transient: bool,
    },
    /// Process execution errors
    Process {
        command: String,
        args: Vec<String>,
        cause: String,
        exit_code: Option<i32>,
    },
    /// Lock acquisition errors
    Lock {
        resource: String,
        timeout: Duration,
        cause: String,
    },
    /// Configuration errors
    Configuration {
        field: String,
        value: String,
        cause: String,
    },
    /// Network/API errors
    Network {
        endpoint: String,
        cause: String,
        is_transient: bool,
    },
    /// State management errors
    State {
        operation: String,
        cause: String,
    },
    /// Validation errors
    Validation {
        field: String,
        value: String,
        cause: String,
    },
}

impl fmt::Display for SolarboatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolarboatError::FileSystem { operation, path, cause } => {
                write!(f, "File system error during {} at {}: {}", operation, path, cause)
            }
            SolarboatError::Terraform { operation, module, workspace, cause, is_transient } => {
                let workspace_info = workspace.as_ref().map(|w| format!(":{}", w)).unwrap_or_default();
                let transient_info = if *is_transient { " (transient)" } else { "" };
                write!(f, "Terraform {} failed for {}{}{}: {}", operation, module, workspace_info, transient_info, cause)
            }
            SolarboatError::Process { command, args, cause, exit_code } => {
                let args_str = args.join(" ");
                let exit_info = exit_code.map(|c| format!(" (exit code: {})", c)).unwrap_or_default();
                write!(f, "Process '{} {}' failed{}: {}", command, args_str, exit_info, cause)
            }
            SolarboatError::Lock { resource, timeout, cause } => {
                write!(f, "Failed to acquire lock on {} after {:?}: {}", resource, timeout, cause)
            }
            SolarboatError::Configuration { field, value, cause } => {
                write!(f, "Configuration error in field '{}' with value '{}': {}", field, value, cause)
            }
            SolarboatError::Network { endpoint, cause, is_transient } => {
                let transient_info = if *is_transient { " (transient)" } else { "" };
                write!(f, "Network error for endpoint '{}'{}: {}", endpoint, transient_info, cause)
            }
            SolarboatError::State { operation, cause } => {
                write!(f, "State management error during {}: {}", operation, cause)
            }
            SolarboatError::Validation { field, value, cause } => {
                write!(f, "Validation error for field '{}' with value '{}': {}", field, value, cause)
            }
        }
    }
}

impl std::error::Error for SolarboatError {}

/// Error categorization for recovery strategies
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorCategory {
    Transient,    // Temporary errors that can be retried
    Permanent,    // Permanent errors that should not be retried
    Configuration, // Configuration errors that need user intervention
    System,       // System-level errors
}

impl SolarboatError {
    pub fn category(&self) -> ErrorCategory {
        match self {
            SolarboatError::FileSystem { .. } => ErrorCategory::System,
            SolarboatError::Terraform { is_transient, .. } => {
                if *is_transient {
                    ErrorCategory::Transient
                } else {
                    ErrorCategory::Permanent
                }
            }
            SolarboatError::Process { .. } => ErrorCategory::System,
            SolarboatError::Lock { .. } => ErrorCategory::Transient,
            SolarboatError::Configuration { .. } => ErrorCategory::Configuration,
            SolarboatError::Network { is_transient, .. } => {
                if *is_transient {
                    ErrorCategory::Transient
                } else {
                    ErrorCategory::Permanent
                }
            }
            SolarboatError::State { .. } => ErrorCategory::System,
            SolarboatError::Validation { .. } => ErrorCategory::Configuration,
        }
    }

    pub fn is_retryable(&self) -> bool {
        self.category() == ErrorCategory::Transient
    }
}

/// Exponential backoff configuration
#[derive(Debug, Clone)]
pub struct BackoffConfig {
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
    pub max_attempts: usize,
    pub jitter: bool,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            multiplier: 2.0,
            max_attempts: 5,
            jitter: true,
        }
    }
}

/// Exponential backoff retry mechanism
pub struct ExponentialBackoff {
    config: BackoffConfig,
    current_attempt: usize,
    current_delay: Duration,
}

impl ExponentialBackoff {
    pub fn new(config: BackoffConfig) -> Self {
        Self {
            current_delay: config.initial_delay,
            current_attempt: 0,
            config,
        }
    }

    pub fn next_delay(&mut self) -> Option<Duration> {
        if self.current_attempt >= self.config.max_attempts {
            return None;
        }

        self.current_attempt += 1;
        let delay = if self.config.jitter {
            self.add_jitter(self.current_delay)
        } else {
            self.current_delay
        };

        // Calculate next delay
        self.current_delay = Duration::from_secs_f64(
            (self.current_delay.as_secs_f64() * self.config.multiplier)
                .min(self.config.max_delay.as_secs_f64())
        );

        Some(delay)
    }

    fn add_jitter(&self, delay: Duration) -> Duration {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        Instant::now().hash(&mut hasher);
        let jitter_factor = (hasher.finish() % 100) as f64 / 100.0;
        
        Duration::from_secs_f64(delay.as_secs_f64() * (0.5 + jitter_factor * 0.5))
    }

    pub fn reset(&mut self) {
        self.current_attempt = 0;
        self.current_delay = self.config.initial_delay;
    }

    pub fn current_attempt(&self) -> usize {
        self.current_attempt
    }
}

/// Circuit breaker for preventing cascading failures
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    failure_threshold: usize,
    recovery_timeout: Duration,
    last_failure_time: Option<Instant>,
    failure_count: usize,
    state: CircuitState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,    // Normal operation
    Open,      // Circuit is open, failing fast
    HalfOpen,  // Testing if service is recovered
}

impl CircuitBreaker {
    pub fn new(failure_threshold: usize, recovery_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            recovery_timeout,
            last_failure_time: None,
            failure_count: 0,
            state: CircuitState::Closed,
        }
    }

    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if Instant::now().duration_since(last_failure) >= self.recovery_timeout {
                        self.state = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn on_success(&mut self) {
        match self.state {
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                self.state = CircuitState::Closed;
                self.failure_count = 0;
                self.last_failure_time = None;
            }
            CircuitState::Open => {}
        }
    }

    pub fn on_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(Instant::now());

        match self.state {
            CircuitState::Closed => {
                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
            }
            CircuitState::Open => {}
        }
    }

    pub fn state(&self) -> &CircuitState {
        &self.state
    }
}

/// Error recovery context for tracking and managing errors
#[derive(Debug)]
pub struct ErrorRecoveryContext {
    errors: Arc<Mutex<Vec<(SolarboatError, Instant)>>>,
    circuit_breakers: Arc<Mutex<HashMap<String, CircuitBreaker>>>,
    backoff_configs: Arc<Mutex<HashMap<String, BackoffConfig>>>,
}

impl Default for ErrorRecoveryContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorRecoveryContext {
    pub fn new() -> Self {
        Self {
            errors: Arc::new(Mutex::new(Vec::new())),
            circuit_breakers: Arc::new(Mutex::new(HashMap::new())),
            backoff_configs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn record_error(&self, error: SolarboatError) {
        let mut errors = self.errors.lock().expect("Failed to acquire errors lock");
        errors.push((error, Instant::now()));
    }

    pub fn get_circuit_breaker(&self, key: &str) -> CircuitBreaker {
        let mut breakers = self.circuit_breakers.lock().expect("Failed to acquire circuit breakers lock");
        breakers.entry(key.to_string()).or_insert_with(|| {
            CircuitBreaker::new(3, Duration::from_secs(30))
        }).clone()
    }

    pub fn update_circuit_breaker(&self, key: &str, success: bool) {
        let mut breakers = self.circuit_breakers.lock().expect("Failed to acquire circuit breakers lock");
        if let Some(breaker) = breakers.get_mut(key) {
            if success {
                breaker.on_success();
            } else {
                breaker.on_failure();
            }
        }
    }

    pub fn get_backoff_config(&self, key: &str) -> BackoffConfig {
        let configs = self.backoff_configs.lock().expect("Failed to acquire backoff configs lock");
        configs.get(key).cloned().unwrap_or_default()
    }

    pub fn set_backoff_config(&self, key: &str, config: BackoffConfig) {
        let mut configs = self.backoff_configs.lock().expect("Failed to acquire backoff configs lock");
        configs.insert(key.to_string(), config);
    }

    pub fn get_recent_errors(&self, duration: Duration) -> Vec<SolarboatError> {
        let errors = self.errors.lock().expect("Failed to acquire errors lock");
        let cutoff = Instant::now().checked_sub(duration).unwrap_or(Instant::now());
        
        errors.iter()
            .filter(|(_, timestamp)| *timestamp >= cutoff)
            .map(|(error, _)| error.clone())
            .collect()
    }

    pub fn clear_old_errors(&self, older_than: Duration) {
        let mut errors = self.errors.lock().expect("Failed to acquire errors lock");
        let cutoff = Instant::now().checked_sub(older_than).unwrap_or(Instant::now());
        errors.retain(|(_, timestamp)| *timestamp >= cutoff);
    }
}

/// Safe wrapper for common operations that might fail
pub struct SafeOperations;

impl SafeOperations {
    /// Safely get current directory with proper error handling
    pub fn current_dir() -> Result<std::path::PathBuf, SolarboatError> {
        std::env::current_dir().map_err(|e| SolarboatError::FileSystem {
            operation: "get current directory".to_string(),
            path: ".".to_string(),
            cause: e.to_string(),
        })
    }

    /// Safely canonicalize a path with proper error handling
    pub fn canonicalize(path: &std::path::Path) -> Result<std::path::PathBuf, SolarboatError> {
        path.canonicalize().map_err(|e| SolarboatError::FileSystem {
            operation: "canonicalize path".to_string(),
            path: path.to_string_lossy().to_string(),
            cause: e.to_string(),
        })
    }

    /// Safely convert OsStr to string with proper error handling
    pub fn os_str_to_string(os_str: &std::ffi::OsStr) -> Result<String, SolarboatError> {
        os_str.to_str().ok_or_else(|| SolarboatError::Validation {
            field: "path".to_string(),
            value: os_str.to_string_lossy().to_string(),
            cause: "Invalid UTF-8 sequence".to_string(),
        }).map(|s| s.to_string())
    }

    /// Safely acquire a mutex lock with timeout
    pub fn lock_with_timeout<'a, T>(
        mutex: &'a Arc<Mutex<T>>,
        timeout: Duration,
        resource_name: &str,
    ) -> Result<std::sync::MutexGuard<'a, T>, SolarboatError> {
        let start = Instant::now();
        
        loop {
            match mutex.try_lock() {
                Ok(guard) => return Ok(guard),
                Err(_) => {
                    if start.elapsed() >= timeout {
                        return Err(SolarboatError::Lock {
                            resource: resource_name.to_string(),
                            timeout,
                            cause: "Mutex lock timeout".to_string(),
                        });
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }

    /// Execute a function with retry logic (synchronous version)
    pub fn with_retry<F, T, E>(
        mut f: F,
        config: BackoffConfig,
        error_context: &str,
    ) -> Result<T, SolarboatError>
    where
        F: FnMut() -> Result<T, E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        let mut backoff = ExponentialBackoff::new(config.clone());

        loop {
            match f() {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if let Some(delay) = backoff.next_delay() {
                        eprintln!("{} failed (attempt {}/{}), retrying in {:?}: {}", 
                            error_context, 
                            backoff.current_attempt(), 
                            config.max_attempts, 
                            delay, 
                            e
                        );
                        std::thread::sleep(delay);
                    } else {
                        break;
                    }
                }
            }
        }

        Err(SolarboatError::Process {
            command: error_context.to_string(),
            args: vec![],
            cause: format!("Failed after {} attempts", config.max_attempts),
            exit_code: None,
        })
    }
}

/// Rollback context for managing failed operations
pub struct RollbackContext {
    operations: Arc<Mutex<Vec<RollbackOperation>>>,
}

impl Default for RollbackContext {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RollbackOperation {
    pub module: String,
    pub workspace: Option<String>,
    pub operation_type: String,
    pub rollback_fn: Box<dyn Fn() -> Result<(), String> + Send + Sync>,
}

impl RollbackContext {
    pub fn new() -> Self {
        Self {
            operations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_operation<F>(&self, module: String, workspace: Option<String>, operation_type: String, rollback_fn: F)
    where
        F: Fn() -> Result<(), String> + Send + Sync + 'static,
    {
        let mut operations = self.operations.lock().expect("Failed to acquire operations lock");
        operations.push(RollbackOperation {
            module,
            workspace,
            operation_type,
            rollback_fn: Box::new(rollback_fn),
        });
    }

    pub fn execute_rollback(&self) -> Vec<(String, Result<(), String>)> {
        let mut operations = self.operations.lock().expect("Failed to acquire operations lock");
        let mut results = Vec::new();

        // Execute rollback operations in reverse order
        while let Some(operation) = operations.pop() {
            let module_info = match &operation.workspace {
                Some(ws) => format!("{}:{}", operation.module, ws),
                None => operation.module.clone(),
            };
            
            let result = (operation.rollback_fn)();
            results.push((module_info, result));
        }

        results
    }

    pub fn clear(&self) {
        let mut operations = self.operations.lock().expect("Failed to acquire operations lock");
        operations.clear();
    }
}

// Global error recovery context
pub static ERROR_CONTEXT: LazyLock<ErrorRecoveryContext> = LazyLock::new(ErrorRecoveryContext::new);
pub static ROLLBACK_CONTEXT: LazyLock<RollbackContext> = LazyLock::new(RollbackContext::new);
