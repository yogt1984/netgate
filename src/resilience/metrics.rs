use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Metrics for tracking API call performance and health
#[derive(Debug, Clone)]
pub struct ApiMetrics {
    /// Total number of requests
    total_requests: Arc<AtomicU64>,
    /// Number of successful requests
    successful_requests: Arc<AtomicU64>,
    /// Number of failed requests
    failed_requests: Arc<AtomicU64>,
    /// Total response time in milliseconds
    total_response_time_ms: Arc<AtomicU64>,
    /// Number of retries performed
    total_retries: Arc<AtomicU64>,
    /// Number of circuit breaker rejections
    circuit_breaker_rejections: Arc<AtomicU64>,
    /// Timestamp of last request
    last_request_time: Arc<AtomicU64>,
}

impl ApiMetrics {
    pub fn new() -> Self {
        Self {
            total_requests: Arc::new(AtomicU64::new(0)),
            successful_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            total_response_time_ms: Arc::new(AtomicU64::new(0)),
            total_retries: Arc::new(AtomicU64::new(0)),
            circuit_breaker_rejections: Arc::new(AtomicU64::new(0)),
            last_request_time: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a request start
    pub fn record_request_start(&self) -> Instant {
        self.total_requests.fetch_add(1, Ordering::SeqCst);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.last_request_time.store(now, Ordering::SeqCst);
        Instant::now()
    }

    /// Record a successful request
    pub fn record_success(&self, start_time: Instant) {
        self.successful_requests.fetch_add(1, Ordering::SeqCst);
        let duration_ms = start_time.elapsed().as_millis() as u64;
        self.total_response_time_ms.fetch_add(duration_ms, Ordering::SeqCst);
    }

    /// Record a failed request
    pub fn record_failure(&self, start_time: Instant) {
        self.failed_requests.fetch_add(1, Ordering::SeqCst);
        let duration_ms = start_time.elapsed().as_millis() as u64;
        self.total_response_time_ms.fetch_add(duration_ms, Ordering::SeqCst);
    }

    /// Record a retry
    pub fn record_retry(&self) {
        self.total_retries.fetch_add(1, Ordering::SeqCst);
    }

    /// Record a circuit breaker rejection
    pub fn record_circuit_breaker_rejection(&self) {
        self.circuit_breaker_rejections.fetch_add(1, Ordering::SeqCst);
    }

    /// Get total number of requests
    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::SeqCst)
    }

    /// Get number of successful requests
    pub fn successful_requests(&self) -> u64 {
        self.successful_requests.load(Ordering::SeqCst)
    }

    /// Get number of failed requests
    pub fn failed_requests(&self) -> u64 {
        self.failed_requests.load(Ordering::SeqCst)
    }

    /// Get success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        let total = self.total_requests();
        if total == 0 {
            return 1.0;
        }
        self.successful_requests() as f64 / total as f64
    }

    /// Get failure rate (0.0 to 1.0)
    pub fn failure_rate(&self) -> f64 {
        let total = self.total_requests();
        if total == 0 {
            return 0.0;
        }
        self.failed_requests() as f64 / total as f64
    }

    /// Get average response time in milliseconds
    pub fn average_response_time_ms(&self) -> f64 {
        let total = self.total_requests();
        if total == 0 {
            return 0.0;
        }
        self.total_response_time_ms.load(Ordering::SeqCst) as f64 / total as f64
    }

    /// Get total number of retries
    pub fn total_retries(&self) -> u64 {
        self.total_retries.load(Ordering::SeqCst)
    }

    /// Get number of circuit breaker rejections
    pub fn circuit_breaker_rejections(&self) -> u64 {
        self.circuit_breaker_rejections.load(Ordering::SeqCst)
    }

    /// Get metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_requests: self.total_requests(),
            successful_requests: self.successful_requests(),
            failed_requests: self.failed_requests(),
            success_rate: self.success_rate(),
            failure_rate: self.failure_rate(),
            average_response_time_ms: self.average_response_time_ms(),
            total_retries: self.total_retries(),
            circuit_breaker_rejections: self.circuit_breaker_rejections(),
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.total_requests.store(0, Ordering::SeqCst);
        self.successful_requests.store(0, Ordering::SeqCst);
        self.failed_requests.store(0, Ordering::SeqCst);
        self.total_response_time_ms.store(0, Ordering::SeqCst);
        self.total_retries.store(0, Ordering::SeqCst);
        self.circuit_breaker_rejections.store(0, Ordering::SeqCst);
    }
}

