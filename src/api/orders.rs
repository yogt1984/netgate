use poem::Request;
use poem_openapi::{payload::Json, ApiResponse, OpenApi};
use std::sync::Arc;

use crate::domain::{CreateSiteOrder, Site};
use crate::domain::tenant::TenantStore;
use crate::security::extract_tenant_id;

pub struct OrdersApi {
    store: Arc<TenantStore>,
}

impl OrdersApi {
    pub fn new(store: Arc<TenantStore>) -> Self {
        Self { store }
    }
}

#[derive(ApiResponse)]
pub enum CreateSiteResponse {
    #[oai(status = 201)]
    Created(Json<Site>),
    
    #[oai(status = 401)]
    Unauthorized,
}

#[OpenApi]
impl OrdersApi {
    #[oai(path = "/orders/site", method = "post")]
    async fn create_site(
        &self,
        req: &Request,
        body: Json<CreateSiteOrder>,
    ) -> Result<CreateSiteResponse, poem::Error> {
        let tenant_id = extract_tenant_id(req)?;
        
        let site = Site::from_order(body.0, tenant_id.clone());
        self.store.add_site(tenant_id, site.clone());
        
        Ok(CreateSiteResponse::Created(Json(site)))
    }
}

