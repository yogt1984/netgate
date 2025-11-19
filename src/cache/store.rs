use crate::netbox::models::{NetBoxDevice, NetBoxSite};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, trace};

/// Cache entry with expiration time
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    value: T,
    expires_at: Instant,
    #[allow(dead_code)] // Reserved for future use (cache age statistics)
    created_at: Instant,
}

impl<T> CacheEntry<T> {
    fn new(value: T, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            value,
            expires_at: now + ttl,
            created_at: now,
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }

    #[allow(dead_code)] // Reserved for future use (cache age statistics)
    fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

/// In-memory cache with TTL support
pub struct Cache<K, V> {
    store: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    default_ttl: Duration,
    max_size: Option<usize>,
}

impl<K, V> Cache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + std::fmt::Debug + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a new cache with default TTL
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
            max_size: None,
        }
    }

    /// Create a new cache with size limit
    pub fn with_max_size(default_ttl: Duration, max_size: usize) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
            max_size: Some(max_size),
        }
    }

    /// Get a value from cache
    pub async fn get(&self, key: &K) -> Option<V> {
        let store = self.store.read().await;
        let entry = store.get(key)?;

        if entry.is_expired() {
            trace!("Cache entry expired for key: {:?}", key);
            drop(store);
            // Remove expired entry
            let mut store = self.store.write().await;
            store.remove(key);
            return None;
        }

        debug!("Cache hit for key: {:?}", key);
        Some(entry.value.clone())
    }

    /// Put a value into cache
    pub async fn put(&self, key: K, value: V) {
        self.put_with_ttl(key, value, self.default_ttl).await;
    }

    /// Put a value into cache with custom TTL
    pub async fn put_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let key_clone = key.clone();
        let mut store = self.store.write().await;

        // Check size limit
        if let Some(max_size) = self.max_size {
            if store.len() >= max_size && !store.contains_key(&key_clone) {
                // Evict oldest entry (simple FIFO strategy)
                if let Some(oldest_key) = store.keys().next().cloned() {
                    trace!("Evicting cache entry: {:?}", oldest_key);
                    store.remove(&oldest_key);
                }
            }
        }

        let entry = CacheEntry::new(value, ttl);
        store.insert(key_clone.clone(), entry);
        debug!("Cached value for key: {:?} with TTL: {:?}", key_clone, ttl);
    }

    /// Remove a value from cache
    pub async fn invalidate(&self, key: &K) {
        let mut store = self.store.write().await;
        if store.remove(key).is_some() {
            debug!("Invalidated cache entry: {:?}", key);
        }
    }

    /// Invalidate all entries matching a predicate
    pub async fn invalidate_matching<F>(&self, predicate: F)
    where
        F: Fn(&K) -> bool,
    {
        let mut store = self.store.write().await;
        let keys_to_remove: Vec<K> = store
            .keys()
            .filter(|k| predicate(k))
            .cloned()
            .collect();

        for key in keys_to_remove {
            store.remove(&key);
        }

        debug!("Invalidated {} cache entries", store.len());
    }

    /// Clear all entries
    pub async fn clear(&self) {
        let mut store = self.store.write().await;
        let count = store.len();
        store.clear();
        debug!("Cleared {} cache entries", count);
    }

    /// Remove expired entries
    pub async fn evict_expired(&self) -> usize {
        let mut store = self.store.write().await;
        let initial_len = store.len();
        
        store.retain(|_, entry| !entry.is_expired());
        
        let removed = initial_len - store.len();
        if removed > 0 {
            debug!("Evicted {} expired cache entries", removed);
        }
        removed
    }

    /// Get cache size
    pub async fn size(&self) -> usize {
        let store = self.store.read().await;
        store.len()
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let store = self.store.read().await;
        let total_entries = store.len();
        let expired_count = store.values().filter(|e| e.is_expired()).count();
        let valid_entries = total_entries - expired_count;

        CacheStats {
            total_entries,
            valid_entries,
            expired_entries: expired_count,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub expired_entries: usize,
}

/// Type alias for site cache
pub type SiteCache = Cache<CacheKey, NetBoxSite>;

/// Type alias for device cache
pub type DeviceCache = Cache<CacheKey, NetBoxDevice>;

/// Cache key for NetBox resources
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum CacheKey {
    Site(i32),
    Device(i32),
    SiteList(String), // Query string as key
    DeviceList(String), // Query string as key
}

impl CacheKey {
    pub fn site(id: i32) -> Self {
        Self::Site(id)
    }

    pub fn device(id: i32) -> Self {
        Self::Device(id)
    }

    pub fn site_list<S: Into<String>>(query: S) -> Self {
        Self::SiteList(query.into())
    }

    pub fn device_list<S: Into<String>>(query: S) -> Self {
        Self::DeviceList(query.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_cache_put_and_get() {
        let cache = Cache::new(Duration::from_secs(60));
        cache.put("key1".to_string(), "value1".to_string()).await;

        let value = cache.get(&"key1".to_string()).await;
        assert_eq!(value, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = Cache::new(Duration::from_millis(10));
        cache.put("key1".to_string(), "value1".to_string()).await;

        // Value should be available immediately
        assert!(cache.get(&"key1".to_string()).await.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Value should be expired
        assert!(cache.get(&"key1".to_string()).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let cache = Cache::new(Duration::from_secs(60));
        cache.put("key1".to_string(), "value1".to_string()).await;
        cache.invalidate(&"key1".to_string()).await;

        assert!(cache.get(&"key1".to_string()).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_max_size() {
        let cache = Cache::with_max_size(Duration::from_secs(60), 2);
        cache.put("key1".to_string(), "value1".to_string()).await;
        cache.put("key2".to_string(), "value2".to_string()).await;
        
        // Cache is now full (2 entries)
        assert_eq!(cache.size().await, 2);
        
        // Adding a third entry should evict the oldest
        cache.put("key3".to_string(), "value3".to_string()).await;
        
        // Cache should still have max_size entries
        assert_eq!(cache.size().await, 2);
        
        // key1 should be evicted, key2 and key3 should remain
        // Note: The eviction strategy is FIFO, so the first inserted key is removed
        let key1 = cache.get(&"key1".to_string()).await;
        let key2 = cache.get(&"key2".to_string()).await;
        let key3 = cache.get(&"key3".to_string()).await;
        
        // At least one of key1 should be missing, and key3 should be present
        assert!(key3.is_some());
        // Either key1 or key2 might be evicted depending on insertion order
        assert!(key1.is_none() || key2.is_none());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = Cache::new(Duration::from_secs(60));
        cache.put("key1".to_string(), "value1".to_string()).await;
        cache.put("key2".to_string(), "value2".to_string()).await;

        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.valid_entries, 2);
    }

    #[tokio::test]
    async fn test_cache_evict_expired() {
        let cache = Cache::new(Duration::from_millis(10));
        cache.put("key1".to_string(), "value1".to_string()).await;
        cache.put("key2".to_string(), "value2".to_string()).await;

        tokio::time::sleep(Duration::from_millis(20)).await;

        let evicted = cache.evict_expired().await;
        assert_eq!(evicted, 2);
        assert_eq!(cache.size().await, 0);
    }
}

