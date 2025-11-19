use crate::error::AppError;
use crate::netbox::client::NetBoxClient;
use crate::netbox::models::*;
use crate::resilience::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use crate::resilience::degradation::DegradationCache;
use crate::resilience::metrics::ApiMetrics;
use crate::resilience::retry::{RetryConfig, retry_with_backoff};
use std::sync::Arc;
use tracing::warn;

/// Resilient NetBox client with retry, circuit breaker, metrics, and graceful degradation
pub struct ResilientNetBoxClient {
    client: Arc<NetBoxClient>,
    circuit_breaker: Arc<CircuitBreaker>,
    metrics: Arc<ApiMetrics>,
    cache: Arc<DegradationCache>,
    retry_config: RetryConfig,
}

impl ResilientNetBoxClient {
    /// Create a new resilient client with default configuration
    pub fn new(client: Arc<NetBoxClient>) -> Self {
        Self {
            client,
            circuit_breaker: Arc::new(CircuitBreaker::new()),
            metrics: Arc::new(ApiMetrics::new()),
            cache: Arc::new(DegradationCache::default()),
            retry_config: RetryConfig::default(),
        }
    }

    /// Create a new resilient client with custom configuration
    pub fn with_config(
        client: Arc<NetBoxClient>,
        circuit_breaker_config: CircuitBreakerConfig,
        retry_config: RetryConfig,
        cache_ttl: std::time::Duration,
    ) -> Self {
        Self {
            client,
            circuit_breaker: Arc::new(CircuitBreaker::with_config(circuit_breaker_config)),
            metrics: Arc::new(ApiMetrics::new()),
            cache: Arc::new(DegradationCache::new(cache_ttl)),
            retry_config,
        }
    }

    /// Get a site with resilience features
    pub async fn get_site(&self, id: i32) -> Result<NetBoxSite, AppError> {
        // Check circuit breaker
        if !self.circuit_breaker.allow_request() {
            self.metrics.record_circuit_breaker_rejection();
            warn!("Circuit breaker is open, attempting graceful degradation for site {}", id);
            
            // Try graceful degradation
            if let Some(cached_site) = self.cache.get_site(id) {
                return Ok(cached_site);
            }
            return Err(AppError::Internal(anyhow::anyhow!("Service unavailable (circuit breaker open)")));
        }

        let start_time = self.metrics.record_request_start();

        // Execute with retry
        let result = retry_with_backoff(&self.retry_config, || {
            let client = Arc::clone(&self.client);
            let id = id;
            Box::pin(async move {
                client.get_site(id).await
            })
        }).await;

        match result {
            Ok(site) => {
                self.circuit_breaker.record_success();
                self.metrics.record_success(start_time);
                // Cache the result
                if let Some(site_id) = site.id {
                    self.cache.cache_site(site_id, site.clone());
                }
                Ok(site)
            }
            Err(e) => {
                self.circuit_breaker.record_failure();
                self.metrics.record_failure(start_time);
                
                // Try graceful degradation
                if let Some(cached_site) = self.cache.get_site(id) {
                    warn!("Using cached site {} due to error: {}", id, e);
                    return Ok(cached_site);
                }
                
                Err(AppError::Internal(anyhow::Error::from(e)))
            }
        }
    }

