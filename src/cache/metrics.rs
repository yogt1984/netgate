use std::sync::atomic::{AtomicU64, Ordering};

/// Cache metrics for tracking cache performance
#[derive(Debug)]
pub struct CacheMetrics {
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    invalidations: AtomicU64,
    puts: AtomicU64,
}

impl CacheMetrics {
    pub fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            invalidations: AtomicU64::new(0),
            puts: AtomicU64::new(0),
        }
    }

    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_invalidation(&self) {
        self.invalidations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_put(&self) {
        self.puts.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> CacheMetricsSnapshot {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total_requests = hits + misses;
        let hit_rate = if total_requests > 0 {
            hits as f64 / total_requests as f64
        } else {
            0.0
        };

        CacheMetricsSnapshot {
            hits,
            misses,
            hit_rate,
            miss_rate: 1.0 - hit_rate,
            evictions: self.evictions.load(Ordering::Relaxed),
            invalidations: self.invalidations.load(Ordering::Relaxed),
            puts: self.puts.load(Ordering::Relaxed),
            total_requests,
        }
    }

    pub fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
        self.invalidations.store(0, Ordering::Relaxed);
        self.puts.store(0, Ordering::Relaxed);
    }
}

impl Default for CacheMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of cache metrics
#[derive(Debug, Clone)]
pub struct CacheMetricsSnapshot {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub miss_rate: f64,
    pub evictions: u64,
    pub invalidations: u64,
    pub puts: u64,
    pub total_requests: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_metrics() {
        let metrics = CacheMetrics::new();
        
        metrics.record_hit();
        metrics.record_hit();
        metrics.record_miss();
        metrics.record_put();
        metrics.record_eviction();
        metrics.record_invalidation();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.hits, 2);
        assert_eq!(snapshot.misses, 1);
        assert_eq!(snapshot.puts, 1);
        assert_eq!(snapshot.evictions, 1);
        assert_eq!(snapshot.invalidations, 1);
        assert_eq!(snapshot.total_requests, 3);
        assert!((snapshot.hit_rate - 2.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn test_cache_metrics_reset() {
        let metrics = CacheMetrics::new();
        metrics.record_hit();
        metrics.record_miss();
        
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_requests, 2);

        metrics.reset();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_requests, 0);
    }
}

