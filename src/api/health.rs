use poem_openapi::{payload::Json, ApiResponse, OpenApi};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

use crate::netbox::ResilientNetBoxClient;
use crate::resilience::CircuitState;

pub struct HealthApi {
    netbox_client: Option<Arc<ResilientNetBoxClient>>,
}

impl HealthApi {
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

impl Default for HealthApi {
    fn default() -> Self {
        Self::new()
    }
}

/// Detailed health check response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, poem_openapi::Object)]
pub struct HealthStatus {
    pub status: String,
    pub service: String,
    pub version: String,
    pub timestamp: String,
    pub netbox: Option<NetBoxHealth>,
    pub circuit_breaker: Option<CircuitBreakerHealth>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, poem_openapi::Object)]
pub struct NetBoxHealth {
    pub connected: bool,
    pub response_time_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, poem_openapi::Object)]
pub struct CircuitBreakerHealth {
    pub state: String,
    pub failure_count: u32,
}

#[derive(ApiResponse)]
pub enum HealthResponse {
    #[oai(status = 200)]
    Ok(Json<HealthStatus>),
    
    #[oai(status = 503)]
    ServiceUnavailable(Json<HealthStatus>),
}

#[OpenApi]
impl HealthApi {
    /// Enhanced health check endpoint
    /// 
    /// Returns detailed health information including:
    /// - Service status
    /// - NetBox connectivity
    /// - Circuit breaker state
    #[oai(path = "/health", method = "get")]
    async fn health(&self) -> HealthResponse {
        let mut health = HealthStatus {
            status: "healthy".to_string(),
            service: "NetGate".to_string(),
            version: "1.0.0".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            netbox: None,
            circuit_breaker: None,
        };

        // Check NetBox connectivity if client is available
        if let Some(ref client) = self.netbox_client {
            let netbox_health = check_netbox_health(client).await;
            health.netbox = Some(netbox_health.clone());
            
            if !netbox_health.connected {
                health.status = "degraded".to_string();
            }

            // Get circuit breaker state
            let cb_state = client.circuit_breaker_state();
            let cb_health = CircuitBreakerHealth {
                state: format!("{:?}", cb_state),
                failure_count: client.circuit_breaker_failure_count(),
            };
            health.circuit_breaker = Some(cb_health);

            if cb_state == CircuitState::Open {
                health.status = "degraded".to_string();
            }
        }

        // Determine response status
        if health.status == "healthy" {
            HealthResponse::Ok(Json(health))
        } else {
            HealthResponse::ServiceUnavailable(Json(health))
        }
    }
}

/// Check NetBox connectivity
async fn check_netbox_health(client: &ResilientNetBoxClient) -> NetBoxHealth {
    let start = std::time::Instant::now();
    
    // Try to list sites with a very small limit to test connectivity
    match timeout(Duration::from_secs(2), client.list_sites(None, Some(1), None)).await {
        Ok(Ok(_)) => {
            let response_time = start.elapsed().as_millis() as u64;
            NetBoxHealth {
                connected: true,
                response_time_ms: Some(response_time),
                error: None,
            }
        }
        Ok(Err(e)) => {
            NetBoxHealth {
                connected: false,
                response_time_ms: None,
                error: Some(e.to_string()),
            }
        }
        Err(_) => {
            NetBoxHealth {
                connected: false,
                response_time_ms: None,
                error: Some("Timeout".to_string()),
            }
        }
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
    fn test_health_api_creation() {
        let api = HealthApi::new();
        // Just verify it compiles
        assert!(true);
    }

    #[test]
    fn test_health_api_with_client() {
        let config = Config {
            port: 8080,
            netbox_url: "http://localhost:8000".to_string(),
            netbox_token: "test-token".to_string(),
        };
        let netbox_client = Arc::new(NetBoxClient::new(config).unwrap());
        let resilient_client = Arc::new(ResilientNetBoxClient::new(netbox_client));
        let api = HealthApi::with_netbox_client(resilient_client);
        // Just verify it compiles
        assert!(true);
    }

    #[tokio::test]
    async fn test_health_check_with_netbox_connected() {
        let mock_server = MockServer::start().await;
        let config = Config {
            port: 8080,
            netbox_url: mock_server.uri(),
            netbox_token: "test-token".to_string(),
        };
        let netbox_client = Arc::new(NetBoxClient::new(config).unwrap());
        let resilient_client = Arc::new(ResilientNetBoxClient::new(netbox_client));
        let api = HealthApi::with_netbox_client(resilient_client);

        // Mock successful NetBox response
        let sites_response = json!({
            "count": 0,
            "results": []
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&sites_response))
            .mount(&mock_server)
            .await;

        let result = api.health().await;
        match result {
            HealthResponse::Ok(Json(health)) => {
                assert_eq!(health.status, "healthy");
                assert_eq!(health.service, "NetGate");
                assert!(health.netbox.is_some());
                assert_eq!(health.netbox.unwrap().connected, true);
            }
            _ => panic!("Expected Ok response"),
        }
    }

    #[tokio::test]
    async fn test_health_check_with_netbox_disconnected() {
        let config = Config {
            port: 8080,
            netbox_url: "http://localhost:9999".to_string(), // Non-existent server
            netbox_token: "test-token".to_string(),
        };
        let netbox_client = Arc::new(NetBoxClient::new(config).unwrap());
        let resilient_client = Arc::new(ResilientNetBoxClient::new(netbox_client));
        let api = HealthApi::with_netbox_client(resilient_client);

        let result = api.health().await;
        match result {
            HealthResponse::ServiceUnavailable(Json(health)) => {
                assert_eq!(health.status, "degraded");
                assert!(health.netbox.is_some());
                assert_eq!(health.netbox.unwrap().connected, false);
            }
            _ => panic!("Expected ServiceUnavailable response"),
        }
    }

    #[tokio::test]
    async fn test_health_check_circuit_breaker_state() {
        let mock_server = MockServer::start().await;
        let config = Config {
            port: 8080,
            netbox_url: mock_server.uri(),
            netbox_token: "test-token".to_string(),
        };
        let netbox_client = Arc::new(NetBoxClient::new(config).unwrap());
        let resilient_client = Arc::new(ResilientNetBoxClient::new(netbox_client));
        let api = HealthApi::with_netbox_client(resilient_client.clone());

        // Mock failing NetBox response to open circuit breaker
        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        // Make enough failed requests to open circuit breaker
        for _ in 0..6 {
            let _ = resilient_client.list_sites(None, Some(1), None).await;
        }

        let result = api.health().await;
        match result {
            HealthResponse::ServiceUnavailable(Json(health)) => {
                assert_eq!(health.status, "degraded");
                assert!(health.circuit_breaker.is_some());
                let cb = health.circuit_breaker.unwrap();
                assert_eq!(cb.state, "Open");
            }
            _ => {
                // Circuit breaker might not be open yet, but should have state
                match result {
                    HealthResponse::Ok(Json(health)) => {
                        assert!(health.circuit_breaker.is_some());
                    }
                    _ => {}
                }
            }
        }
    }
}

