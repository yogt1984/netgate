mod api;
mod business;
mod cache;
mod config;
mod domain;
mod error;
mod logging;
mod netbox;
mod observability;
mod resilience;
mod security;

use std::sync::Arc;

use poem::listener::TcpListener;
use poem_openapi::OpenApiService;

use crate::api::{HealthApi, MetricsApi, OrdersApi, TenantsApi};
use crate::business::{OrderService, WorkflowManager};
use crate::config::Config;
use crate::domain::tenant::TenantStore;
use crate::logging::init;
use crate::netbox::{NetBoxClient, ResilientNetBoxClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init();
    
    let config = Config::from_env();
    
    // Initialize NetBox client (optional - server can run without NetBox for demo)
    let resilient_netbox_client = if config.netbox_token.is_empty() {
        tracing::warn!("NETBOX_TOKEN not set - NetBox features will be unavailable. Set NETBOX_TOKEN to enable NetBox integration.");
        None
    } else {
        let netbox_config = Config {
            port: config.port,
            netbox_url: config.netbox_url.clone(),
            netbox_token: config.netbox_token.clone(),
        };
        match NetBoxClient::new(netbox_config) {
            Ok(client) => {
                tracing::info!("NetBox client initialized successfully");
                Some(Arc::new(ResilientNetBoxClient::new(Arc::new(client))))
            }
            Err(e) => {
                tracing::warn!("Failed to create NetBox client: {}. Server will run without NetBox integration.", e);
                None
            }
        }
    };
    
    // Initialize workflow manager
    let workflow_manager = Arc::new(WorkflowManager::new());
    
    // Initialize order service (requires NetBox client)
    let order_service = if let Some(ref client) = resilient_netbox_client {
        Some(Arc::new(OrderService::new(
            workflow_manager.clone(),
            client.clone(),
        )))
    } else {
        tracing::warn!("OrderService not initialized - NetBox client unavailable. Order endpoints will return errors.");
        None
    };
    
    // Initialize stores
    let store = Arc::new(TenantStore::new());
    
    // Initialize APIs
    let health_api = if let Some(ref client) = resilient_netbox_client {
        HealthApi::with_netbox_client(client.clone())
    } else {
        HealthApi::new()
    };
    
    let metrics_api = if let Some(ref client) = resilient_netbox_client {
        MetricsApi::with_netbox_client(client.clone())
    } else {
        MetricsApi::new()
    };
    
    // For orders API, we need a NetBox client. If unavailable, create a minimal one
    // that will fail gracefully when used
    let orders_api = if let Some(ref service) = order_service {
        OrdersApi::new(service.clone())
    } else {
        // Create a service with a dummy client - will fail when NetBox is called
        // but allows the server to start
        let dummy_config = Config {
            port: 8080,
            netbox_url: "http://localhost:8000".to_string(),
            netbox_token: "dummy-token-for-startup".to_string(),
        };
        let dummy_client = Arc::new(ResilientNetBoxClient::new(Arc::new(
            NetBoxClient::new(dummy_config).unwrap_or_else(|_| {
                // If this fails, we're in trouble, but try to continue
                panic!("Cannot create even dummy NetBox client")
            })
        )));
        OrdersApi::new(Arc::new(OrderService::new(
            workflow_manager.clone(),
            dummy_client,
        )))
    };
    let tenants_api = TenantsApi::new(store);
    
    let api_service = OpenApiService::new(
        (health_api, metrics_api, orders_api, tenants_api),
        "NetGate API",
        "1.0",
    )
    .server("http://localhost:8080");
    
    let ui = api_service.swagger_ui();
    let spec = api_service.spec_endpoint();
    
    let app = poem::Route::new()
        .nest("/", api_service)
        .nest("/docs", ui)
        .nest("/spec", spec);
    
    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Starting NetGate server on {}", addr);
    
    poem::Server::new(TcpListener::bind(&addr))
        .run(app)
        .await?;
    
    Ok(())
}
