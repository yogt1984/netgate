use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries (milliseconds)
    pub initial_delay_ms: u64,
    /// Maximum delay between retries (milliseconds)
    pub max_delay_ms: u64,
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    /// Whether to use jitter (random variation in delay)
    pub use_jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            use_jitter: true,
        }
    }
}

impl RetryConfig {
    pub fn new(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            ..Default::default()
        }
    }

    /// Calculate delay for a given attempt number
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = (self.initial_delay_ms as f64) * (self.backoff_multiplier.powi(attempt as i32 - 1));
        let delay_ms = base_delay.min(self.max_delay_ms as f64) as u64;
        
        let final_delay = if self.use_jitter {
            // Add jitter: random variation between 0.5x and 1.5x
            let jitter_range = delay_ms / 2;
            let jitter = fastrand::u64(0..=jitter_range * 2);
            delay_ms.saturating_sub(jitter_range).saturating_add(jitter)
        } else {
            delay_ms
        };
        
        Duration::from_millis(final_delay)
    }
}

/// Error types that should trigger retries
pub trait RetryableError: std::error::Error {
    /// Check if this error should trigger a retry
    fn is_retryable(&self) -> bool;
}

/// Retry a function with exponential backoff
pub async fn retry_with_backoff<F, T, E>(
    config: &RetryConfig,
    operation: F,
) -> Result<T, E>
where
    F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
    E: RetryableError + Send + 'static,
{
    let mut last_error = None;
    
    for attempt in 1..=config.max_attempts {
        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    debug!("Operation succeeded after {} attempts", attempt);
                }
                return Ok(result);
            }
            Err(e) => {
                last_error = Some(e);
                let err = last_error.as_ref().unwrap();
                
                // Check if error is retryable
                if !err.is_retryable() {
                    debug!("Error is not retryable, aborting");
                    break;
                }
                
                // Don't retry on last attempt
                if attempt < config.max_attempts {
                    let delay = config.calculate_delay(attempt);
                    warn!(
                        "Operation failed (attempt {}/{}), retrying in {:?}: {}",
                        attempt,
                        config.max_attempts,
                        delay,
                        err
                    );
                    sleep(delay).await;
                }
            }
        }
    }
    
    Err(last_error.expect("Should have at least one error"))
}

/// Retry a function with custom retry logic
pub async fn retry<F, T, E>(
    max_attempts: u32,
    operation: F,
) -> Result<T, E>
where
    F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
    E: RetryableError + Send + 'static,
{
    let config = RetryConfig::new(max_attempts);
    retry_with_backoff(&config, operation).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use thiserror::Error;

    #[derive(Error, Debug)]
    #[error("Test error")]
    struct TestError {
        retryable: bool,
    }

    impl RetryableError for TestError {
        fn is_retryable(&self) -> bool {
            self.retryable
        }
    }

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let config = RetryConfig::default();
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = Arc::clone(&call_count);
        
        let result = retry_with_backoff(&config, move || {
            let count = Arc::clone(&call_count_clone);
            Box::pin(async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok::<i32, TestError>(42)
            })
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 10,
            max_delay_ms: 100,
            backoff_multiplier: 2.0,
            use_jitter: false,
        };
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = Arc::clone(&call_count);
        
        let result = retry_with_backoff(&config, move || {
            let count = Arc::clone(&call_count_clone);
            Box::pin(async move {
                let attempt = count.fetch_add(1, Ordering::SeqCst);
                if attempt < 2 {
                    Err(TestError { retryable: true })
                } else {
                    Ok::<i32, TestError>(42)
                }
            })
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_fails_after_max_attempts() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 10,
            max_delay_ms: 100,
            backoff_multiplier: 2.0,
            use_jitter: false,
        };
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = Arc::clone(&call_count);
        
        let result: Result<i32, TestError> = retry_with_backoff(&config, move || {
            let count = Arc::clone(&call_count_clone);
            Box::pin(async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err(TestError { retryable: true })
            })
        }).await;
        
        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_stops_on_non_retryable_error() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 10,
            max_delay_ms: 100,
            backoff_multiplier: 2.0,
            use_jitter: false,
        };
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = Arc::clone(&call_count);
        
        let result: Result<i32, TestError> = retry_with_backoff(&config, move || {
            let count = Arc::clone(&call_count_clone);
            Box::pin(async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err(TestError { retryable: false })
            })
        }).await;
        
        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_retry_config_calculate_delay() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 1000,
            backoff_multiplier: 2.0,
            use_jitter: false,
        };
        
        let delay1 = config.calculate_delay(1);
        assert_eq!(delay1.as_millis(), 100);
        
        let delay2 = config.calculate_delay(2);
        assert_eq!(delay2.as_millis(), 200);
        
        let delay3 = config.calculate_delay(3);
        assert_eq!(delay3.as_millis(), 400);
    }

    #[test]
    fn test_retry_config_respects_max_delay() {
        let config = RetryConfig {
            max_attempts: 5,
            initial_delay_ms: 1000,
            max_delay_ms: 2000,
            backoff_multiplier: 2.0,
            use_jitter: false,
        };
        
        let delay4 = config.calculate_delay(4);
        assert!(delay4.as_millis() <= 2000);
    }
}

