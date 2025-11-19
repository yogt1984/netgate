use crate::error::AppError;
use crate::netbox::client::NetBoxClient;
use crate::netbox::models::*;
use crate::security::tenant::{TenantAccessControl, TenantId, TenantMappingService, TenantResourceVisibility};
use std::sync::Arc;

/// Tenant-aware NetBox client wrapper
/// Ensures all operations are scoped to a specific tenant
pub struct TenantAwareNetBoxClient {
    client: Arc<NetBoxClient>,
    access_control: Arc<TenantAccessControl>,
    visibility: Arc<TenantResourceVisibility>,
}

impl TenantAwareNetBoxClient {
    pub fn new(
        client: Arc<NetBoxClient>,
        access_control: Arc<TenantAccessControl>,
    ) -> Self {
        // Create a new TenantResourceVisibility that shares the same mapping service
        let shared_mapping = Arc::clone(access_control.mapping_service());
        let visibility_access_control = TenantAccessControl {
            mapping_service: shared_mapping,
        };
        let visibility = Arc::new(TenantResourceVisibility::new(visibility_access_control));
        Self {
            client,
            access_control,
            visibility,
        }
    }

    /// Get a site by ID with tenant access control
    pub async fn get_site(&self, tenant_id: &TenantId, site_id: i32) -> Result<NetBoxSite, AppError> {
        let site = self.client.get_site(site_id).await
            .map_err(|e| AppError::Internal(anyhow::Error::from(e)))?;
        
        self.visibility.ensure_site_visible(tenant_id, &site)?;
        Ok(site)
    }

