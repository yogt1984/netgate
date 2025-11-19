use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

/// Order state in the workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderState {
    /// Order received, pending validation
    Pending,
    /// Order validated, ready for processing
    Validated,
    /// Order being processed (transforming, creating in NetBox)
    Processing,
    /// Order completed successfully
    Completed,
    /// Order failed (validation, transformation, or NetBox error)
    Failed,
    /// Order cancelled
    Cancelled,
}

impl OrderState {
    /// Check if order can transition to a new state
    pub fn can_transition_to(&self, new_state: OrderState) -> bool {
        match (self, new_state) {
            // From Pending
            (OrderState::Pending, OrderState::Validated) => true,
            (OrderState::Pending, OrderState::Failed) => true,
            (OrderState::Pending, OrderState::Cancelled) => true,
            
            // From Validated
            (OrderState::Validated, OrderState::Processing) => true,
            (OrderState::Validated, OrderState::Cancelled) => true,
            
            // From Processing
            (OrderState::Processing, OrderState::Completed) => true,
            (OrderState::Processing, OrderState::Failed) => true,
            
            // Terminal states
            (OrderState::Completed, _) => false,
            (OrderState::Failed, _) => false,
            (OrderState::Cancelled, _) => false,
            
            // Invalid transitions
            _ => false,
        }
    }

    /// Check if state is terminal (cannot transition further)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            OrderState::Completed | OrderState::Failed | OrderState::Cancelled
        )
    }
}

/// Order workflow entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderWorkflow {
    pub order_id: String,
    pub state: OrderState,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub error_message: Option<String>,
    pub netbox_site_id: Option<i32>,
    pub tenant_id: String,
}

impl OrderWorkflow {
    /// Create a new order workflow entry
    pub fn new(order_id: String, tenant_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            order_id,
            state: OrderState::Pending,
            created_at: now,
            updated_at: now,
            error_message: None,
            netbox_site_id: None,
            tenant_id,
        }
    }

    /// Transition to a new state
    pub fn transition_to(&mut self, new_state: OrderState) -> Result<(), WorkflowError> {
        if !self.state.can_transition_to(new_state) {
            return Err(WorkflowError::InvalidTransition {
                from: self.state,
                to: new_state,
            });
        }

        self.state = new_state;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    /// Mark as failed with error message
    pub fn mark_failed(&mut self, error: String) -> Result<(), WorkflowError> {
        self.transition_to(OrderState::Failed)?;
        self.error_message = Some(error);
        Ok(())
    }

    /// Mark as completed with NetBox site ID
    pub fn mark_completed(&mut self, netbox_site_id: i32) -> Result<(), WorkflowError> {
        self.transition_to(OrderState::Completed)?;
        self.netbox_site_id = Some(netbox_site_id);
        Ok(())
    }
}

/// Workflow error
#[derive(Debug, Clone, PartialEq)]
pub enum WorkflowError {
    InvalidTransition { from: OrderState, to: OrderState },
    OrderNotFound(String),
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowError::InvalidTransition { from, to } => {
                write!(f, "Cannot transition from {:?} to {:?}", from, to)
            }
            WorkflowError::OrderNotFound(id) => {
                write!(f, "Order not found: {}", id)
            }
        }
    }
}

/// Workflow manager for tracking order states
pub struct WorkflowManager {
    orders: RwLock<HashMap<String, OrderWorkflow>>,
}

