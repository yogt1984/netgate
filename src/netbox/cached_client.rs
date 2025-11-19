use crate::cache::{Cache, CacheConfig, CacheKey, CacheMetrics};
use crate::error::AppError;
use crate::netbox::models::*;
use crate::netbox::ResilientNetBoxClient;
use std::sync::Arc;
use tracing::{debug, trace};

/// Cached NetBox client that wraps ResilientNetBoxClient with caching
pub struct CachedNetBoxClient {
    client: Arc<ResilientNetBoxClient>,
    site_cache: Arc<Cache<CacheKey, NetBoxSite>>,
    site_list_cache: Arc<Cache<CacheKey, Vec<NetBoxSite>>>,
    metrics: Arc<CacheMetrics>,
    config: CacheConfig,
}

impl CachedNetBoxClient {
    /// Create a new cached client with default configuration
    pub fn new(client: Arc<ResilientNetBoxClient>) -> Self {
        Self::with_config(client, CacheConfig::default())
    }

    /// Create a new cached client with custom configuration
    pub fn with_config(client: Arc<ResilientNetBoxClient>, config: CacheConfig) -> Self {
        let site_cache = Arc::new(if let Some(max_size) = config.max_size {
            Cache::with_max_size(config.default_ttl, max_size)
        } else {
            Cache::new(config.default_ttl)
        });

        let site_list_cache = Arc::new(if let Some(max_size) = config.max_size {
            Cache::with_max_size(config.default_ttl, max_size)
        } else {
            Cache::new(config.default_ttl)
        });

        Self {
            client,
            site_cache,
            site_list_cache,
            metrics: Arc::new(CacheMetrics::new()),
            config,
        }
    }

    /// Get a site with caching
    pub async fn get_site(&self, id: i32) -> Result<NetBoxSite, AppError> {
        let key = CacheKey::site(id);

        // Try cache first
        if let Some(cached) = self.site_cache.get(&key).await {
            if self.config.enable_metrics {
                self.metrics.record_hit();
            }
            trace!("Cache hit for site {}", id);
            return Ok(cached);
        }

        // Cache miss - fetch from NetBox
        if self.config.enable_metrics {
            self.metrics.record_miss();
        }
        trace!("Cache miss for site {}", id);

        let site = self.client.get_site(id).await?;

        // Store in cache
        self.site_cache.put(key, site.clone()).await;
        if self.config.enable_metrics {
            self.metrics.record_put();
        }

        Ok(site)
    }

    /// List sites with caching
    pub async fn list_sites(
        &self,
        tenant_id: Option<i32>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<NetBoxResponse<NetBoxSite>, AppError> {
        // Create cache key from query parameters
        let query_key = format!(
            "tenant={:?}&limit={:?}&offset={:?}",
            tenant_id, limit, offset
        );
        let key = CacheKey::site_list(query_key.clone());

        // Try cache first
        if let Some(cached) = self.site_list_cache.get(&key).await {
            if self.config.enable_metrics {
                self.metrics.record_hit();
            }
            trace!("Cache hit for site list: {}", query_key);
            return Ok(NetBoxResponse {
                count: Some(cached.len() as i32),
                next: None,
                previous: None,
                results: Some(cached),
            });
        }

        // Cache miss - fetch from NetBox
        if self.config.enable_metrics {
            self.metrics.record_miss();
        }
        trace!("Cache miss for site list: {}", query_key);

        let response = self.client.list_sites(tenant_id, limit, offset).await?;

        // Store in cache if we have results
        if let Some(ref sites) = response.results {
            self.site_list_cache.put(key, sites.clone()).await;
            if self.config.enable_metrics {
                self.metrics.record_put();
            }
        }

        Ok(response)
    }

    /// Create a site and invalidate cache
    pub async fn create_site(&self, request: CreateSiteRequest) -> Result<NetBoxSite, AppError> {
        let site = self.client.create_site(request).await?;

        // Invalidate cache based on strategy
        self.invalidate_site_cache(&site.id).await;

        Ok(site)
    }

    /// Invalidate site cache based on strategy
    async fn invalidate_site_cache(&self, site_id: &Option<i32>) {
        if let Some(id) = site_id {
            let keys = crate::cache::strategy::get_invalidation_keys(
                &CacheKey::site(*id),
                self.config.invalidation_strategy,
            );

            for key in keys {
                match key {
                    CacheKey::Site(id) => {
                        self.site_cache.invalidate(&CacheKey::site(id)).await;
                    }
                    CacheKey::SiteList(_) => {
                        self.invalidate_site_list_cache().await;
                    }
                    _ => {}
                }
            }

            if self.config.enable_metrics {
                self.metrics.record_invalidation();
            }
        }
    }

    /// Invalidate all site list cache entries
    async fn invalidate_site_list_cache(&self) {
        self.site_list_cache
            .invalidate_matching(|k| matches!(k, CacheKey::SiteList(_)))
            .await;
    }

    /// Get cache metrics
    pub fn cache_metrics(&self) -> crate::cache::CacheMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheClientStats {
        CacheClientStats {
            site_cache: self.site_cache.stats().await,
            site_list_cache: self.site_list_cache.stats().await,
            metrics: self.metrics.snapshot(),
        }
    }

    /// Clear all caches
    pub async fn clear_all_caches(&self) {
        self.site_cache.clear().await;
        self.site_list_cache.clear().await;
        debug!("Cleared all caches");
    }

    /// Evict expired entries from all caches
    pub async fn evict_expired(&self) -> usize {
        let mut total = 0;
        total += self.site_cache.evict_expired().await;
        total += self.site_list_cache.evict_expired().await;
        total
    }
}

/// Cache statistics for the cached client
#[derive(Debug, Clone)]
pub struct CacheClientStats {
    pub site_cache: crate::cache::CacheStats,
    pub site_list_cache: crate::cache::CacheStats,
    pub metrics: crate::cache::CacheMetricsSnapshot,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::netbox::client::NetBoxClient;
    use crate::cache::InvalidationStrategy;
    use serde_json::json;
    use std::time::Duration;
    use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};

