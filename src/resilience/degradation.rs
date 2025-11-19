use crate::error::AppError;
use crate::netbox::models::{NetBoxDevice, NetBoxSite};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, warn};

/// Cache for graceful degradation
pub struct DegradationCache {
    sites: Arc<RwLock<HashMap<i32, CachedSite>>>,
    devices: Arc<RwLock<HashMap<i32, CachedDevice>>>,
    site_lists: Arc<RwLock<HashMap<String, CachedSiteList>>>,
    device_lists: Arc<RwLock<HashMap<String, CachedDeviceList>>>,
    ttl: std::time::Duration,
}

#[derive(Debug, Clone)]
struct CachedSite {
    site: NetBoxSite,
    cached_at: std::time::Instant,
}

#[derive(Debug, Clone)]
struct CachedDevice {
    device: NetBoxDevice,
    cached_at: std::time::Instant,
}

#[derive(Debug, Clone)]
struct CachedSiteList {
    sites: Vec<NetBoxSite>,
    cached_at: std::time::Instant,
}

#[derive(Debug, Clone)]
struct CachedDeviceList {
    devices: Vec<NetBoxDevice>,
    cached_at: std::time::Instant,
}

impl DegradationCache {
    pub fn new(ttl: std::time::Duration) -> Self {
        Self {
            sites: Arc::new(RwLock::new(HashMap::new())),
            devices: Arc::new(RwLock::new(HashMap::new())),
            site_lists: Arc::new(RwLock::new(HashMap::new())),
            device_lists: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    pub fn default() -> Self {
        Self::new(std::time::Duration::from_secs(300)) // 5 minutes default TTL
    }

    /// Get cached site if available and not expired
    pub fn get_site(&self, id: i32) -> Option<NetBoxSite> {
        let sites = self.sites.read().unwrap();
        if let Some(cached) = sites.get(&id) {
            if cached.cached_at.elapsed() < self.ttl {
                debug!("Returning cached site {}", id);
                return Some(cached.site.clone());
            }
        }
        None
    }

    /// Cache a site
    pub fn cache_site(&self, id: i32, site: NetBoxSite) {
        let mut sites = self.sites.write().unwrap();
        sites.insert(id, CachedSite {
            site,
            cached_at: std::time::Instant::now(),
        });
    }

    /// Get cached device if available and not expired
    pub fn get_device(&self, id: i32) -> Option<NetBoxDevice> {
        let devices = self.devices.read().unwrap();
        if let Some(cached) = devices.get(&id) {
            if cached.cached_at.elapsed() < self.ttl {
                debug!("Returning cached device {}", id);
                return Some(cached.device.clone());
            }
        }
        None
    }

    /// Cache a device
    pub fn cache_device(&self, id: i32, device: NetBoxDevice) {
        let mut devices = self.devices.write().unwrap();
        devices.insert(id, CachedDevice {
            device,
            cached_at: std::time::Instant::now(),
        });
    }

    /// Get cached site list if available and not expired
    pub fn get_site_list(&self, key: &str) -> Option<Vec<NetBoxSite>> {
        let lists = self.site_lists.read().unwrap();
        if let Some(cached) = lists.get(key) {
            if cached.cached_at.elapsed() < self.ttl {
                debug!("Returning cached site list for key: {}", key);
                return Some(cached.sites.clone());
            }
        }
        None
    }

    /// Cache a site list
    pub fn cache_site_list(&self, key: String, sites: Vec<NetBoxSite>) {
        let mut lists = self.site_lists.write().unwrap();
        lists.insert(key, CachedSiteList {
            sites,
            cached_at: std::time::Instant::now(),
        });
    }

    /// Get cached device list if available and not expired
    pub fn get_device_list(&self, key: &str) -> Option<Vec<NetBoxDevice>> {
        let lists = self.device_lists.read().unwrap();
        if let Some(cached) = lists.get(key) {
            if cached.cached_at.elapsed() < self.ttl {
                debug!("Returning cached device list for key: {}", key);
                return Some(cached.devices.clone());
            }
        }
        None
    }

    /// Cache a device list
    pub fn cache_device_list(&self, key: String, devices: Vec<NetBoxDevice>) {
        let mut lists = self.device_lists.write().unwrap();
        lists.insert(key, CachedDeviceList {
            devices,
            cached_at: std::time::Instant::now(),
        });
    }

    /// Clear expired entries
    pub fn clear_expired(&self) {
        let now = std::time::Instant::now();
        
        // Clear expired sites
        {
            let mut sites = self.sites.write().unwrap();
            sites.retain(|_, cached| now.duration_since(cached.cached_at) < self.ttl);
        }
        
        // Clear expired devices
        {
            let mut devices = self.devices.write().unwrap();
            devices.retain(|_, cached| now.duration_since(cached.cached_at) < self.ttl);
        }
        
        // Clear expired site lists
        {
            let mut lists = self.site_lists.write().unwrap();
            lists.retain(|_, cached| now.duration_since(cached.cached_at) < self.ttl);
        }
        
        // Clear expired device lists
        {
            let mut lists = self.device_lists.write().unwrap();
            lists.retain(|_, cached| now.duration_since(cached.cached_at) < self.ttl);
        }
    }

    /// Clear all cache
    pub fn clear_all(&self) {
        self.sites.write().unwrap().clear();
        self.devices.write().unwrap().clear();
        self.site_lists.write().unwrap().clear();
        self.device_lists.write().unwrap().clear();
    }
}

/// Graceful degradation strategies
pub enum DegradationStrategy {
    /// Return cached data if available
    UseCache,
    /// Return empty result
    ReturnEmpty,
    /// Return error
    ReturnError,
    /// Return partial data if available
    ReturnPartial,
}

/// Apply graceful degradation for site retrieval
pub fn degrade_site_retrieval(
    cache: &DegradationCache,
    site_id: i32,
    strategy: DegradationStrategy,
) -> Result<Option<NetBoxSite>, AppError> {
    match strategy {
        DegradationStrategy::UseCache => {
            if let Some(cached_site) = cache.get_site(site_id) {
                warn!("Using cached site {} due to service degradation", site_id);
                return Ok(Some(cached_site));
            }
            Ok(None)
        }
        DegradationStrategy::ReturnEmpty => {
            warn!("Returning empty result for site {} due to service degradation", site_id);
            Ok(None)
        }
        DegradationStrategy::ReturnError => {
            Err(AppError::Internal(anyhow::anyhow!("Service unavailable")))
        }
        DegradationStrategy::ReturnPartial => {
            if let Some(cached_site) = cache.get_site(site_id) {
                warn!("Returning cached site {} as partial data", site_id);
                return Ok(Some(cached_site));
            }
            Ok(None)
        }
    }
}

/// Apply graceful degradation for site list retrieval
pub fn degrade_site_list_retrieval(
    cache: &DegradationCache,
    cache_key: &str,
    strategy: DegradationStrategy,
) -> Result<Vec<NetBoxSite>, AppError> {
    match strategy {
        DegradationStrategy::UseCache => {
            if let Some(cached_sites) = cache.get_site_list(cache_key) {
                warn!("Using cached site list for key {} due to service degradation", cache_key);
                return Ok(cached_sites);
            }
            Ok(vec![])
        }
        DegradationStrategy::ReturnEmpty => {
            warn!("Returning empty site list due to service degradation");
            Ok(vec![])
        }
        DegradationStrategy::ReturnError => {
            Err(AppError::Internal(anyhow::anyhow!("Service unavailable")))
        }
        DegradationStrategy::ReturnPartial => {
            if let Some(cached_sites) = cache.get_site_list(cache_key) {
                warn!("Returning cached site list as partial data");
                return Ok(cached_sites);
            }
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netbox::models::{SiteStatus, DeviceStatus};
    use std::time::Duration;

    fn create_test_site(id: i32) -> NetBoxSite {
        NetBoxSite {
            id: Some(id),
            name: format!("Site {}", id),
            status: Some(SiteStatus::Active),
            ..Default::default()
        }
    }

    fn create_test_device(id: i32) -> NetBoxDevice {
        NetBoxDevice {
            id: Some(id),
            name: Some(format!("Device {}", id)),
            status: Some(DeviceStatus::Active),
            ..Default::default()
        }
    }

    #[test]
    fn test_cache_site_and_retrieve() {
        let cache = DegradationCache::new(Duration::from_secs(60));
        let site = create_test_site(1);
        
        cache.cache_site(1, site.clone());
        let retrieved = cache.get_site(1);
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, Some(1));
    }

    #[test]
    fn test_cache_expires() {
        let cache = DegradationCache::new(Duration::from_millis(10));
        let site = create_test_site(1);
        
        cache.cache_site(1, site);
        std::thread::sleep(Duration::from_millis(20));
        
        let retrieved = cache.get_site(1);
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_cache_device_and_retrieve() {
        let cache = DegradationCache::new(Duration::from_secs(60));
        let device = create_test_device(1);
        
        cache.cache_device(1, device.clone());
        let retrieved = cache.get_device(1);
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, Some(1));
    }

    #[test]
    fn test_cache_site_list() {
        let cache = DegradationCache::new(Duration::from_secs(60));
        let sites = vec![create_test_site(1), create_test_site(2)];
        
        cache.cache_site_list("key1".to_string(), sites.clone());
        let retrieved = cache.get_site_list("key1");
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().len(), 2);
    }

    #[test]
    fn test_degrade_site_retrieval_use_cache() {
        let cache = DegradationCache::new(Duration::from_secs(60));
        let site = create_test_site(1);
        cache.cache_site(1, site.clone());
        
        let result = degrade_site_retrieval(&cache, 1, DegradationStrategy::UseCache).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_degrade_site_retrieval_return_empty() {
        let cache = DegradationCache::new(Duration::from_secs(60));
        let result = degrade_site_retrieval(&cache, 1, DegradationStrategy::ReturnEmpty).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_degrade_site_list_retrieval_use_cache() {
        let cache = DegradationCache::new(Duration::from_secs(60));
        let sites = vec![create_test_site(1), create_test_site(2)];
        cache.cache_site_list("key1".to_string(), sites);
        
        let result = degrade_site_list_retrieval(&cache, "key1", DegradationStrategy::UseCache).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_clear_expired() {
        let cache = DegradationCache::new(Duration::from_millis(10));
        cache.cache_site(1, create_test_site(1));
        std::thread::sleep(Duration::from_millis(20));
        cache.clear_expired();
        
        assert!(cache.get_site(1).is_none());
    }

    #[test]
    fn test_clear_all() {
        let cache = DegradationCache::new(Duration::from_secs(60));
        cache.cache_site(1, create_test_site(1));
        cache.cache_device(1, create_test_device(1));
        
        cache.clear_all();
        
        assert!(cache.get_site(1).is_none());
        assert!(cache.get_device(1).is_none());
    }
}