impl Default for WorkflowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowManager {
    /// Create a new workflow manager
    pub fn new() -> Self {
        Self {
            orders: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new order workflow
    pub fn create_order(&self, tenant_id: String) -> String {
        let order_id = Uuid::new_v4().to_string();
        let workflow = OrderWorkflow::new(order_id.clone(), tenant_id);

        let mut orders = self.orders.write().unwrap();
        orders.insert(order_id.clone(), workflow);
        order_id
    }

    /// Get order workflow by ID
    pub fn get_order(&self, order_id: &str) -> Option<OrderWorkflow> {
        let orders = self.orders.read().unwrap();
        orders.get(order_id).cloned()
    }

    /// Update order state
    pub fn update_order_state(
        &self,
        order_id: &str,
        new_state: OrderState,
    ) -> Result<(), WorkflowError> {
        let mut orders = self.orders.write().unwrap();
        let workflow = orders
            .get_mut(order_id)
            .ok_or_else(|| WorkflowError::OrderNotFound(order_id.to_string()))?;

        workflow.transition_to(new_state)
    }

    /// Mark order as failed
    pub fn mark_order_failed(&self, order_id: &str, error: String) -> Result<(), WorkflowError> {
        let mut orders = self.orders.write().unwrap();
        let workflow = orders
            .get_mut(order_id)
            .ok_or_else(|| WorkflowError::OrderNotFound(order_id.to_string()))?;

        workflow.mark_failed(error)
    }

    /// Mark order as completed
    pub fn mark_order_completed(
        &self,
        order_id: &str,
        netbox_site_id: i32,
    ) -> Result<(), WorkflowError> {
        let mut orders = self.orders.write().unwrap();
        let workflow = orders
            .get_mut(order_id)
            .ok_or_else(|| WorkflowError::OrderNotFound(order_id.to_string()))?;

        workflow.mark_completed(netbox_site_id)
    }

    /// Get all orders for a tenant
    pub fn get_tenant_orders(&self, tenant_id: &str) -> Vec<OrderWorkflow> {
        let orders = self.orders.read().unwrap();
        orders
            .values()
            .filter(|w| w.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    /// Get orders by state
    pub fn get_orders_by_state(&self, state: OrderState) -> Vec<OrderWorkflow> {
        let orders = self.orders.read().unwrap();
        orders
            .values()
            .filter(|w| w.state == state)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_state_transitions() {
        assert!(OrderState::Pending.can_transition_to(OrderState::Validated));
        assert!(OrderState::Pending.can_transition_to(OrderState::Failed));
        assert!(!OrderState::Pending.can_transition_to(OrderState::Completed));

        assert!(OrderState::Validated.can_transition_to(OrderState::Processing));
        assert!(!OrderState::Validated.can_transition_to(OrderState::Pending));

        assert!(OrderState::Processing.can_transition_to(OrderState::Completed));
        assert!(OrderState::Processing.can_transition_to(OrderState::Failed));

        assert!(!OrderState::Completed.can_transition_to(OrderState::Processing));
        assert!(!OrderState::Failed.can_transition_to(OrderState::Pending));
    }

    #[test]
    fn test_terminal_states() {
        assert!(!OrderState::Pending.is_terminal());
        assert!(!OrderState::Validated.is_terminal());
        assert!(!OrderState::Processing.is_terminal());
        assert!(OrderState::Completed.is_terminal());
        assert!(OrderState::Failed.is_terminal());
        assert!(OrderState::Cancelled.is_terminal());
    }

    #[test]
    fn test_workflow_transition() {
        let mut workflow = OrderWorkflow::new("order-1".to_string(), "tenant-1".to_string());
        
        assert_eq!(workflow.state, OrderState::Pending);
        
        assert!(workflow.transition_to(OrderState::Validated).is_ok());
        assert_eq!(workflow.state, OrderState::Validated);
        
        assert!(workflow.transition_to(OrderState::Processing).is_ok());
        assert_eq!(workflow.state, OrderState::Processing);
        
        assert!(workflow.transition_to(OrderState::Completed).is_ok());
        assert_eq!(workflow.state, OrderState::Completed);
        
        // Cannot transition from completed
        assert!(workflow.transition_to(OrderState::Pending).is_err());
    }

    #[test]
    fn test_workflow_mark_failed() {
        let mut workflow = OrderWorkflow::new("order-1".to_string(), "tenant-1".to_string());
        workflow.transition_to(OrderState::Validated).unwrap();
        workflow.transition_to(OrderState::Processing).unwrap();
        
        assert!(workflow.mark_failed("Test error".to_string()).is_ok());
        assert_eq!(workflow.state, OrderState::Failed);
        assert_eq!(workflow.error_message, Some("Test error".to_string()));
    }

    #[test]
    fn test_workflow_mark_completed() {
        let mut workflow = OrderWorkflow::new("order-1".to_string(), "tenant-1".to_string());
        workflow.transition_to(OrderState::Validated).unwrap();
        workflow.transition_to(OrderState::Processing).unwrap();
        
        assert!(workflow.mark_completed(123).is_ok());
        assert_eq!(workflow.state, OrderState::Completed);
        assert_eq!(workflow.netbox_site_id, Some(123));
    }

    #[test]
    fn test_workflow_manager_create_order() {
        let manager = WorkflowManager::new();
        let order_id = manager.create_order("tenant-1".to_string());
        
        let workflow = manager.get_order(&order_id).unwrap();
        assert_eq!(workflow.state, OrderState::Pending);
        assert_eq!(workflow.tenant_id, "tenant-1");
    }

    #[test]
    fn test_workflow_manager_get_tenant_orders() {
        let manager = WorkflowManager::new();
        let order1 = manager.create_order("tenant-1".to_string());
        let order2 = manager.create_order("tenant-1".to_string());
        manager.create_order("tenant-2".to_string());

        let tenant_orders = manager.get_tenant_orders("tenant-1");
        assert_eq!(tenant_orders.len(), 2);
        assert!(tenant_orders.iter().any(|o| o.order_id == order1));
        assert!(tenant_orders.iter().any(|o| o.order_id == order2));
    }

    #[test]
    fn test_workflow_manager_get_orders_by_state() {
        let manager = WorkflowManager::new();
        let order1 = manager.create_order("tenant-1".to_string());
        let order2 = manager.create_order("tenant-1".to_string());
        
        manager.update_order_state(&order1, OrderState::Validated).unwrap();
        manager.update_order_state(&order2, OrderState::Validated).unwrap();
        manager.update_order_state(&order2, OrderState::Processing).unwrap();

        let pending = manager.get_orders_by_state(OrderState::Pending);
        assert_eq!(pending.len(), 0);

        let validated = manager.get_orders_by_state(OrderState::Validated);
        assert_eq!(validated.len(), 1);

        let processing = manager.get_orders_by_state(OrderState::Processing);
        assert_eq!(processing.len(), 1);
    }
}

