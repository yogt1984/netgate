mod api;
mod business;
mod config;
mod domain;
mod error;
mod logging;
mod netbox;
mod resilience;
mod security;

use std::sync::Arc;

use poem::listener::TcpListener;
use poem_openapi::OpenApiService;

use crate::api::{HealthApi, OrdersApi, TenantsApi};
use crate::business::{OrderService, WorkflowManager};
use crate::config::Config;
use crate::domain::tenant::TenantStore;
use crate::logging::init;
use crate::netbox::{NetBoxClient, ResilientNetBoxClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init();
    
    let config = Config::from_env();
    
    // Initialize NetBox client
    let netbox_config = Config {
        port: config.port,
        netbox_url: config.netbox_url.clone(),
        netbox_token: config.netbox_token.clone(),
    };
    let netbox_client = Arc::new(NetBoxClient::new(netbox_config)
        .map_err(|e| format!("Failed to create NetBox client: {}", e))?);
    let resilient_netbox_client = Arc::new(ResilientNetBoxClient::new(netbox_client));
    
    // Initialize workflow manager
    let workflow_manager = Arc::new(WorkflowManager::new());
    
    // Initialize order service
    let order_service = Arc::new(OrderService::new(
        workflow_manager,
        resilient_netbox_client,
    ));
    
    // Initialize stores
    let store = Arc::new(TenantStore::new());
    
    // Initialize APIs
    let health_api = HealthApi;
    let orders_api = OrdersApi::new(order_service);
    let tenants_api = TenantsApi::new(store);
    
    let api_service = OpenApiService::new(
        (health_api, orders_api, tenants_api),
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
