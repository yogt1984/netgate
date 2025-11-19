use crate::config::Config;
use crate::netbox::error::NetBoxError;
use crate::netbox::models::*;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use std::fmt::Write;
use tracing::{debug, error};

/// NetBox API Client
pub struct NetBoxClient {
    base_url: String,
    token: String,
    client: reqwest::Client,
}

impl NetBoxClient {
    /// Create a new NetBox client
    pub fn new(config: Config) -> Result<Self, NetBoxError> {
        let base_url = config.netbox_url.trim_end_matches('/').to_string();
        let token = config.netbox_token;

        if token.is_empty() {
            return Err(NetBoxError::AuthenticationError(
                "NetBox token is required".to_string(),
            ));
        }

        let mut headers = HeaderMap::new();
        let auth_value = format!("Token {}", token);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value).map_err(|e| {
                NetBoxError::AuthenticationError(format!("Invalid token format: {}", e))
            })?,
        );
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/json"),
        );
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| NetBoxError::NetworkError(e))?;

        Ok(Self {
            base_url,
            token,
            client,
        })
    }

    /// Build URL for a NetBox API endpoint
    fn build_url(&self, endpoint: &str) -> Result<String, NetBoxError> {
        let mut url = self.base_url.clone();
        if !endpoint.starts_with('/') {
            write!(url, "/api/{}", endpoint).map_err(|e| {
                NetBoxError::InvalidUrl(format!("Failed to build URL: {}", e))
            })?;
        } else {
            write!(url, "/api{}", endpoint).map_err(|e| {
                NetBoxError::InvalidUrl(format!("Failed to build URL: {}", e))
            })?;
        }
        Ok(url)
    }

    // ========== Site CRUD Operations ==========

    /// Create a new site in NetBox
    pub async fn create_site(&self, request: CreateSiteRequest) -> Result<NetBoxSite, NetBoxError> {
        let url = self.build_url("dcim/sites/")?;
        debug!("Creating site in NetBox: {}", url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| NetBoxError::NetworkError(e))?;

        let status = response.status();
        let text = response.text().await.map_err(|e| NetBoxError::NetworkError(e))?;

        if !status.is_success() {
            error!("NetBox API error: {} - {}", status, text);
            return Err(NetBoxError::from_status_code(status.as_u16(), text));
        }

        serde_json::from_str(&text).map_err(|e| NetBoxError::SerializationError(e))
    }

    /// Get a site by ID
    pub async fn get_site(&self, id: i32) -> Result<NetBoxSite, NetBoxError> {
        let url = self.build_url(&format!("dcim/sites/{}/", id))?;
        debug!("Getting site from NetBox: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| NetBoxError::NetworkError(e))?;

        let status = response.status();
        let text = response.text().await.map_err(|e| NetBoxError::NetworkError(e))?;

        if !status.is_success() {
            if status == 404 {
                return Err(NetBoxError::NotFound(format!("Site with ID {} not found", id)));
            }
            error!("NetBox API error: {} - {}", status, text);
            return Err(NetBoxError::from_status_code(status.as_u16(), text));
        }

        serde_json::from_str(&text).map_err(|e| NetBoxError::SerializationError(e))
    }

    /// List sites with optional filters
    pub async fn list_sites(
        &self,
        tenant_id: Option<i32>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<NetBoxResponse<NetBoxSite>, NetBoxError> {
        let mut url = self.build_url("dcim/sites/")?;
        
        let mut params = Vec::new();
        if let Some(tenant) = tenant_id {
            params.push(("tenant_id", tenant.to_string()));
        }
        if let Some(lim) = limit {
            params.push(("limit", lim.to_string()));
        }
        if let Some(off) = offset {
            params.push(("offset", off.to_string()));
        }

        if !params.is_empty() {
            let query_string: String = params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            write!(url, "?{}", query_string).map_err(|e| {
                NetBoxError::InvalidUrl(format!("Failed to build query: {}", e))
            })?;
        }

        debug!("Listing sites from NetBox: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| NetBoxError::NetworkError(e))?;

        let status = response.status();
        let text = response.text().await.map_err(|e| NetBoxError::NetworkError(e))?;

        if !status.is_success() {
            error!("NetBox API error: {} - {}", status, text);
            return Err(NetBoxError::from_status_code(status.as_u16(), text));
        }

        serde_json::from_str(&text).map_err(|e| NetBoxError::SerializationError(e))
    }

    /// Update a site
    pub async fn update_site(
        &self,
        id: i32,
        request: UpdateSiteRequest,
    ) -> Result<NetBoxSite, NetBoxError> {
        let url = self.build_url(&format!("dcim/sites/{}/", id))?;
        debug!("Updating site in NetBox: {}", url);

        let response = self
            .client
            .patch(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| NetBoxError::NetworkError(e))?;

        let status = response.status();
        let text = response.text().await.map_err(|e| NetBoxError::NetworkError(e))?;

        if !status.is_success() {
            if status == 404 {
                return Err(NetBoxError::NotFound(format!("Site with ID {} not found", id)));
            }
            error!("NetBox API error: {} - {}", status, text);
            return Err(NetBoxError::from_status_code(status.as_u16(), text));
        }

        serde_json::from_str(&text).map_err(|e| NetBoxError::SerializationError(e))
    }

    /// Delete a site
    pub async fn delete_site(&self, id: i32) -> Result<(), NetBoxError> {
        let url = self.build_url(&format!("dcim/sites/{}/", id))?;
        debug!("Deleting site from NetBox: {}", url);

        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| NetBoxError::NetworkError(e))?;

        let status = response.status();

        if !status.is_success() {
            if status == 404 {
                return Err(NetBoxError::NotFound(format!("Site with ID {} not found", id)));
            }
            let text = response.text().await.unwrap_or_default();
            error!("NetBox API error: {} - {}", status, text);
            return Err(NetBoxError::from_status_code(status.as_u16(), text));
        }

        Ok(())
    }

    // ========== Device CRUD Operations ==========

    /// Create a new device in NetBox
    pub async fn create_device(
        &self,
        request: CreateDeviceRequest,
    ) -> Result<NetBoxDevice, NetBoxError> {
        let url = self.build_url("dcim/devices/")?;
        debug!("Creating device in NetBox: {}", url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| NetBoxError::NetworkError(e))?;

        let status = response.status();
        let text = response.text().await.map_err(|e| NetBoxError::NetworkError(e))?;

        if !status.is_success() {
            error!("NetBox API error: {} - {}", status, text);
            return Err(NetBoxError::from_status_code(status.as_u16(), text));
        }

        serde_json::from_str(&text).map_err(|e| NetBoxError::SerializationError(e))
    }

    /// Get a device by ID
    pub async fn get_device(&self, id: i32) -> Result<NetBoxDevice, NetBoxError> {
        let url = self.build_url(&format!("dcim/devices/{}/", id))?;
        debug!("Getting device from NetBox: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| NetBoxError::NetworkError(e))?;

        let status = response.status();
        let text = response.text().await.map_err(|e| NetBoxError::NetworkError(e))?;

        if !status.is_success() {
            if status == 404 {
                return Err(NetBoxError::NotFound(format!("Device with ID {} not found", id)));
            }
            error!("NetBox API error: {} - {}", status, text);
            return Err(NetBoxError::from_status_code(status.as_u16(), text));
        }

        serde_json::from_str(&text).map_err(|e| NetBoxError::SerializationError(e))
    }

    /// List devices with optional filters
    pub async fn list_devices(
        &self,
        site_id: Option<i32>,
        tenant_id: Option<i32>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<NetBoxResponse<NetBoxDevice>, NetBoxError> {
        let mut url = self.build_url("dcim/devices/")?;
        
        let mut params = Vec::new();
        if let Some(site) = site_id {
            params.push(("site_id", site.to_string()));
        }
        if let Some(tenant) = tenant_id {
            params.push(("tenant_id", tenant.to_string()));
        }
        if let Some(lim) = limit {
            params.push(("limit", lim.to_string()));
        }
        if let Some(off) = offset {
            params.push(("offset", off.to_string()));
        }

        if !params.is_empty() {
            let query_string: String = params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            write!(url, "?{}", query_string).map_err(|e| {
                NetBoxError::InvalidUrl(format!("Failed to build query: {}", e))
            })?;
        }

        debug!("Listing devices from NetBox: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| NetBoxError::NetworkError(e))?;

        let status = response.status();
        let text = response.text().await.map_err(|e| NetBoxError::NetworkError(e))?;

        if !status.is_success() {
            error!("NetBox API error: {} - {}", status, text);
            return Err(NetBoxError::from_status_code(status.as_u16(), text));
        }

        serde_json::from_str(&text).map_err(|e| NetBoxError::SerializationError(e))
    }

    /// Update a device
    pub async fn update_device(
        &self,
        id: i32,
        request: UpdateDeviceRequest,
    ) -> Result<NetBoxDevice, NetBoxError> {
        let url = self.build_url(&format!("dcim/devices/{}/", id))?;
        debug!("Updating device in NetBox: {}", url);

        let response = self
            .client
            .patch(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| NetBoxError::NetworkError(e))?;

        let status = response.status();
        let text = response.text().await.map_err(|e| NetBoxError::NetworkError(e))?;

        if !status.is_success() {
            if status == 404 {
                return Err(NetBoxError::NotFound(format!("Device with ID {} not found", id)));
            }
            error!("NetBox API error: {} - {}", status, text);
            return Err(NetBoxError::from_status_code(status.as_u16(), text));
        }

        serde_json::from_str(&text).map_err(|e| NetBoxError::SerializationError(e))
    }

    /// Delete a device
    pub async fn delete_device(&self, id: i32) -> Result<(), NetBoxError> {
        let url = self.build_url(&format!("dcim/devices/{}/", id))?;
        debug!("Deleting device from NetBox: {}", url);

        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| NetBoxError::NetworkError(e))?;

        let status = response.status();

        if !status.is_success() {
            if status == 404 {
                return Err(NetBoxError::NotFound(format!("Device with ID {} not found", id)));
            }
            let text = response.text().await.unwrap_or_default();
            error!("NetBox API error: {} - {}", status, text);
            return Err(NetBoxError::from_status_code(status.as_u16(), text));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use serde_json::json;
    use wiremock::{
        matchers::{header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    fn create_test_config(base_url: String, token: String) -> Config {
        Config {
            port: 8080,
            netbox_url: base_url,
            netbox_token: token,
        }
    }

    #[tokio::test]
    async fn test_client_creation_success() {
        let config = create_test_config("http://localhost:8000".to_string(), "test-token".to_string());
        let result = NetBoxClient::new(config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_client_creation_no_token() {
        let config = create_test_config("http://localhost:8000".to_string(), "".to_string());
        let result = NetBoxClient::new(config);
        assert!(result.is_err());
        if let Err(NetBoxError::AuthenticationError(_)) = result {
            // Expected error
        } else {
            panic!("Expected AuthenticationError");
        }
    }

    #[tokio::test]
    async fn test_create_site_success() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = NetBoxClient::new(config).unwrap();

        let site_response = json!({
            "id": 1,
            "name": "Test Site",
            "description": "Test Description",
            "status": "active"
        });

        Mock::given(method("POST"))
            .and(path("/api/dcim/sites/"))
            .and(header("Authorization", "Token test-token"))
            .respond_with(ResponseTemplate::new(201).set_body_json(&site_response))
            .mount(&mock_server)
            .await;

        let request = CreateSiteRequest {
            name: "Test Site".to_string(),
            description: Some("Test Description".to_string()),
            slug: None,
            status: Some(SiteStatus::Active),
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

        let result = client.create_site(request).await;
        assert!(result.is_ok());
        let site = result.unwrap();
        assert_eq!(site.name, "Test Site");
        assert_eq!(site.description, Some("Test Description".to_string()));
        assert_eq!(site.id, Some(1));
    }

    #[tokio::test]
    async fn test_create_site_authentication_error() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = NetBoxClient::new(config).unwrap();

        Mock::given(method("POST"))
            .and(path("/api/dcim/sites/"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "detail": "Invalid token"
            })))
            .mount(&mock_server)
            .await;

        let request = CreateSiteRequest {
            name: "Test Site".to_string(),
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

        let result = client.create_site(request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            NetBoxError::AuthenticationError(_) => {}
            _ => panic!("Expected AuthenticationError"),
        }
    }

    #[tokio::test]
    async fn test_get_site_success() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = NetBoxClient::new(config).unwrap();

        let site_response = json!({
            "id": 1,
            "name": "Test Site",
            "description": "Test Description",
            "status": "active"
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/1/"))
            .and(header("Authorization", "Token test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&site_response))
            .mount(&mock_server)
            .await;

        let result = client.get_site(1).await;
        assert!(result.is_ok());
        let site = result.unwrap();
        assert_eq!(site.id, Some(1));
        assert_eq!(site.name, "Test Site");
    }

    #[tokio::test]
    async fn test_get_site_not_found() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = NetBoxClient::new(config).unwrap();

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/999/"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({
                "detail": "Not found"
            })))
            .mount(&mock_server)
            .await;

        let result = client.get_site(999).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            NetBoxError::NotFound(_) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_list_sites_success() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = NetBoxClient::new(config).unwrap();

        let sites_response = json!({
            "count": 2,
            "next": null,
            "previous": null,
            "results": [
                {
                    "id": 1,
                    "name": "Site 1",
                    "status": "active"
                },
                {
                    "id": 2,
                    "name": "Site 2",
                    "status": "active"
                }
            ]
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&sites_response))
            .mount(&mock_server)
            .await;

        let result = client.list_sites(None, None, None).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.count, Some(2));
        assert_eq!(response.results.as_ref().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_list_sites_with_tenant_filter() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = NetBoxClient::new(config).unwrap();

        let sites_response = json!({
            "count": 1,
            "results": [
                {
                    "id": 1,
                    "name": "Tenant Site",
                    "tenant": 10,
                    "status": "active"
                }
            ]
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/sites/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&sites_response))
            .mount(&mock_server)
            .await;

        let result = client.list_sites(Some(10), None, None).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.count, Some(1));
        assert_eq!(response.results.as_ref().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_update_site_success() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = NetBoxClient::new(config).unwrap();

        let site_response = json!({
            "id": 1,
            "name": "Updated Site",
            "description": "Updated Description",
            "status": "active"
        });

        Mock::given(method("PATCH"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&site_response))
            .mount(&mock_server)
            .await;

        let request = UpdateSiteRequest {
            name: Some("Updated Site".to_string()),
            description: Some("Updated Description".to_string()),
            slug: None,
            status: Some(SiteStatus::Active),
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

        let result = client.update_site(1, request).await;
        assert!(result.is_ok());
        let site = result.unwrap();
        assert_eq!(site.name, "Updated Site");
        assert_eq!(site.description, Some("Updated Description".to_string()));
    }

    #[tokio::test]
    async fn test_delete_site_success() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = NetBoxClient::new(config).unwrap();

        Mock::given(method("DELETE"))
            .and(path("/api/dcim/sites/1/"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let result = client.delete_site(1).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_site_not_found() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = NetBoxClient::new(config).unwrap();

        Mock::given(method("DELETE"))
            .and(path("/api/dcim/sites/999/"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({
                "detail": "Not found"
            })))
            .mount(&mock_server)
            .await;

        let result = client.delete_site(999).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            NetBoxError::NotFound(_) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_create_device_success() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = NetBoxClient::new(config).unwrap();

        let device_response = json!({
            "id": 1,
            "name": "test-device",
            "device_type": 1,
            "device_role": 1,
            "site": 1,
            "status": "active"
        });

        Mock::given(method("POST"))
            .and(path("/api/dcim/devices/"))
            .respond_with(ResponseTemplate::new(201).set_body_json(&device_response))
            .mount(&mock_server)
            .await;

        let request = CreateDeviceRequest {
            name: Some("test-device".to_string()),
            device_type: 1,
            device_role: 1,
            site: 1,
            tenant: None,
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

        let result = client.create_device(request).await;
        assert!(result.is_ok());
        let device = result.unwrap();
        assert_eq!(device.id, Some(1));
        assert_eq!(device.name, Some("test-device".to_string()));
    }

    #[tokio::test]
    async fn test_get_device_success() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = NetBoxClient::new(config).unwrap();

        let device_response = json!({
            "id": 1,
            "name": "test-device",
            "device_type": 1,
            "device_role": 1,
            "site": 1,
            "status": "active"
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/devices/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&device_response))
            .mount(&mock_server)
            .await;

        let result = client.get_device(1).await;
        assert!(result.is_ok());
        let device = result.unwrap();
        assert_eq!(device.id, Some(1));
        assert_eq!(device.name, Some("test-device".to_string()));
    }

    #[tokio::test]
    async fn test_list_devices_with_filters() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = NetBoxClient::new(config).unwrap();

        let devices_response = json!({
            "count": 1,
            "results": [
                {
                    "id": 1,
                    "name": "device-1",
                    "site": 1,
                    "tenant": 10,
                    "status": "active"
                }
            ]
        });

        Mock::given(method("GET"))
            .and(path("/api/dcim/devices/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&devices_response))
            .mount(&mock_server)
            .await;

        let result = client.list_devices(Some(1), Some(10), None, None).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.count, Some(1));
        assert_eq!(response.results.as_ref().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_update_device_success() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = NetBoxClient::new(config).unwrap();

        let device_response = json!({
            "id": 1,
            "name": "updated-device",
            "status": "offline"
        });

        Mock::given(method("PATCH"))
            .and(path("/api/dcim/devices/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&device_response))
            .mount(&mock_server)
            .await;

        let request = UpdateDeviceRequest {
            name: Some("updated-device".to_string()),
            status: Some(DeviceStatus::Offline),
            device_type: None,
            device_role: None,
            tenant: None,
            platform: None,
            serial: None,
            asset_tag: None,
            site: None,
            location: None,
            rack: None,
            position: None,
            face: None,
            cluster: None,
            comments: None,
            tags: None,
        };

        let result = client.update_device(1, request).await;
        assert!(result.is_ok());
        let device = result.unwrap();
        assert_eq!(device.name, Some("updated-device".to_string()));
    }

    #[tokio::test]
    async fn test_delete_device_success() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = NetBoxClient::new(config).unwrap();

        Mock::given(method("DELETE"))
            .and(path("/api/dcim/devices/1/"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let result = client.delete_device(1).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validation_error() {
        let mock_server = MockServer::start().await;
        let config = create_test_config(mock_server.uri(), "test-token".to_string());
        let client = NetBoxClient::new(config).unwrap();

        Mock::given(method("POST"))
            .and(path("/api/dcim/sites/"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "name": ["This field is required."]
            })))
            .mount(&mock_server)
            .await;

        let request = CreateSiteRequest {
            name: "".to_string(), // Invalid: empty name
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

        let result = client.create_site(request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            NetBoxError::ValidationError(_) => {}
            _ => panic!("Expected ValidationError"),
        }
    }
}
