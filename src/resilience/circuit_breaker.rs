use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed - normal operation
    Closed,
    /// Circuit is open - failing, reject requests immediately
    Open,
    /// Circuit is half-open - testing if service recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Failure threshold - number of failures before opening circuit
    pub failure_threshold: u32,
    /// Success threshold - number of successes in half-open to close circuit
    pub success_threshold: u32,
    /// Timeout duration for open state before transitioning to half-open
    pub timeout_duration: Duration,
    /// Window duration for counting failures
    pub window_duration: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_duration: Duration::from_secs(60),
            window_duration: Duration::from_secs(60),
        }
    }
}

/// Internal state for circuit breaker
struct CircuitBreakerState {
    state: Arc<AtomicU32>, // 0=Closed, 1=Open, 2=HalfOpen
    failure_count: Arc<AtomicU32>,
    success_count: Arc<AtomicU32>,
    last_failure_time: Arc<AtomicU64>, // Unix timestamp in milliseconds
    state_changed_time: Arc<AtomicU64>, // When state last changed
}

impl CircuitBreakerState {
    fn new() -> Self {
        Self {
            state: Arc::new(AtomicU32::new(0)), // Closed
            failure_count: Arc::new(AtomicU32::new(0)),
            success_count: Arc::new(AtomicU32::new(0)),
            last_failure_time: Arc::new(AtomicU64::new(0)),
            state_changed_time: Arc::new(AtomicU64::new(0)),
        }
    }

    fn get_state(&self) -> CircuitState {
        match self.state.load(Ordering::SeqCst) {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }

    fn set_state(&self, new_state: CircuitState) {
        let state_val = match new_state {
            CircuitState::Closed => 0,
            CircuitState::Open => 1,
            CircuitState::HalfOpen => 2,
        };
        self.state.store(state_val, Ordering::SeqCst);
        self.state_changed_time.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            Ordering::SeqCst,
        );
    }
}

/// Circuit breaker for protecting external service calls
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: CircuitBreakerState,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default configuration
    pub fn new() -> Self {
        Self {
            config: CircuitBreakerConfig::default(),
            state: CircuitBreakerState::new(),
        }
    }

    /// Create a new circuit breaker with custom configuration
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: CircuitBreakerState::new(),
        }
    }

    /// Check if request should be allowed
    pub fn allow_request(&self) -> bool {
        let current_state = self.state.get_state();
        
        match current_state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has passed
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let state_changed = self.state.state_changed_time.load(Ordering::SeqCst);
                
                if now.saturating_sub(state_changed) >= self.config.timeout_duration.as_millis() as u64 {
                    // Transition to half-open
                    debug!("Circuit breaker transitioning from Open to HalfOpen");
                    self.state.set_state(CircuitState::HalfOpen);
                    self.state.success_count.store(0, Ordering::SeqCst);
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true, // Allow limited requests to test recovery
        }
    }

    /// Record a successful call
    pub fn record_success(&self) {
        let current_state = self.state.get_state();
        
        match current_state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.state.failure_count.store(0, Ordering::SeqCst);
            }
            CircuitState::HalfOpen => {
                let success_count = self.state.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                if success_count >= self.config.success_threshold {
                    debug!("Circuit breaker transitioning from HalfOpen to Closed");
                    self.state.set_state(CircuitState::Closed);
                    self.state.failure_count.store(0, Ordering::SeqCst);
                    self.state.success_count.store(0, Ordering::SeqCst);
                }
            }
            CircuitState::Open => {
                // Should not happen, but handle gracefully
            }
        }
    }

    /// Record a failed call
    pub fn record_failure(&self) {
        let current_state = self.state.get_state();
        
        match current_state {
            CircuitState::Closed => {
                let failure_count = self.state.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                self.state.last_failure_time.store(now, Ordering::SeqCst);
                
                if failure_count >= self.config.failure_threshold {
                    warn!("Circuit breaker transitioning from Closed to Open ({} failures)", failure_count);
                    self.state.set_state(CircuitState::Open);
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open immediately opens the circuit
                warn!("Circuit breaker transitioning from HalfOpen to Open (failure detected)");
                self.state.set_state(CircuitState::Open);
                self.state.success_count.store(0, Ordering::SeqCst);
            }
            CircuitState::Open => {
                // Already open, just update failure time
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                self.state.last_failure_time.store(now, Ordering::SeqCst);
            }
        }
    }

    /// Get current circuit state
    pub fn state(&self) -> CircuitState {
        self.state.get_state()
    }

    /// Get current failure count
    pub fn failure_count(&self) -> u32 {
        self.state.failure_count.load(Ordering::SeqCst)
    }

    /// Reset circuit breaker to closed state
    pub fn reset(&self) {
        self.state.set_state(CircuitState::Closed);
        self.state.failure_count.store(0, Ordering::SeqCst);
        self.state.success_count.store(0, Ordering::SeqCst);
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_circuit_breaker_starts_closed() {
        let cb = CircuitBreaker::new();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_request());
    }

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let cb = CircuitBreaker::new();
        
        // Record failures up to threshold
        for _ in 0..cb.config.failure_threshold {
            cb.record_failure();
        }
        
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.allow_request());
    }

    #[test]
    fn test_circuit_breaker_resets_on_success() {
        let cb = CircuitBreaker::new();
        
        // Record some failures
        cb.record_failure();
        cb.record_failure();
        
        // Success should reset failure count
        cb.record_success();
        assert_eq!(cb.failure_count(), 0);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_transitions_to_half_open() {
        let mut config = CircuitBreakerConfig::default();
        config.timeout_duration = Duration::from_millis(100);
        let cb = CircuitBreaker::with_config(config);
        
        // Open the circuit
        for _ in 0..cb.config.failure_threshold {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);
        
        // Wait for timeout
        std::thread::sleep(Duration::from_millis(150));
        
        // Should transition to half-open
        assert!(cb.allow_request());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_breaker_closes_after_success_threshold() {
        let mut config = CircuitBreakerConfig::default();
        config.timeout_duration = Duration::from_millis(100);
        config.success_threshold = 2;
        let cb = CircuitBreaker::with_config(config);
        
        // Open the circuit
        for _ in 0..cb.config.failure_threshold {
            cb.record_failure();
        }
        
        // Wait and transition to half-open
        std::thread::sleep(Duration::from_millis(150));
        cb.allow_request();
        
        // Record successes
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_failure_in_half_open_opens_again() {
        let mut config = CircuitBreakerConfig::default();
        config.timeout_duration = Duration::from_millis(100);
        let cb = CircuitBreaker::with_config(config);
        
        // Open the circuit
        for _ in 0..cb.config.failure_threshold {
            cb.record_failure();
        }
        
        // Wait and transition to half-open
        std::thread::sleep(Duration::from_millis(150));
        cb.allow_request();
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        
        // Failure should immediately open again
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let cb = CircuitBreaker::new();
        
        // Open the circuit
        for _ in 0..cb.config.failure_threshold {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);
        
        // Reset
        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
    }
}

