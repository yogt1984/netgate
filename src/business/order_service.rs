use crate::business::{
    OrderTransformer, OrderValidator, ObjectEnricher, EnrichmentData,
    OrderState, WorkflowManager,
};
use crate::domain::CreateSiteOrder;
use crate::error::AppError;
use crate::netbox::{
    ResilientNetBoxClient, NetBoxSite,
};
use crate::security::TenantId;
use std::sync::Arc;
use tracing::{debug, error, info};

/// Order service that orchestrates the full order processing flow
pub struct OrderService {
    validator: OrderValidator,
    transformer: OrderTransformer,
    enricher: ObjectEnricher,
    workflow_manager: Arc<WorkflowManager>,
    netbox_client: Arc<ResilientNetBoxClient>,
}

impl OrderService {
    /// Create a new order service
    pub fn new(
        workflow_manager: Arc<WorkflowManager>,
        netbox_client: Arc<ResilientNetBoxClient>,
    ) -> Self {
        Self {
            validator: OrderValidator::new(),
            transformer: OrderTransformer::new(),
            enricher: ObjectEnricher::new(),
            workflow_manager,
            netbox_client,
        }
    }

    /// Process a site order through the full pipeline:
    /// 1. Validate the order
    /// 2. Create workflow entry
    /// 3. Transform order to NetBox request
    /// 4. Enrich the NetBox request
    /// 5. Create site in NetBox
    /// 6. Update workflow state
    pub async fn process_site_order(
        &self,
        order: CreateSiteOrder,
        tenant_id: TenantId,
    ) -> Result<ProcessedOrderResult, AppError> {
        // Step 1: Validate the order
        debug!("Validating order");
        self.validator.validate_site_order(&order)?;

        // Step 2: Create workflow entry (this generates the order ID)
        debug!("Creating workflow");
        let order_id = self.workflow_manager.create_order(tenant_id.clone());
        info!("Processing site order {} for tenant {}", order_id, tenant_id);
        
        // Step 3: Update workflow to Validated state
        self.workflow_manager.update_order_state(&order_id, OrderState::Validated)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Workflow error: {}", e)))?;

        // Step 4: Transform order to NetBox request
        debug!("Transforming order {} to NetBox request", order_id);
        let mut netbox_request = self.transformer.transform_site_order(order, None);

        // Step 5: Enrich the NetBox request (apply enrichment to tags and description)
        debug!("Enriching NetBox request for order {}", order_id);
        let enrichment_data = EnrichmentData::default();
        
        // Apply enrichment tags to the request
        let mut tags = netbox_request.tags.unwrap_or_default();
        tags.push("netgate".to_string());
        tags.push("enriched".to_string());
        netbox_request.tags = Some(tags);

        // Step 6: Update workflow to Processing state
        self.workflow_manager.update_order_state(&order_id, OrderState::Processing)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Workflow error: {}", e)))?;

        // Step 7: Create site in NetBox
        debug!("Creating site in NetBox for order {}", order_id);
        let netbox_site = match self.netbox_client.create_site(netbox_request).await {
            Ok(site) => {
                // Step 8: Enrich the created site
                let enriched_site = self.enricher.enrich_site(site, &enrichment_data);
                
                // Step 9: Update workflow with NetBox ID and mark as completed
                if let Some(site_id) = enriched_site.id {
                    self.workflow_manager.mark_order_completed(&order_id, site_id)
                        .map_err(|e| AppError::Internal(anyhow::anyhow!("Workflow error: {}", e)))?;
                }

                info!("Successfully processed order {} - NetBox site created", order_id);
                enriched_site
            }
            Err(e) => {
                error!("Failed to create site in NetBox for order {}: {}", order_id, e);
                
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
            netbox_site,
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
            netbox_site_id: workflow.netbox_site_id,
            created_at: workflow.created_at,
            updated_at: workflow.updated_at,
        })
    }
}

/// Result of processing an order
#[derive(Debug, Clone)]
pub struct ProcessedOrderResult {
    pub order_id: String,
    pub tenant_id: TenantId,
    pub netbox_site: NetBoxSite,
    pub workflow_state: OrderState,
}

/// Order status information
#[derive(Debug, Clone)]
pub struct OrderStatus {
    pub order_id: String,
    pub state: OrderState,
    pub netbox_site_id: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::netbox::client::NetBoxClient;
    use std::sync::Arc;

    fn create_test_order() -> CreateSiteOrder {
        CreateSiteOrder {
            name: "Test Site".to_string(),
            description: Some("Test Description".to_string()),
            address: Some("123 Test St".to_string()),
        }
    }

    fn create_test_netbox_client() -> Arc<ResilientNetBoxClient> {
        let config = Config {
            port: 8080,
            netbox_url: "http://localhost:8000".to_string(),
            netbox_token: "test-token".to_string(),
        };
        let client = Arc::new(NetBoxClient::new(config).unwrap());
        Arc::new(ResilientNetBoxClient::new(client))
    }

    #[tokio::test]
    async fn test_order_service_creation() {
        let workflow_manager = Arc::new(WorkflowManager::new());
        let netbox_client = create_test_netbox_client();
        let service = OrderService::new(workflow_manager, netbox_client);
        
        // Service should be created successfully
        assert!(true); // Just verify it compiles and creates
    }

    #[tokio::test]
    async fn test_get_order_status_not_found() {
        let workflow_manager = Arc::new(WorkflowManager::new());
        let netbox_client = create_test_netbox_client();
        let service = OrderService::new(workflow_manager, netbox_client);
        
        let result = service.get_order_status("nonexistent", &"tenant1".to_string()).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::NotFound(_) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_get_order_status_unauthorized() {
        let workflow_manager = Arc::new(WorkflowManager::new());
        let netbox_client = create_test_netbox_client();
        let service = OrderService::new(workflow_manager.clone(), netbox_client);
        
        // Create workflow for tenant1
        let order_id = workflow_manager.create_order("tenant1".to_string());
        
        // Try to access with tenant2
        let result = service.get_order_status(&order_id, &"tenant2".to_string()).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Unauthorized => {}
            _ => panic!("Expected Unauthorized error"),
        }
    }

