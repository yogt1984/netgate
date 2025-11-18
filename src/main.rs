mod api;
mod config;
mod domain;
mod error;
mod logging;
mod netbox;
mod security;

use std::sync::Arc;

use poem::listener::TcpListener;
use poem_openapi::OpenApiService;

use crate::api::{HealthApi, OrdersApi, TenantsApi};
use crate::config::Config;
use crate::domain::tenant::TenantStore;
use crate::logging::init;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init();
    
    let config = Config::from_env();
    let store = Arc::new(TenantStore::new());
    
    let health_api = HealthApi;
    let orders_api = OrdersApi::new(store.clone());
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
