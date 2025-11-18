use poem_openapi::{payload::Json, ApiResponse, OpenApi, param::Path};
use poem::Request;
use std::sync::Arc;

use crate::domain::Site;
use crate::domain::tenant::TenantStore;
use crate::error::AppError;
use crate::security::extract_tenant_id;

pub struct TenantsApi {
    store: Arc<TenantStore>,
}

impl TenantsApi {
    pub fn new(store: Arc<TenantStore>) -> Self {
        Self { store }
    }
}

#[derive(ApiResponse)]
pub enum GetSitesResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<Site>>),
    
    #[oai(status = 401)]
    Unauthorized,
}

#[OpenApi]
impl TenantsApi {
    #[oai(path = "/tenants/:tenant_id/sites", method = "get")]
    async fn get_sites(
        &self,
        req: &Request,
        tenant_id: Path<String>,
    ) -> Result<GetSitesResponse, poem::Error> {
        // Verify the tenant_id in path matches the one in header
        let header_tenant_id = extract_tenant_id(req)?;
        
        if header_tenant_id != tenant_id.0 {
            return Err(AppError::Unauthorized.into());
        }
        
        let sites = self.store.get_sites(&header_tenant_id);
        Ok(GetSitesResponse::Ok(Json(sites)))
    }
}

