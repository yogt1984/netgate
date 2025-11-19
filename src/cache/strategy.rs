use crate::cache::CacheKey;
use std::time::Duration;

/// Cache invalidation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidationStrategy {
    /// Never invalidate (only TTL-based expiration)
    Never,
    /// Invalidate on write operations
    WriteThrough,
    /// Invalidate related entries on write
    WriteBack,
    /// Invalidate all entries of the same type
    TypeBased,
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub default_ttl: Duration,
    pub max_size: Option<usize>,
    pub invalidation_strategy: InvalidationStrategy,
    pub enable_metrics: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::from_secs(300), // 5 minutes
            max_size: Some(1000),
            invalidation_strategy: InvalidationStrategy::WriteThrough,
            enable_metrics: true,
        }
    }
}

impl CacheConfig {
    /// Create a new cache configuration
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            default_ttl,
            ..Default::default()
        }
    }

    /// Set maximum cache size
    pub fn with_max_size(mut self, max_size: usize) -> Self {
        self.max_size = Some(max_size);
        self
    }

    /// Set invalidation strategy
    pub fn with_invalidation_strategy(mut self, strategy: InvalidationStrategy) -> Self {
        self.invalidation_strategy = strategy;
        self
    }

    /// Enable or disable metrics
    pub fn with_metrics(mut self, enable: bool) -> Self {
        self.enable_metrics = enable;
        self
    }
}

/// Helper to determine which cache keys to invalidate
pub fn get_invalidation_keys(key: &CacheKey, strategy: InvalidationStrategy) -> Vec<CacheKey> {
    match strategy {
        InvalidationStrategy::Never => vec![],
        InvalidationStrategy::WriteThrough => vec![key.clone()],
        InvalidationStrategy::WriteBack => {
            // Invalidate the specific key and related list queries
            match key {
                CacheKey::Site(id) => {
                    vec![
                        CacheKey::Site(*id),
                        CacheKey::SiteList("*".to_string()), // Invalidate all site lists
                    ]
                }
                CacheKey::Device(id) => {
                    vec![
                        CacheKey::Device(*id),
                        CacheKey::DeviceList("*".to_string()), // Invalidate all device lists
                    ]
                }
                _ => vec![key.clone()],
            }
        }
        InvalidationStrategy::TypeBased => {
            // Invalidate all entries of the same type
            match key {
                CacheKey::Site(_) => vec![CacheKey::SiteList("*".to_string())],
                CacheKey::Device(_) => vec![CacheKey::DeviceList("*".to_string())],
                _ => vec![key.clone()],
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalidation_strategy_never() {
        let key = CacheKey::Site(1);
        let keys = get_invalidation_keys(&key, InvalidationStrategy::Never);
        assert!(keys.is_empty());
    }

    #[test]
    fn test_invalidation_strategy_write_through() {
        let key = CacheKey::Site(1);
        let keys = get_invalidation_keys(&key, InvalidationStrategy::WriteThrough);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], CacheKey::Site(1));
    }

    #[test]
    fn test_invalidation_strategy_write_back() {
        let key = CacheKey::Site(1);
        let keys = get_invalidation_keys(&key, InvalidationStrategy::WriteBack);
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&CacheKey::Site(1)));
        assert!(keys.contains(&CacheKey::SiteList("*".to_string())));
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.default_ttl, Duration::from_secs(300));
        assert_eq!(config.max_size, Some(1000));
        assert_eq!(config.invalidation_strategy, InvalidationStrategy::WriteThrough);
    }

    #[test]
    fn test_cache_config_builder() {
        let config = CacheConfig::new(Duration::from_secs(60))
            .with_max_size(500)
            .with_invalidation_strategy(InvalidationStrategy::WriteBack)
            .with_metrics(false);

        assert_eq!(config.default_ttl, Duration::from_secs(60));
        assert_eq!(config.max_size, Some(500));
        assert_eq!(config.invalidation_strategy, InvalidationStrategy::WriteBack);
        assert!(!config.enable_metrics);
    }
}