impl Default for ApiMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of metrics at a point in time
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub success_rate: f64,
    pub failure_rate: f64,
    pub average_response_time_ms: f64,
    pub total_retries: u64,
    pub circuit_breaker_rejections: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_metrics_initial_state() {
        let metrics = ApiMetrics::new();
        assert_eq!(metrics.total_requests(), 0);
        assert_eq!(metrics.successful_requests(), 0);
        assert_eq!(metrics.failed_requests(), 0);
        assert_eq!(metrics.success_rate(), 1.0);
        assert_eq!(metrics.failure_rate(), 0.0);
    }

    #[test]
    fn test_metrics_record_success() {
        let metrics = ApiMetrics::new();
        let start = metrics.record_request_start();
        std::thread::sleep(Duration::from_millis(10));
        metrics.record_success(start);
        
        assert_eq!(metrics.total_requests(), 1);
        assert_eq!(metrics.successful_requests(), 1);
        assert_eq!(metrics.failed_requests(), 0);
        assert_eq!(metrics.success_rate(), 1.0);
        assert!(metrics.average_response_time_ms() >= 10.0);
    }

    #[test]
    fn test_metrics_record_failure() {
        let metrics = ApiMetrics::new();
        let start = metrics.record_request_start();
        metrics.record_failure(start);
        
        assert_eq!(metrics.total_requests(), 1);
        assert_eq!(metrics.successful_requests(), 0);
        assert_eq!(metrics.failed_requests(), 1);
        assert_eq!(metrics.success_rate(), 0.0);
        assert_eq!(metrics.failure_rate(), 1.0);
    }

    #[test]
    fn test_metrics_success_rate() {
        let metrics = ApiMetrics::new();
        
        // Record 3 successes and 1 failure
        for _ in 0..3 {
            let start = metrics.record_request_start();
            metrics.record_success(start);
        }
        let start = metrics.record_request_start();
        metrics.record_failure(start);
        
        assert_eq!(metrics.total_requests(), 4);
        assert_eq!(metrics.success_rate(), 0.75);
        assert_eq!(metrics.failure_rate(), 0.25);
    }

    #[test]
    fn test_metrics_retries() {
        let metrics = ApiMetrics::new();
        metrics.record_retry();
        metrics.record_retry();
        
        assert_eq!(metrics.total_retries(), 2);
    }

    #[test]
    fn test_metrics_circuit_breaker_rejections() {
        let metrics = ApiMetrics::new();
        metrics.record_circuit_breaker_rejection();
        metrics.record_circuit_breaker_rejection();
        
        assert_eq!(metrics.circuit_breaker_rejections(), 2);
    }

    #[test]
    fn test_metrics_snapshot() {
        let metrics = ApiMetrics::new();
        let start = metrics.record_request_start();
        metrics.record_success(start);
        
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_requests, 1);
        assert_eq!(snapshot.successful_requests, 1);
        assert_eq!(snapshot.success_rate, 1.0);
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = ApiMetrics::new();
        let start = metrics.record_request_start();
        metrics.record_success(start);
        metrics.record_retry();
        
        assert_eq!(metrics.total_requests(), 1);
        assert_eq!(metrics.total_retries(), 1);
        
        metrics.reset();
        assert_eq!(metrics.total_requests(), 0);
        assert_eq!(metrics.total_retries(), 0);
    }
}

