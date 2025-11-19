use crate::business::enrichment::EnrichmentData;
use crate::error::AppError;
use crate::netbox::models::{CreateSiteRequest, NetBoxSite};
use crate::netbox::ResilientNetBoxClient;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

/// Order type identifier
pub type OrderType = String;

/// Order payload enum - represents different order types
#[derive(Debug, Clone)]
pub enum OrderPayload {
    Site(crate::domain::CreateSiteOrder),
    // Future: Device(crate::domain::CreateDeviceOrder),
    // Future: Network(crate::domain::CreateNetworkOrder),
}

impl OrderPayload {
    pub fn order_type(&self) -> &'static str {
        match self {
            OrderPayload::Site(_) => "site",
        }
    }
}

/// NetBox resource request enum
#[derive(Debug, Clone)]
pub enum NetBoxResourceRequest {
    Site(CreateSiteRequest),
    // Future: Device(CreateDeviceRequest),
}

impl NetBoxResourceRequest {
    pub fn resource_type(&self) -> &str {
        match self {
            NetBoxResourceRequest::Site(_) => "site",
        }
    }
}

/// NetBox resource enum
#[derive(Debug, Clone)]
pub enum NetBoxResource {
    Site(NetBoxSite),
    // Future: Device(NetBoxDevice),
}

impl NetBoxResource {
    pub fn resource_id(&self) -> Option<i32> {
        match self {
            NetBoxResource::Site(site) => site.id,
        }
    }

    pub fn resource_type(&self) -> &str {
        match self {
            NetBoxResource::Site(_) => "site",
        }
    }
}

/// Order processor trait - defines the contract for processing different order types
#[async_trait]
pub trait OrderProcessor: Send + Sync {
    /// Get the order type this processor handles
    fn order_type(&self) -> &'static str;

    /// Validate the order
    fn validate(&self, order: &OrderPayload) -> Result<(), AppError>;

    /// Transform the order to a NetBox resource request
    fn transform(
        &self,
        order: OrderPayload,
        tenant_id: Option<i32>,
    ) -> Result<NetBoxResourceRequest, AppError>;

    /// Enrich the NetBox resource request
    fn enrich_request(
        &self,
        request: &mut NetBoxResourceRequest,
        enrichment_data: &EnrichmentData,
    ) -> Result<(), AppError>;

    /// Create the resource in NetBox
    async fn create_resource(
        &self,
        client: &Arc<ResilientNetBoxClient>,
        request: NetBoxResourceRequest,
    ) -> Result<NetBoxResource, AppError>;

    /// Enrich the created NetBox resource
    fn enrich_resource(
        &self,
        resource: NetBoxResource,
        enrichment_data: &EnrichmentData,
    ) -> NetBoxResource;
}

/// Order type registry for managing order processors
pub struct OrderTypeRegistry {
    processors: HashMap<String, Arc<dyn OrderProcessor>>,
    default_order_type: String,
}

impl OrderTypeRegistry {
    /// Create a new registry
    pub fn new(default_order_type: OrderType) -> Self {
        Self {
            processors: HashMap::new(),
            default_order_type,
        }
    }

    /// Register an order processor
    pub fn register(&mut self, processor: Arc<dyn OrderProcessor>) {
        let order_type = processor.order_type().to_string();
        debug!("Registering order processor for type: {}", order_type);
        self.processors.insert(order_type, processor);
    }

    /// Get a processor for an order type
    pub fn get_processor(&self, order_type: &str) -> Option<Arc<dyn OrderProcessor>> {
        self.processors.get(order_type).cloned()
    }

    /// Get the default order type
    pub fn default_order_type(&self) -> &str {
        &self.default_order_type
    }

    /// Get all registered order types
    pub fn registered_types(&self) -> Vec<String> {
        self.processors.keys().cloned().collect()
    }

    /// Check if an order type is registered
    pub fn is_registered(&self, order_type: &str) -> bool {
        self.processors.contains_key(order_type)
    }
}

impl Default for OrderTypeRegistry {
    fn default() -> Self {
        Self::new("site".to_string())
    }
}

/// Configuration for order type mappings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderTypeConfig {
    /// Order type identifier
    pub order_type: OrderType,
    /// Processor class/name
    pub processor: String,
    /// Configuration parameters
    pub config: HashMap<String, serde_json::Value>,
}

/// Order type configuration loader
pub struct OrderTypeConfigLoader;

impl OrderTypeConfigLoader {
    /// Load order type configurations from a map
    pub fn load_from_map(
        configs: HashMap<String, OrderTypeConfig>,
    ) -> Result<Vec<OrderTypeConfig>, AppError> {
        Ok(configs.into_values().collect())
    }

    /// Create default configurations
    pub fn default_configs() -> Vec<OrderTypeConfig> {
        vec![OrderTypeConfig {
            order_type: "site".to_string(),
            processor: "SiteOrderProcessor".to_string(),
            config: HashMap::new(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::processors::SiteOrderProcessor;
    use std::sync::Arc;

    #[test]
    fn test_order_type_registry_creation() {
        let registry = OrderTypeRegistry::new("site".to_string());
        assert_eq!(registry.default_order_type(), "site");
        assert!(registry.registered_types().is_empty());
    }

    #[test]
    fn test_order_type_registry_default() {
        let registry = OrderTypeRegistry::default();
        assert_eq!(registry.default_order_type(), "site");
    }

    #[test]
    fn test_order_type_registry_register() {
        let mut registry = OrderTypeRegistry::new("site".to_string());
        let processor = Arc::new(SiteOrderProcessor::new());
        
        registry.register(processor);
        assert_eq!(registry.registered_types().len(), 1);
        assert!(registry.is_registered("site"));
    }

    #[test]
    fn test_order_type_registry_get_processor() {
        let mut registry = OrderTypeRegistry::new("site".to_string());
        let processor = Arc::new(SiteOrderProcessor::new());
        
        registry.register(processor);
        
        let retrieved = registry.get_processor("site");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().order_type(), "site");
    }

    #[test]
    fn test_order_type_registry_get_nonexistent() {
        let registry = OrderTypeRegistry::new("site".to_string());
        let retrieved = registry.get_processor("nonexistent");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_order_type_config_loader_default() {
        let configs = OrderTypeConfigLoader::default_configs();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].order_type, "site");
    }

    #[test]
    fn test_order_payload_order_type() {
        let order = OrderPayload::Site(crate::domain::CreateSiteOrder {
            name: "Test".to_string(),
            description: None,
            address: None,
        });
        assert_eq!(order.order_type(), "site");
    }

    #[test]
    fn test_netbox_resource_request_type() {
        use crate::netbox::models::CreateSiteRequest;
        let request = NetBoxResourceRequest::Site(CreateSiteRequest {
            name: "Test".to_string(),
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
        });
        assert_eq!(request.resource_type(), "site");
    }

    #[test]
    fn test_netbox_resource_id() {
        use crate::netbox::models::NetBoxSite;
        let resource = NetBoxResource::Site(NetBoxSite {
            id: Some(123),
            name: "Test".to_string(),
            ..Default::default()
        });
        assert_eq!(resource.resource_id(), Some(123));
        assert_eq!(resource.resource_type(), "site");
    }
}