    /// List sites for a tenant (automatically filters by tenant)
    pub async fn list_sites(
        &self,
        tenant_id: &TenantId,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<NetBoxSite>, AppError> {
        // Get NetBox tenant ID for filtering
        let netbox_tenant_id = self.access_control
            .get_netbox_tenant_id(tenant_id)
            .ok_or(AppError::Unauthorized)?;

        // List sites from NetBox with tenant filter
        let response = self.client.list_sites(Some(netbox_tenant_id), limit, offset).await
            .map_err(|e| AppError::Internal(anyhow::Error::from(e)))?;

        // Extract sites and ensure they're all visible to the tenant
        let sites = response.results.unwrap_or_default();
        
        // Double-check visibility (defense in depth)
        let filtered = self.visibility.get_tenant_sites(tenant_id, sites)?;
        Ok(filtered)
    }

    /// Create a site for a tenant (automatically assigns tenant)
    pub async fn create_site(
        &self,
        tenant_id: &TenantId,
        mut request: CreateSiteRequest,
    ) -> Result<NetBoxSite, AppError> {
        // Get NetBox tenant ID
        let netbox_tenant_id = self.access_control
            .get_netbox_tenant_id(tenant_id)
            .ok_or(AppError::Unauthorized)?;

        // Ensure tenant is set in request
        request.tenant = Some(netbox_tenant_id);

        // Create site in NetBox
        let site = self.client.create_site(request).await
            .map_err(|e| AppError::Internal(anyhow::Error::from(e)))?;

        // Verify the created site belongs to the tenant
        self.visibility.ensure_site_visible(tenant_id, &site)?;
        Ok(site)
    }

    /// Update a site with tenant access control
    pub async fn update_site(
        &self,
        tenant_id: &TenantId,
        site_id: i32,
        request: UpdateSiteRequest,
    ) -> Result<NetBoxSite, AppError> {
        // First verify access to the existing site
        let _existing_site = self.get_site(tenant_id, site_id).await?;

        // Update site
        let site = self.client.update_site(site_id, request).await
            .map_err(|e| AppError::Internal(anyhow::Error::from(e)))?;

        // Verify the updated site still belongs to the tenant
        self.visibility.ensure_site_visible(tenant_id, &site)?;
        Ok(site)
    }

    /// Delete a site with tenant access control
    pub async fn delete_site(&self, tenant_id: &TenantId, site_id: i32) -> Result<(), AppError> {
        // Verify access before deletion
        let _site = self.get_site(tenant_id, site_id).await?;

        // Delete site
        self.client.delete_site(site_id).await
            .map_err(|e| AppError::Internal(anyhow::Error::from(e)))?;
        
        Ok(())
    }

    /// Get a device by ID with tenant access control
    pub async fn get_device(&self, tenant_id: &TenantId, device_id: i32) -> Result<NetBoxDevice, AppError> {
        let device = self.client.get_device(device_id).await
            .map_err(|e| AppError::Internal(anyhow::Error::from(e)))?;
        
        self.visibility.ensure_device_visible(tenant_id, &device)?;
        Ok(device)
    }

    /// List devices for a tenant (automatically filters by tenant)
    pub async fn list_devices(
        &self,
        tenant_id: &TenantId,
        site_id: Option<i32>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<NetBoxDevice>, AppError> {
        // Get NetBox tenant ID for filtering
        let netbox_tenant_id = self.access_control
            .get_netbox_tenant_id(tenant_id)
            .ok_or(AppError::Unauthorized)?;

        // List devices from NetBox with tenant filter
        let response = self.client.list_devices(site_id, Some(netbox_tenant_id), limit, offset).await
            .map_err(|e| AppError::Internal(anyhow::Error::from(e)))?;

        // Extract devices and ensure they're all visible to the tenant
        let devices = response.results.unwrap_or_default();
        
        // Double-check visibility (defense in depth)
        let filtered = self.visibility.get_tenant_devices(tenant_id, devices)?;
        Ok(filtered)
    }

    /// Create a device for a tenant (automatically assigns tenant)
    pub async fn create_device(
        &self,
        tenant_id: &TenantId,
        mut request: CreateDeviceRequest,
    ) -> Result<NetBoxDevice, AppError> {
        // Get NetBox tenant ID
        let netbox_tenant_id = self.access_control
            .get_netbox_tenant_id(tenant_id)
            .ok_or(AppError::Unauthorized)?;

        // Ensure tenant is set in request
        request.tenant = Some(netbox_tenant_id);

        // Create device in NetBox
        let device = self.client.create_device(request).await
            .map_err(|e| AppError::Internal(anyhow::Error::from(e)))?;

        // Verify the created device belongs to the tenant
        self.visibility.ensure_device_visible(tenant_id, &device)?;
        Ok(device)
    }

    /// Update a device with tenant access control
    pub async fn update_device(
        &self,
        tenant_id: &TenantId,
        device_id: i32,
        request: UpdateDeviceRequest,
    ) -> Result<NetBoxDevice, AppError> {
        // First verify access to the existing device
        let _existing_device = self.get_device(tenant_id, device_id).await?;

        // Update device
        let device = self.client.update_device(device_id, request).await
            .map_err(|e| AppError::Internal(anyhow::Error::from(e)))?;

        // Verify the updated device still belongs to the tenant
        self.visibility.ensure_device_visible(tenant_id, &device)?;
        Ok(device)
    }

    /// Delete a device with tenant access control
    pub async fn delete_device(&self, tenant_id: &TenantId, device_id: i32) -> Result<(), AppError> {
        // Verify access before deletion
        let _device = self.get_device(tenant_id, device_id).await?;

        // Delete device
        self.client.delete_device(device_id).await
            .map_err(|e| AppError::Internal(anyhow::Error::from(e)))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::netbox::models::{SiteStatus, DeviceStatus};
    use crate::security::tenant::TenantMappingService;
    use serde_json::json;
    use wiremock::{
        matchers::{header, method, path, query_param},
        Mock, MockServer, ResponseTemplate,
    };

    fn create_test_config(base_url: String, token: String) -> Config {
        Config {
            port: 8080,
            netbox_url: base_url,
            netbox_token: token,
        }
    }

    fn setup_tenant_aware_client(
        mock_server: &MockServer,
    ) -> (TenantAwareNetBoxClient, Arc<TenantMappingService>) {
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = Arc::new(NetBoxClient::new(config).unwrap());
        
        let mapping_service = Arc::new(TenantMappingService::new());
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        mapping_service.register_mapping("tenant-2".to_string(), 20);
        
        let access_control = Arc::new(TenantAccessControl {
            mapping_service: Arc::clone(&mapping_service),
        });
        let tenant_client = TenantAwareNetBoxClient::new(client, access_control);
        
        (tenant_client, mapping_service)
    }

    #[tokio::test]
    async fn test_get_site_with_tenant_access_control_success() {
        let mock_server = MockServer::start().await;
        let (client, _) = setup_tenant_aware_client(&mock_server);

        let site_response = json!({
            "id": 1,
            "name": "Test Site",
            "tenant": 10,
            "status": "active"
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&site_response))
            .mount(&mock_server)
            .await;

        let result = client.get_site(&"tenant-1".to_string(), 1).await;
        assert!(result.is_ok());
        let site = result.unwrap();
        assert_eq!(site.id, Some(1));
        assert_eq!(site.tenant, Some(10));
    }

    #[tokio::test]
    async fn test_get_site_with_tenant_access_control_unauthorized() {
        let mock_server = MockServer::start().await;
        let (client, _) = setup_tenant_aware_client(&mock_server);

        let site_response = json!({
            "id": 1,
            "name": "Test Site",
            "tenant": 20, // Different tenant
            "status": "active"
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&site_response))
            .mount(&mock_server)
            .await;

        let result = client.get_site(&"tenant-1".to_string(), 1).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Unauthorized => {}
            _ => panic!("Expected Unauthorized error"),
        }
    }

    #[tokio::test]
    async fn test_list_sites_with_tenant_filter() {
        let mock_server = MockServer::start().await;
        let (client, _) = setup_tenant_aware_client(&mock_server);

        let sites_response = json!({
            "count": 2,
            "results": [
                {
                    "id": 1,
                    "name": "Site 1",
                    "tenant": 10,
                    "status": "active"
                },
                {
                    "id": 2,
                    "name": "Site 2",
                    "tenant": 10,
                    "status": "active"
                }
            ]
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/"))
            .and(query_param("tenant_id", "10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&sites_response))
            .mount(&mock_server)
            .await;

        let result = client.list_sites(&"tenant-1".to_string(), None, None).await;
        assert!(result.is_ok());
        let sites = result.unwrap();
        assert_eq!(sites.len(), 2);
        assert!(sites.iter().all(|s| s.tenant == Some(10)));
    }

    #[tokio::test]
    async fn test_list_sites_filters_out_wrong_tenant() {
        let mock_server = MockServer::start().await;
        let (client, _) = setup_tenant_aware_client(&mock_server);

        // NetBox returns sites from multiple tenants (shouldn't happen with proper filtering, but defense in depth)
        let sites_response = json!({
            "count": 3,
            "results": [
                {
                    "id": 1,
                    "name": "Site 1",
                    "tenant": 10, // tenant-1
                    "status": "active"
                },
                {
                    "id": 2,
                    "name": "Site 2",
                    "tenant": 20, // tenant-2 (should be filtered out)
                    "status": "active"
                },
                {
                    "id": 3,
                    "name": "Site 3",
                    "tenant": 10, // tenant-1
                    "status": "active"
                }
            ]
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/"))
            .and(query_param("tenant_id", "10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&sites_response))
            .mount(&mock_server)
            .await;

        let result = client.list_sites(&"tenant-1".to_string(), None, None).await;
        assert!(result.is_ok());
        let sites = result.unwrap();
        // Should filter out tenant-2's site
        assert_eq!(sites.len(), 2);
        assert!(sites.iter().all(|s| s.tenant == Some(10)));
    }

    #[tokio::test]
    async fn test_create_site_assigns_tenant() {
        let mock_server = MockServer::start().await;
        let (client, _) = setup_tenant_aware_client(&mock_server);

        let site_response = json!({
            "id": 1,
            "name": "New Site",
            "tenant": 10,
            "status": "active"
        });

        Mock::given(method("POST"))
            .and(path("/api/dcim/sites/"))
            .respond_with(ResponseTemplate::new(201).set_body_json(&site_response))
            .mount(&mock_server)
            .await;

        let request = CreateSiteRequest {
            name: "New Site".to_string(),
            description: None,
            slug: None,
            status: Some(SiteStatus::Active),
            region: None,
            tenant: None, // Will be set automatically
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
        };

        let result = client.create_site(&"tenant-1".to_string(), request).await;
        assert!(result.is_ok());
        let site = result.unwrap();
        assert_eq!(site.tenant, Some(10));
    }

    #[tokio::test]
    async fn test_update_site_verifies_access() {
        let mock_server = MockServer::start().await;
        let (client, _) = setup_tenant_aware_client(&mock_server);

        // First GET to verify access
        let existing_site = json!({
            "id": 1,
            "name": "Existing Site",
            "tenant": 10,
            "status": "active"
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&existing_site))
            .mount(&mock_server)
            .await;

        // Then PATCH to update
        let updated_site = json!({
            "id": 1,
            "name": "Updated Site",
            "tenant": 10,
            "status": "active"
        });

        Mock::given(method("PATCH"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&updated_site))
            .mount(&mock_server)
            .await;

        let request = UpdateSiteRequest {
            name: Some("Updated Site".to_string()),
            description: None,
            slug: None,
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
        };

        let result = client.update_site(&"tenant-1".to_string(), 1, request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_site_unauthorized() {
        let mock_server = MockServer::start().await;
        let (client, _) = setup_tenant_aware_client(&mock_server);

        // Site belongs to different tenant
        let existing_site = json!({
            "id": 1,
            "name": "Existing Site",
            "tenant": 20, // tenant-2
            "status": "active"
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&existing_site))
            .mount(&mock_server)
            .await;

        let request = UpdateSiteRequest {
            name: Some("Updated Site".to_string()),
            description: None,
            slug: None,
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
        };

        let result = client.update_site(&"tenant-1".to_string(), 1, request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Unauthorized => {}
            _ => panic!("Expected Unauthorized error"),
        }
    }

    #[tokio::test]
    async fn test_delete_site_verifies_access() {
        let mock_server = MockServer::start().await;
        let (client, _) = setup_tenant_aware_client(&mock_server);

        // First GET to verify access
        let existing_site = json!({
            "id": 1,
            "name": "Site to Delete",
            "tenant": 10,
            "status": "active"
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&existing_site))
            .mount(&mock_server)
            .await;

        // Then DELETE
        Mock::given(method("DELETE"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let result = client.delete_site(&"tenant-1".to_string(), 1).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_device_with_tenant_access_control() {
        let mock_server = MockServer::start().await;
        let (client, _) = setup_tenant_aware_client(&mock_server);

        let device_response = json!({
            "id": 1,
            "name": "Test Device",
            "tenant": 10,
            "status": "active"
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/devices/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&device_response))
            .mount(&mock_server)
            .await;

        let result = client.get_device(&"tenant-1".to_string(), 1).await;
        assert!(result.is_ok());
        let device = result.unwrap();
        assert_eq!(device.id, Some(1));
        assert_eq!(device.tenant, Some(10));
    }

    #[tokio::test]
    async fn test_list_devices_with_tenant_filter() {
        let mock_server = MockServer::start().await;
        let (client, _) = setup_tenant_aware_client(&mock_server);

        let devices_response = json!({
            "count": 2,
            "results": [
                {
                    "id": 1,
                    "name": "Device 1",
                    "tenant": 10,
                    "status": "active"
                },
                {
                    "id": 2,
                    "name": "Device 2",
                    "tenant": 10,
                    "status": "active"
                }
            ]
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/devices/"))
            .and(query_param("tenant_id", "10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&devices_response))
            .mount(&mock_server)
            .await;

        let result = client.list_devices(&"tenant-1".to_string(), None, None, None).await;
        assert!(result.is_ok());
        let devices = result.unwrap();
        assert_eq!(devices.len(), 2);
        assert!(devices.iter().all(|d| d.tenant == Some(10)));
    }

    #[tokio::test]
    async fn test_create_device_assigns_tenant() {
        let mock_server = MockServer::start().await;
        let (client, _) = setup_tenant_aware_client(&mock_server);

        let device_response = json!({
            "id": 1,
            "name": "New Device",
            "tenant": 10,
            "status": "active"
        });

        Mock::given(method("POST"))
            .and(path("/api/dcim/devices/"))
            .respond_with(ResponseTemplate::new(201).set_body_json(&device_response))
            .mount(&mock_server)
            .await;

        let request = CreateDeviceRequest {
            name: Some("New Device".to_string()),
            device_type: 1,
            device_role: 1,
            site: 1,
            tenant: None, // Will be set automatically
            platform: None,
            serial: None,
            asset_tag: None,
            location: None,
            rack: None,
            position: None,
            face: None,
            status: Some(DeviceStatus::Active),
            cluster: None,
            comments: None,
            tags: None,
        };

        let result = client.create_device(&"tenant-1".to_string(), request).await;
        assert!(result.is_ok());
        let device = result.unwrap();
        assert_eq!(device.tenant, Some(10));
    }

    #[tokio::test]
    async fn test_list_sites_no_mapping() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = Arc::new(NetBoxClient::new(config).unwrap());
        
        let mapping_service = TenantMappingService::new();
        let access_control = Arc::new(TenantAccessControl::new(mapping_service));
        let tenant_client = TenantAwareNetBoxClient::new(client, access_control);

        let result = tenant_client.list_sites(&"nonexistent".to_string(), None, None).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Unauthorized => {}
            _ => panic!("Expected Unauthorized error"),
        }
    }

    #[tokio::test]
    async fn test_tenant_isolation_between_tenants() {
        let mock_server = MockServer::start().await;
        let (client, _) = setup_tenant_aware_client(&mock_server);

        // tenant-1 tries to access tenant-2's site
        let site_response = json!({
            "id": 1,
            "name": "Tenant 2 Site",
            "tenant": 20, // tenant-2
            "status": "active"
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&site_response))
            .mount(&mock_server)
            .await;

        let result = client.get_site(&"tenant-1".to_string(), 1).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Unauthorized => {}
            _ => panic!("Expected Unauthorized error"),
        }
    }
}