    /// List sites with resilience features
    pub async fn list_sites(
        &self,
        tenant_id: Option<i32>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<NetBoxResponse<NetBoxSite>, AppError> {
        // Check circuit breaker
        if !self.circuit_breaker.allow_request() {
            self.metrics.record_circuit_breaker_rejection();
            warn!("Circuit breaker is open, attempting graceful degradation for site list");
            
            // Try graceful degradation
            let cache_key = format!("sites:tenant:{}:limit:{}:offset:{}", 
                tenant_id.unwrap_or(0), limit.unwrap_or(0), offset.unwrap_or(0));
            if let Some(cached_sites) = self.cache.get_site_list(&cache_key) {
                return Ok(NetBoxResponse {
                    count: Some(cached_sites.len() as i32),
                    next: None,
                    previous: None,
                    results: Some(cached_sites),
                });
            }
            return Err(AppError::Internal(anyhow::anyhow!("Service unavailable (circuit breaker open)")));
        }

        let start_time = self.metrics.record_request_start();

        // Execute with retry
        let result = retry_with_backoff(&self.retry_config, || {
            let client = Arc::clone(&self.client);
            let tenant_id = tenant_id;
            let limit = limit;
            let offset = offset;
            Box::pin(async move {
                client.list_sites(tenant_id, limit, offset).await
            })
        }).await;

        match result {
            Ok(response) => {
                self.circuit_breaker.record_success();
                self.metrics.record_success(start_time);
                
                // Cache the result
                if let Some(ref sites) = response.results {
                    let cache_key = format!("sites:tenant:{}:limit:{}:offset:{}", 
                        tenant_id.unwrap_or(0), limit.unwrap_or(0), offset.unwrap_or(0));
                    self.cache.cache_site_list(cache_key, sites.clone());
                }
                
                Ok(response)
            }
            Err(e) => {
                self.circuit_breaker.record_failure();
                self.metrics.record_failure(start_time);
                
                // Try graceful degradation
                let cache_key = format!("sites:tenant:{}:limit:{}:offset:{}", 
                    tenant_id.unwrap_or(0), limit.unwrap_or(0), offset.unwrap_or(0));
                if let Some(cached_sites) = self.cache.get_site_list(&cache_key) {
                    warn!("Using cached site list due to error: {}", e);
                    return Ok(NetBoxResponse {
                        count: Some(cached_sites.len() as i32),
                        next: None,
                        previous: None,
                        results: Some(cached_sites),
                    });
                }
                
                Err(AppError::Internal(anyhow::Error::from(e)))
            }
        }
    }

    /// Create a site with resilience features
    pub async fn create_site(&self, request: CreateSiteRequest) -> Result<NetBoxSite, AppError> {
        // Check circuit breaker
        if !self.circuit_breaker.allow_request() {
            self.metrics.record_circuit_breaker_rejection();
            return Err(AppError::Internal(anyhow::anyhow!("Service unavailable (circuit breaker open)")));
        }

        let start_time = self.metrics.record_request_start();

        // Execute with retry
        let result = retry_with_backoff(&self.retry_config, || {
            let client = Arc::clone(&self.client);
            let request = request.clone();
            Box::pin(async move {
                client.create_site(request).await
            })
        }).await;

        match result {
            Ok(site) => {
                self.circuit_breaker.record_success();
                self.metrics.record_success(start_time);
                // Cache the result
                if let Some(site_id) = site.id {
                    self.cache.cache_site(site_id, site.clone());
                }
                Ok(site)
            }
            Err(e) => {
                self.circuit_breaker.record_failure();
                self.metrics.record_failure(start_time);
                Err(AppError::Internal(anyhow::Error::from(e)))
            }
        }
    }

    /// Get metrics snapshot
    pub fn metrics(&self) -> crate::resilience::MetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Get circuit breaker state
    pub fn circuit_breaker_state(&self) -> crate::resilience::CircuitState {
        self.circuit_breaker.state()
    }

    /// Clear cache
    pub fn clear_cache(&self) {
        self.cache.clear_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use serde_json::json;
    use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};

    fn create_test_config(base_url: String, token: String) -> Config {
        Config {
            port: 8080,
            netbox_url: base_url,
            netbox_token: token,
        }
    }

    #[tokio::test]
    async fn test_resilient_client_get_site_success() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = Arc::new(NetBoxClient::new(config).unwrap());
        let resilient_client = ResilientNetBoxClient::new(client);

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

        let result = resilient_client.get_site(1).await;
        assert!(result.is_ok());
        
        let metrics = resilient_client.metrics();
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.success_rate, 1.0);
    }

    #[tokio::test]
    async fn test_resilient_client_retries_on_failure() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = Arc::new(NetBoxClient::new(config).unwrap());
        
        let retry_config = RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 10,
            max_delay_ms: 100,
            backoff_multiplier: 2.0,
            use_jitter: false,
        };
        let resilient_client = ResilientNetBoxClient::with_config(
            client,
            CircuitBreakerConfig::default(),
            retry_config,
            std::time::Duration::from_secs(60),
        );

        // First two calls fail, third succeeds
        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 1,
                "name": "Test Site",
                "status": "active"
            })))
            .mount(&mock_server)
            .await;

        let result = resilient_client.get_site(1).await;
        assert!(result.is_ok());
        
        // Verify that the operation succeeded after retries
        // The retry logic handles failures internally, so we just verify success
        let metrics = resilient_client.metrics();
        assert_eq!(metrics.total_requests, 1); // One logical request (with internal retries)
        assert_eq!(metrics.successful_requests, 1);
    }

    #[tokio::test]
    async fn test_resilient_client_circuit_breaker_opens() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = Arc::new(NetBoxClient::new(config).unwrap());
        
        let cb_config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 1,
            timeout_duration: std::time::Duration::from_secs(60),
            window_duration: std::time::Duration::from_secs(60),
        };
        let resilient_client = ResilientNetBoxClient::with_config(
            client,
            cb_config,
            RetryConfig::default(),
            std::time::Duration::from_secs(60),
        );

        // Fail twice to open circuit breaker
        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let _ = resilient_client.get_site(1).await;
        let _ = resilient_client.get_site(1).await;

        // Circuit breaker should be open now
        assert_eq!(resilient_client.circuit_breaker_state(), crate::resilience::CircuitState::Open);
        
        // Next request should be rejected
        let result = resilient_client.get_site(1).await;
        assert!(result.is_err());
        
        let metrics = resilient_client.metrics();
        assert!(metrics.circuit_breaker_rejections > 0);
    }

    #[tokio::test]
    async fn test_resilient_client_uses_cache_on_failure() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = Arc::new(NetBoxClient::new(config).unwrap());
        let resilient_client = ResilientNetBoxClient::new(client);

        // First call succeeds and caches
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

        let result1 = resilient_client.get_site(1).await;
        assert!(result1.is_ok());

        // Second call fails but should use cache
        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let result2 = resilient_client.get_site(1).await;
        assert!(result2.is_ok()); // Should return cached value
    }
}