    fn create_test_client(uri: String) -> Arc<ResilientNetBoxClient> {
        let config = Config {
            port: 8080,
            netbox_url: uri,
            netbox_token: "test-token".to_string(),
        };
        let client = Arc::new(NetBoxClient::new(config).unwrap());
        Arc::new(ResilientNetBoxClient::new(client))
    }

    #[test]
    fn test_cached_client_creation() {
        let client = create_test_client("http://localhost:8000".to_string());
        let _cached = CachedNetBoxClient::new(client);
        // Just verify it compiles
        assert!(true);
    }

    #[test]
    fn test_cached_client_with_config() {
        let client = create_test_client("http://localhost:8000".to_string());
        let config = CacheConfig::new(Duration::from_secs(60))
            .with_max_size(100)
            .with_invalidation_strategy(InvalidationStrategy::WriteBack);
        let _cached = CachedNetBoxClient::with_config(client, config);
        // Just verify it compiles
        assert!(true);
    }

    #[tokio::test]
    async fn test_cached_get_site_hit() {
        let mock_server = MockServer::start().await;
        let client = create_test_client(mock_server.uri());
        let cached = CachedNetBoxClient::new(client.clone());

        let site_response = json!({
            "id": 1,
            "name": "Test Site",
            "status": "active"
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&site_response))
            .mount(&mock_server)
            .await;

        // First call - cache miss
        let result1 = cached.get_site(1).await;
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap().name, "Test Site");

        // Second call - should be cache hit (no HTTP request)
        let result2 = cached.get_site(1).await;
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap().name, "Test Site");

        // Verify metrics
        let metrics = cached.cache_metrics();
        assert_eq!(metrics.hits, 1);
        assert_eq!(metrics.misses, 1);
        assert_eq!(metrics.puts, 1);
    }

    #[tokio::test]
    async fn test_cached_list_sites() {
        let mock_server = MockServer::start().await;
        let client = create_test_client(mock_server.uri());
        let cached = CachedNetBoxClient::new(client.clone());

        let sites_response = json!({
            "count": 2,
            "results": [
                {"id": 1, "name": "Site 1"},
                {"id": 2, "name": "Site 2"}
            ]
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&sites_response))
            .mount(&mock_server)
            .await;

        // First call - cache miss
        let result1 = cached.list_sites(None, Some(10), None).await;
        assert!(result1.is_ok());
        let response1 = result1.unwrap();
        assert_eq!(response1.results.as_ref().unwrap().len(), 2);

        // Second call - should be cache hit
        let result2 = cached.list_sites(None, Some(10), None).await;
        assert!(result2.is_ok());

        let metrics = cached.cache_metrics();
        assert_eq!(metrics.hits, 1);
        assert_eq!(metrics.misses, 1);
    }

    #[tokio::test]
    async fn test_cached_create_site_invalidation() {
        let mock_server = MockServer::start().await;
        let client = create_test_client(mock_server.uri());
        let cached = CachedNetBoxClient::new(client.clone());

        // First, get a site and cache it
        let site_response = json!({
            "id": 1,
            "name": "Test Site",
            "status": "active"
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&site_response))
            .mount(&mock_server)
            .await;

        let _ = cached.get_site(1).await;

        // Create a new site - should invalidate cache
        let create_response = json!({
            "id": 2,
            "name": "New Site",
            "status": "active"
        });

        Mock::given(method("POST"))
            .and(path("/api/dcim/sites/"))
            .respond_with(ResponseTemplate::new(201).set_body_json(&create_response))
            .mount(&mock_server)
            .await;

        let create_request = CreateSiteRequest {
            name: "New Site".to_string(),
            slug: None,
            description: None,
            status: None,
            region: None,
            tenant: None,
            facility: None,
            physical_address: None,
            shipping_address: None,
            latitude: None,
            longitude: None,
            contact_name: None,
            contact_phone: None,
            contact_email: None,
            comments: None,
            tags: None,
        };

        let result = cached.create_site(create_request).await;
        assert!(result.is_ok());

        let metrics = cached.cache_metrics();
        assert!(metrics.invalidations > 0);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let mock_server = MockServer::start().await;
        let client = create_test_client(mock_server.uri());
        let cached = CachedNetBoxClient::new(client.clone());

        let site_response = json!({
            "id": 1,
            "name": "Test Site",
            "status": "active"
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&site_response))
            .mount(&mock_server)
            .await;

        let _ = cached.get_site(1).await;

        let stats = cached.cache_stats().await;
        assert!(stats.site_cache.total_entries > 0);
        assert!(stats.metrics.total_requests > 0);
    }

    #[tokio::test]
    async fn test_cache_evict_expired() {
        let mock_server = MockServer::start().await;
        let client = create_test_client(mock_server.uri());
        let config = CacheConfig::new(Duration::from_millis(10));
        let cached = CachedNetBoxClient::with_config(client.clone(), config);

        let site_response = json!({
            "id": 1,
            "name": "Test Site",
            "status": "active"
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&site_response))
            .mount(&mock_server)
            .await;

        let _ = cached.get_site(1).await;

        tokio::time::sleep(Duration::from_millis(20)).await;

        let evicted = cached.evict_expired().await;
        assert!(evicted > 0);
    }
}

