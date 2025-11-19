use poem_openapi::{payload::Json, ApiResponse, OpenApi};
use std::sync::Arc;

use crate::netbox::ResilientNetBoxClient;

pub struct MetricsApi {
    netbox_client: Option<Arc<ResilientNetBoxClient>>,
}

impl MetricsApi {
    pub fn new() -> Self {
        Self {
            netbox_client: None,
        }
    }

    pub fn with_netbox_client(netbox_client: Arc<ResilientNetBoxClient>) -> Self {
        Self {
            netbox_client: Some(netbox_client),
        }
    }
}

impl Default for MetricsApi {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, poem_openapi::Object)]
pub struct MetricsResponse {
    pub netbox: Option<NetBoxMetrics>,
    pub timestamp: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, poem_openapi::Object)]
pub struct NetBoxMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub success_rate: f64,
    pub failure_rate: f64,
    pub average_response_time_ms: f64,
    pub total_retries: u64,
    pub circuit_breaker_rejections: u64,
    pub circuit_breaker_state: String,
}

#[derive(ApiResponse)]
pub enum GetMetricsResponse {
    #[oai(status = 200)]
    Ok(Json<MetricsResponse>),
}

#[OpenApi]
impl MetricsApi {
    /// Get metrics for monitoring and observability
    /// 
    /// Returns performance metrics including:
    /// - Request counts and rates
    /// - Response times
    /// - Retry statistics
    /// - Circuit breaker state
    #[oai(path = "/metrics", method = "get")]
    async fn get_metrics(&self) -> GetMetricsResponse {
        let mut response = MetricsResponse {
            netbox: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        if let Some(ref client) = self.netbox_client {
            let metrics_snapshot = client.metrics();
            let cb_state = client.circuit_breaker_state();

            response.netbox = Some(NetBoxMetrics {
                total_requests: metrics_snapshot.total_requests,
                successful_requests: metrics_snapshot.successful_requests,
                failed_requests: metrics_snapshot.failed_requests,
                success_rate: metrics_snapshot.success_rate,
                failure_rate: metrics_snapshot.failure_rate,
                average_response_time_ms: metrics_snapshot.average_response_time_ms,
                total_retries: metrics_snapshot.total_retries,
                circuit_breaker_rejections: metrics_snapshot.circuit_breaker_rejections,
                circuit_breaker_state: format!("{:?}", cb_state),
            });
        }

        GetMetricsResponse::Ok(Json(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::netbox::client::NetBoxClient;
    use serde_json::json;
    use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_metrics_api_creation() {
        let api = MetricsApi::new();
        // Just verify it compiles
        assert!(true);
    }

    #[test]
    fn test_metrics_api_with_client() {
        let config = Config {
            port: 8080,
            netbox_url: "http://localhost:8000".to_string(),
            netbox_token: "test-token".to_string(),
        };
        let netbox_client = Arc::new(NetBoxClient::new(config).unwrap());
        let resilient_client = Arc::new(ResilientNetBoxClient::new(netbox_client));
        let api = MetricsApi::with_netbox_client(resilient_client);
        // Just verify it compiles
        assert!(true);
    }

    #[tokio::test]
    async fn test_get_metrics() {
        let mock_server = MockServer::start().await;
        let config = Config {
            port: 8080,
            netbox_url: mock_server.uri(),
            netbox_token: "test-token".to_string(),
        };
        let netbox_client = Arc::new(NetBoxClient::new(config).unwrap());
        let resilient_client = Arc::new(ResilientNetBoxClient::new(netbox_client));
        let api = MetricsApi::with_netbox_client(resilient_client.clone());

        // Make some requests to generate metrics
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

        // Make a request
        let _ = resilient_client.get_site(1).await;

        let result = api.get_metrics().await;
        match result {
            GetMetricsResponse::Ok(Json(metrics)) => {
                assert!(metrics.netbox.is_some());
                let netbox_metrics = metrics.netbox.unwrap();
                assert!(netbox_metrics.total_requests > 0);
                assert!(netbox_metrics.successful_requests > 0);
                assert!(netbox_metrics.success_rate > 0.0);
                assert!(netbox_metrics.average_response_time_ms >= 0.0);
            }
        }
    }

    #[tokio::test]
    async fn test_get_metrics_with_failures() {
        let mock_server = MockServer::start().await;
        let config = Config {
            port: 8080,
            netbox_url: mock_server.uri(),
            netbox_token: "test-token".to_string(),
        };
        let netbox_client = Arc::new(NetBoxClient::new(config).unwrap());
        let resilient_client = Arc::new(ResilientNetBoxClient::new(netbox_client));
        let api = MetricsApi::with_netbox_client(resilient_client.clone());

        // Mock failing requests
        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        // Make a failing request
        let _ = resilient_client.get_site(1).await;

        let result = api.get_metrics().await;
        match result {
            GetMetricsResponse::Ok(Json(metrics)) => {
                assert!(metrics.netbox.is_some());
                let netbox_metrics = metrics.netbox.unwrap();
                assert!(netbox_metrics.total_requests > 0);
                assert!(netbox_metrics.failed_requests > 0);
                assert!(netbox_metrics.failure_rate > 0.0);
            }
        }
    }

    #[tokio::test]
    async fn test_get_metrics_circuit_breaker_rejections() {
        let mock_server = MockServer::start().await;
        let config = Config {
            port: 8080,
            netbox_url: mock_server.uri(),
            netbox_token: "test-token".to_string(),
        };
        let netbox_client = Arc::new(NetBoxClient::new(config).unwrap());
        let resilient_client = Arc::new(ResilientNetBoxClient::new(netbox_client));
        let api = MetricsApi::with_netbox_client(resilient_client.clone());

        // Mock failing requests to open circuit breaker
        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        // Make enough failed requests to open circuit breaker
        for _ in 0..6 {
            let _ = resilient_client.get_site(1).await;
        }

        // Try one more request - should be rejected by circuit breaker
        let _ = resilient_client.get_site(1).await;

        let result = api.get_metrics().await;
        match result {
            GetMetricsResponse::Ok(Json(metrics)) => {
                assert!(metrics.netbox.is_some());
                let netbox_metrics = metrics.netbox.unwrap();
                assert!(netbox_metrics.circuit_breaker_rejections > 0);
                assert_eq!(netbox_metrics.circuit_breaker_state, "Open");
            }
        }
    }
}

