use poem::Request;
use poem_openapi::{payload::Json, ApiResponse, OpenApi, param::Path};
use std::sync::Arc;

use crate::business::OrderService;
use crate::domain::CreateSiteOrder;
use crate::error::AppError;
use crate::security::extract_tenant_id;

pub struct OrdersApi {
    order_service: Arc<OrderService>,
}

impl OrdersApi {
    pub fn new(order_service: Arc<OrderService>) -> Self {
        Self { order_service }
    }
}

/// Response for site order creation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, poem_openapi::Object)]
pub struct SiteOrderResponse {
    pub order_id: String,
    pub tenant_id: String,
    pub netbox_site_id: Option<i32>,
    pub state: String,
    pub site_name: String,
}

#[derive(ApiResponse)]
pub enum CreateSiteResponse {
    #[oai(status = 201)]
    Created(Json<SiteOrderResponse>),
    
    #[oai(status = 400)]
    BadRequest(Json<serde_json::Value>),
    
    #[oai(status = 401)]
    Unauthorized,
    
    #[oai(status = 500)]
    InternalError(Json<serde_json::Value>),
}

/// Response for order status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, poem_openapi::Object)]
pub struct OrderStatusResponse {
    pub order_id: String,
    pub state: String,
    pub netbox_site_id: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(ApiResponse)]
pub enum GetOrderStatusResponse {
    #[oai(status = 200)]
    Ok(Json<OrderStatusResponse>),
    
    #[oai(status = 401)]
    Unauthorized,
    
    #[oai(status = 404)]
    NotFound,
}

#[OpenApi]
impl OrdersApi {
    /// Create a new site order
    /// 
    /// This endpoint processes a site order through the full pipeline:
    /// 1. Validates the order
    /// 2. Transforms it to NetBox format
    /// 3. Enriches it with computed fields
    /// 4. Creates the site in NetBox
    /// 5. Tracks the workflow state
    #[oai(path = "/orders/site", method = "post")]
    async fn create_site(
        &self,
        req: &Request,
        body: Json<CreateSiteOrder>,
    ) -> Result<CreateSiteResponse, poem::Error> {
        let tenant_id = extract_tenant_id(req)?;
        
        match self.order_service.process_site_order(body.0, tenant_id.clone()).await {
            Ok(result) => {
                Ok(CreateSiteResponse::Created(Json(SiteOrderResponse {
                    order_id: result.order_id,
                    tenant_id: result.tenant_id,
                    netbox_site_id: result.netbox_site.id,
                    state: format!("{:?}", result.workflow_state),
                    site_name: result.netbox_site.name,
                })))
            }
            Err(AppError::ValidationError(msg)) => {
                Ok(CreateSiteResponse::BadRequest(Json(serde_json::json!({
                    "error": "Validation failed",
                    "message": msg
                }))))
            }
            Err(AppError::Unauthorized) => {
                Ok(CreateSiteResponse::Unauthorized)
            }
            Err(e) => {
                Ok(CreateSiteResponse::InternalError(Json(serde_json::json!({
                    "error": "Internal server error",
                    "message": e.to_string()
                }))))
            }
        }
    }

    /// Get the status of an order
    #[oai(path = "/orders/:order_id/status", method = "get")]
    async fn get_order_status(
        &self,
        req: &Request,
        order_id: Path<String>,
    ) -> Result<GetOrderStatusResponse, poem::Error> {
        let tenant_id = extract_tenant_id(req)?;
        
        match self.order_service.get_order_status(&order_id.0, &tenant_id).await {
            Ok(status) => {
                Ok(GetOrderStatusResponse::Ok(Json(OrderStatusResponse {
                    order_id: status.order_id,
                    state: format!("{:?}", status.state),
                    netbox_site_id: status.netbox_site_id,
                    created_at: status.created_at.to_rfc3339(),
                    updated_at: status.updated_at.to_rfc3339(),
                })))
            }
            Err(AppError::NotFound(_)) => {
                Ok(GetOrderStatusResponse::NotFound)
            }
            Err(AppError::Unauthorized) => {
                Ok(GetOrderStatusResponse::Unauthorized)
            }
            Err(_) => {
                Ok(GetOrderStatusResponse::NotFound)
            }
        }
    }
}

