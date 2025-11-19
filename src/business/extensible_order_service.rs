use crate::business::plugin::{NetBoxResource, OrderPayload, OrderProcessor, OrderTypeRegistry};
use crate::business::{EnrichmentData, OrderState, WorkflowManager};
use crate::error::AppError;
use crate::netbox::ResilientNetBoxClient;
use crate::security::TenantId;
use std::sync::Arc;
use tracing::{debug, error, info};

/// Extensible order service that uses the plugin pattern
pub struct ExtensibleOrderService {
    registry: Arc<OrderTypeRegistry>,
    workflow_manager: Arc<WorkflowManager>,
    netbox_client: Arc<ResilientNetBoxClient>,
}

impl ExtensibleOrderService {
    /// Create a new extensible order service with a registry
    pub fn new(
        registry: Arc<OrderTypeRegistry>,
        workflow_manager: Arc<WorkflowManager>,
        netbox_client: Arc<ResilientNetBoxClient>,
    ) -> Self {
        Self {
            registry,
            workflow_manager,
            netbox_client,
        }
    }

    /// Process an order through the full pipeline using the plugin pattern
    pub async fn process_order(
        &self,
        order: OrderPayload,
        tenant_id: TenantId,
        order_type: Option<&str>,
    ) -> Result<ProcessedOrderResult, AppError> {
        // Determine order type
        let order_type = order_type.unwrap_or_else(|| {
            self.registry.default_order_type()
        });

        // Get processor for this order type
        let processor = self.registry
            .get_processor(order_type)
            .ok_or_else(|| AppError::ValidationError(
                format!("No processor registered for order type: {}", order_type)
            ))?;

        // Step 1: Validate the order
        debug!("Validating {} order", order_type);
        processor.validate(&order)?;

        // Step 2: Create workflow entry
        debug!("Creating workflow");
        let order_id = self.workflow_manager.create_order(tenant_id.clone());
        info!("Processing {} order {} for tenant {}", order_type, order_id, tenant_id);

        // Step 3: Update workflow to Validated state
        self.workflow_manager.update_order_state(&order_id, OrderState::Validated)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Workflow error: {}", e)))?;

        // Step 4: Transform order to NetBox request
        debug!("Transforming order {} to NetBox request", order_id);
        let mut netbox_request = processor.transform(order, None)?;

        // Step 5: Enrich the NetBox request
        debug!("Enriching NetBox request for order {}", order_id);
        let enrichment_data = EnrichmentData::default();
        processor.enrich_request(&mut netbox_request, &enrichment_data)?;

        // Step 6: Update workflow to Processing state
        self.workflow_manager.update_order_state(&order_id, OrderState::Processing)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Workflow error: {}", e)))?;

        // Step 7: Create resource in NetBox
        debug!("Creating resource in NetBox for order {}", order_id);
        let netbox_resource = match processor.create_resource(&self.netbox_client, netbox_request).await {
            Ok(resource) => {
                // Step 8: Enrich the created resource
                let enriched_resource = processor.enrich_resource(resource, &enrichment_data);

                // Step 9: Update workflow with NetBox ID and mark as completed
                if let Some(resource_id) = enriched_resource.resource_id() {
                    self.workflow_manager.mark_order_completed(&order_id, resource_id)
                        .map_err(|e| AppError::Internal(anyhow::anyhow!("Workflow error: {}", e)))?;
                }

                info!("Successfully processed order {} - NetBox resource created", order_id);
                enriched_resource
            }
            Err(e) => {
                error!("Failed to create resource in NetBox for order {}: {}", order_id, e);

                // Mark workflow as failed
                let _ = self.workflow_manager.mark_order_failed(&order_id, e.to_string());

                return Err(e);
            }
        };

        // Get final workflow state
        let workflow = self.workflow_manager.get_order(&order_id)
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Workflow not found after processing")))?;

        Ok(ProcessedOrderResult {
            order_id,
            tenant_id,
            netbox_resource,
            workflow_state: workflow.state,
        })
    }

    /// Get order status by order ID
    pub async fn get_order_status(
        &self,
        order_id: &str,
        tenant_id: &TenantId,
    ) -> Result<OrderStatus, AppError> {
        let workflow = self.workflow_manager
            .get_order(order_id)
            .ok_or_else(|| AppError::NotFound(format!("Order {} not found", order_id)))?;

        // Verify tenant access
        if workflow.tenant_id != *tenant_id {
            return Err(AppError::Unauthorized);
        }

        Ok(OrderStatus {
            order_id: order_id.to_string(),
            state: workflow.state,
            netbox_resource_id: workflow.netbox_site_id,
            created_at: workflow.created_at,
            updated_at: workflow.updated_at,
        })
    }

    /// Get the order type registry
    pub fn registry(&self) -> &Arc<OrderTypeRegistry> {
        &self.registry
    }
}

/// Result of processing an order
#[derive(Debug, Clone)]
pub struct ProcessedOrderResult {
    pub order_id: String,
    pub tenant_id: TenantId,
    pub netbox_resource: NetBoxResource,
    pub workflow_state: OrderState,
}

/// Current status of an order
#[derive(Debug, Clone)]
pub struct OrderStatus {
    pub order_id: String,
    pub state: OrderState,
    pub netbox_resource_id: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Builder for creating an extensible order service with default processors
pub struct ExtensibleOrderServiceBuilder {
    registry: OrderTypeRegistry,
}

impl ExtensibleOrderServiceBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            registry: OrderTypeRegistry::default(),
        }
    }

    /// Register a processor
    pub fn with_processor(mut self, processor: Arc<dyn OrderProcessor>) -> Self {
        self.registry.register(processor);
        self
    }

    /// Register the default site processor
    pub fn with_default_processors(mut self) -> Self {
        use crate::business::processors::SiteOrderProcessor;
        self.registry.register(Arc::new(SiteOrderProcessor::new()));
        self
    }

    /// Build the service
    pub fn build(
        self,
        workflow_manager: Arc<WorkflowManager>,
        netbox_client: Arc<ResilientNetBoxClient>,
    ) -> ExtensibleOrderService {
        ExtensibleOrderService::new(
            Arc::new(self.registry),
            workflow_manager,
            netbox_client,
        )
    }
}

impl Default for ExtensibleOrderServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::netbox::client::NetBoxClient;
    use crate::netbox::resilient_client::ResilientNetBoxClient;

    fn create_test_netbox_client() -> Arc<ResilientNetBoxClient> {
        let config = Config {
            port: 8080,
            netbox_url: "http://localhost:8000".to_string(),
            netbox_token: "test-token".to_string(),
        };
        let client = Arc::new(NetBoxClient::new(config).unwrap());
        Arc::new(ResilientNetBoxClient::new(client))
    }

    #[test]
    fn test_extensible_order_service_builder() {
        let builder = ExtensibleOrderServiceBuilder::new()
            .with_default_processors();
        
        let workflow_manager = Arc::new(WorkflowManager::new());
        let netbox_client = create_test_netbox_client();
        let _service = builder.build(workflow_manager, netbox_client);
        
        // Just verify it compiles
        assert!(true);
    }

    #[tokio::test]
    async fn test_get_order_status_not_found() {
        let builder = ExtensibleOrderServiceBuilder::new()
            .with_default_processors();
        
        let workflow_manager = Arc::new(WorkflowManager::new());
        let netbox_client = create_test_netbox_client();
        let service = builder.build(workflow_manager, netbox_client);

        let result = service.get_order_status("nonexistent", &"tenant1".to_string()).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::NotFound(_) => {}
            _ => panic!("Expected NotFound error"),
        }
    }
}