    #[tokio::test]
    async fn test_get_order_status_success() {
        let workflow_manager = Arc::new(WorkflowManager::new());
        let netbox_client = create_test_netbox_client();
        let service = OrderService::new(workflow_manager.clone(), netbox_client);
        
        // Create workflow for tenant1
        let order_id = workflow_manager.create_order("tenant1".to_string());
        workflow_manager.update_order_state(&order_id, OrderState::Validated).unwrap();
        
        let result = service.get_order_status(&order_id, &"tenant1".to_string()).await;
        assert!(result.is_ok());
        let status = result.unwrap();
        assert_eq!(status.order_id, order_id);
        assert_eq!(status.state, OrderState::Validated);
    }

    #[tokio::test]
    async fn test_process_site_order_validation_failure() {
        let workflow_manager = Arc::new(WorkflowManager::new());
        let netbox_client = create_test_netbox_client();
        let service = OrderService::new(workflow_manager, netbox_client);
        
        // Create invalid order (empty name)
        let invalid_order = CreateSiteOrder {
            name: "".to_string(),
            description: None,
            address: None,
        };
        
        let result = service.process_site_order(invalid_order, "tenant1".to_string()).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::ValidationError(_) => {}
            _ => panic!("Expected ValidationError"),
        }
    }

    #[tokio::test]
    async fn test_process_site_order_workflow_states() {
        let workflow_manager = Arc::new(WorkflowManager::new());
        let netbox_client = create_test_netbox_client();
        let service = OrderService::new(workflow_manager.clone(), netbox_client);
        
        let order = create_test_order();
        
        // Process will fail at NetBox creation (no mock server), but we can verify workflow states
        let result = service.process_site_order(order, "tenant1".to_string()).await;
        
        // Should fail at NetBox creation, but workflow should be created
        assert!(result.is_err());
        
        // Verify workflow was created and transitioned through states
        // The order_id is generated in process_site_order, so we need to check all orders
        let orders = workflow_manager.get_tenant_orders("tenant1");
        assert!(!orders.is_empty());
        
        // The last order should be in Failed state
        let last_order = orders.last().unwrap();
        assert_eq!(last_order.state, OrderState::Failed);
    }

    #[tokio::test]
    async fn test_order_service_full_flow_with_mock() {
        use crate::netbox::client::NetBoxClient;
        use crate::netbox::resilient_client::ResilientNetBoxClient;
        use serde_json::json;
        use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};
        
        let mock_server = MockServer::start().await;
        let config = Config {
            port: 8080,
            netbox_url: mock_server.uri(),
            netbox_token: "test-token".to_string(),
        };
        let netbox_client = Arc::new(NetBoxClient::new(config).unwrap());
        let resilient_client = Arc::new(ResilientNetBoxClient::new(netbox_client));
        
        let workflow_manager = Arc::new(WorkflowManager::new());
        let service = OrderService::new(workflow_manager.clone(), resilient_client);
        
        // Mock NetBox API response
        let site_response = json!({
            "id": 123,
            "name": "Test Site",
            "description": "Test Description",
            "status": "active",
            "tags": ["netgate", "enriched"]
        });
        
        Mock::given(method("POST"))
            .and(path("/api/dcim/sites/"))
            .respond_with(ResponseTemplate::new(201).set_body_json(&site_response))
            .mount(&mock_server)
            .await;
        
        let order = create_test_order();
        let result = service.process_site_order(order, "tenant1".to_string()).await;
        
        assert!(result.is_ok());
        let processed = result.unwrap();
        assert_eq!(processed.netbox_site.id, Some(123));
        assert_eq!(processed.netbox_site.name, "Test Site");
        assert_eq!(processed.workflow_state, OrderState::Completed);
        
        // Verify workflow state
        let workflow = workflow_manager.get_order(&processed.order_id).unwrap();
        assert_eq!(workflow.state, OrderState::Completed);
        assert_eq!(workflow.netbox_site_id, Some(123));
    }

    #[tokio::test]
    async fn test_order_service_netbox_failure_handling() {
        use crate::netbox::client::NetBoxClient;
        use crate::netbox::resilient_client::ResilientNetBoxClient;
        use serde_json::json;
        use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};
        
        let mock_server = MockServer::start().await;
        let config = Config {
            port: 8080,
            netbox_url: mock_server.uri(),
            netbox_token: "test-token".to_string(),
        };
        let netbox_client = Arc::new(NetBoxClient::new(config).unwrap());
        let resilient_client = Arc::new(ResilientNetBoxClient::new(netbox_client));
        
        let workflow_manager = Arc::new(WorkflowManager::new());
        let service = OrderService::new(workflow_manager.clone(), resilient_client);
        
        // Mock NetBox API error
        Mock::given(method("POST"))
            .and(path("/api/dcim/sites/"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({
                "detail": "Internal server error"
            })))
            .mount(&mock_server)
            .await;
        
        let order = create_test_order();
        let result = service.process_site_order(order, "tenant1".to_string()).await;
        
        assert!(result.is_err());
        
        // Verify workflow is in Failed state
        let orders = workflow_manager.get_tenant_orders("tenant1");
        assert!(!orders.is_empty());
        let failed_order = orders.last().unwrap();
        assert_eq!(failed_order.state, OrderState::Failed);
        assert!(failed_order.error_message.is_some());
    }
}

