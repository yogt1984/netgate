use std::collections::HashMap;
use std::sync::RwLock;
use crate::error::AppError;
use crate::netbox::models::{NetBoxSite, NetBoxDevice};

/// Tenant ID type alias
pub type TenantId = String;

/// NetBox tenant ID type
pub type NetBoxTenantId = i32;

/// Tenant mapping service - maps application tenant IDs to NetBox tenant IDs
pub struct TenantMappingService {
    // Map from application tenant ID (string) to NetBox tenant ID (i32)
    mappings: RwLock<HashMap<TenantId, NetBoxTenantId>>,
}

impl TenantMappingService {
    pub fn new() -> Self {
        Self {
            mappings: RwLock::new(HashMap::new()),
        }
    }

    /// Register a mapping between application tenant ID and NetBox tenant ID
    pub fn register_mapping(&self, tenant_id: TenantId, netbox_tenant_id: NetBoxTenantId) {
        let mut mappings = self.mappings.write().unwrap();
        mappings.insert(tenant_id, netbox_tenant_id);
    }

    /// Get NetBox tenant ID for an application tenant ID
    pub fn get_netbox_tenant_id(&self, tenant_id: &TenantId) -> Option<NetBoxTenantId> {
        let mappings = self.mappings.read().unwrap();
        mappings.get(tenant_id).copied()
    }

    /// Check if a tenant mapping exists
    pub fn has_mapping(&self, tenant_id: &TenantId) -> bool {
        let mappings = self.mappings.read().unwrap();
        mappings.contains_key(tenant_id)
    }

    /// Remove a tenant mapping
    pub fn remove_mapping(&self, tenant_id: &TenantId) {
        let mut mappings = self.mappings.write().unwrap();
        mappings.remove(tenant_id);
    }

    /// Get all registered tenant IDs
    pub fn get_all_tenant_ids(&self) -> Vec<TenantId> {
        let mappings = self.mappings.read().unwrap();
        mappings.keys().cloned().collect()
    }
}

impl Default for TenantMappingService {
    fn default() -> Self {
        Self::new()
    }
}

/// Tenant access control service
pub struct TenantAccessControl {
    pub(crate) mapping_service: std::sync::Arc<TenantMappingService>,
}

impl TenantAccessControl {
    pub fn new(mapping_service: TenantMappingService) -> Self {
        Self { 
            mapping_service: std::sync::Arc::new(mapping_service),
        }
    }

    /// Get a reference to the underlying mapping service
    pub fn mapping_service(&self) -> &std::sync::Arc<TenantMappingService> {
        &self.mapping_service
    }

    /// Verify that a NetBox site belongs to the specified tenant
    pub fn verify_site_access(&self, tenant_id: &TenantId, site: &NetBoxSite) -> Result<(), AppError> {
        let netbox_tenant_id = self.mapping_service
            .get_netbox_tenant_id(tenant_id)
            .ok_or_else(|| AppError::Unauthorized)?;

        // Check if site's tenant matches
        if let Some(site_tenant) = site.tenant {
            if site_tenant == netbox_tenant_id {
                Ok(())
            } else {
                Err(AppError::Unauthorized)
            }
        } else {
            // Site has no tenant assigned - deny access
            Err(AppError::Unauthorized)
        }
    }

    /// Verify that a NetBox device belongs to the specified tenant
    pub fn verify_device_access(&self, tenant_id: &TenantId, device: &NetBoxDevice) -> Result<(), AppError> {
        let netbox_tenant_id = self.mapping_service
            .get_netbox_tenant_id(tenant_id)
            .ok_or_else(|| AppError::Unauthorized)?;

        // Check if device's tenant matches
        if let Some(device_tenant) = device.tenant {
            if device_tenant == netbox_tenant_id {
                Ok(())
            } else {
                Err(AppError::Unauthorized)
            }
        } else {
            // Device has no tenant assigned - deny access
            Err(AppError::Unauthorized)
        }
    }

    /// Get NetBox tenant ID for filtering
    pub fn get_netbox_tenant_id(&self, tenant_id: &TenantId) -> Option<NetBoxTenantId> {
        self.mapping_service.get_netbox_tenant_id(tenant_id)
    }

    /// Filter sites by tenant - returns only sites that belong to the tenant
    pub fn filter_sites_by_tenant(
        &self,
        tenant_id: &TenantId,
        sites: Vec<NetBoxSite>,
    ) -> Result<Vec<NetBoxSite>, AppError> {
        let netbox_tenant_id = self.mapping_service
            .get_netbox_tenant_id(tenant_id)
            .ok_or_else(|| AppError::Unauthorized)?;

        let filtered: Vec<NetBoxSite> = sites
            .into_iter()
            .filter(|site| {
                site.tenant.map(|t| t == netbox_tenant_id).unwrap_or(false)
            })
            .collect();

        Ok(filtered)
    }

    /// Filter devices by tenant - returns only devices that belong to the tenant
    pub fn filter_devices_by_tenant(
        &self,
        tenant_id: &TenantId,
        devices: Vec<NetBoxDevice>,
    ) -> Result<Vec<NetBoxDevice>, AppError> {
        let netbox_tenant_id = self.mapping_service
            .get_netbox_tenant_id(tenant_id)
            .ok_or_else(|| AppError::Unauthorized)?;

        let filtered: Vec<NetBoxDevice> = devices
            .into_iter()
            .filter(|device| {
                device.tenant.map(|t| t == netbox_tenant_id).unwrap_or(false)
            })
            .collect();

        Ok(filtered)
    }

    /// Check if tenant has access to a resource (by NetBox tenant ID)
    pub fn has_access_to_netbox_tenant(&self, tenant_id: &TenantId, netbox_tenant_id: NetBoxTenantId) -> bool {
        self.mapping_service
            .get_netbox_tenant_id(tenant_id)
            .map(|t| t == netbox_tenant_id)
            .unwrap_or(false)
    }
}

/// Tenant-scoped resource visibility service
pub struct TenantResourceVisibility {
    access_control: TenantAccessControl,
}

impl TenantResourceVisibility {
    pub fn new(access_control: TenantAccessControl) -> Self {
        Self { access_control }
    }

    /// Ensure a site is visible to the tenant (throws error if not)
    pub fn ensure_site_visible(&self, tenant_id: &TenantId, site: &NetBoxSite) -> Result<(), AppError> {
        self.access_control.verify_site_access(tenant_id, site)
    }

    /// Ensure a device is visible to the tenant (throws error if not)
    pub fn ensure_device_visible(&self, tenant_id: &TenantId, device: &NetBoxDevice) -> Result<(), AppError> {
        self.access_control.verify_device_access(tenant_id, device)
    }

    /// Get tenant-scoped sites (filters and validates)
    pub fn get_tenant_sites(
        &self,
        tenant_id: &TenantId,
        sites: Vec<NetBoxSite>,
    ) -> Result<Vec<NetBoxSite>, AppError> {
        self.access_control.filter_sites_by_tenant(tenant_id, sites)
    }

    /// Get tenant-scoped devices (filters and validates)
    pub fn get_tenant_devices(
        &self,
        tenant_id: &TenantId,
        devices: Vec<NetBoxDevice>,
    ) -> Result<Vec<NetBoxDevice>, AppError> {
        self.access_control.filter_devices_by_tenant(tenant_id, devices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netbox::models::{SiteStatus, DeviceStatus};

    fn create_test_site(id: i32, tenant_id: Option<i32>) -> NetBoxSite {
        NetBoxSite {
            id: Some(id),
            name: format!("Site {}", id),
            tenant: tenant_id,
            status: Some(SiteStatus::Active),
            ..Default::default()
        }
    }

    fn create_test_device(id: i32, tenant_id: Option<i32>) -> NetBoxDevice {
        NetBoxDevice {
            id: Some(id),
            name: Some(format!("Device {}", id)),
            tenant: tenant_id,
            status: Some(DeviceStatus::Active),
            ..Default::default()
        }
    }

    // ========== TenantMappingService Tests ==========

    #[test]
    fn test_tenant_mapping_service_register() {
        let service = TenantMappingService::new();
        service.register_mapping("tenant-1".to_string(), 10);
        
        assert!(service.has_mapping(&"tenant-1".to_string()));
        assert_eq!(service.get_netbox_tenant_id(&"tenant-1".to_string()), Some(10));
    }

    #[test]
    fn test_tenant_mapping_service_get_nonexistent() {
        let service = TenantMappingService::new();
        assert!(service.get_netbox_tenant_id(&"nonexistent".to_string()).is_none());
        assert!(!service.has_mapping(&"nonexistent".to_string()));
    }

    #[test]
    fn test_tenant_mapping_service_remove() {
        let service = TenantMappingService::new();
        service.register_mapping("tenant-1".to_string(), 10);
        assert!(service.has_mapping(&"tenant-1".to_string()));
        
        service.remove_mapping(&"tenant-1".to_string());
        assert!(!service.has_mapping(&"tenant-1".to_string()));
    }

    #[test]
    fn test_tenant_mapping_service_multiple_tenants() {
        let service = TenantMappingService::new();
        service.register_mapping("tenant-1".to_string(), 10);
        service.register_mapping("tenant-2".to_string(), 20);
        service.register_mapping("tenant-3".to_string(), 30);
        
        assert_eq!(service.get_netbox_tenant_id(&"tenant-1".to_string()), Some(10));
        assert_eq!(service.get_netbox_tenant_id(&"tenant-2".to_string()), Some(20));
        assert_eq!(service.get_netbox_tenant_id(&"tenant-3".to_string()), Some(30));
    }

    #[test]
    fn test_tenant_mapping_service_get_all_tenant_ids() {
        let service = TenantMappingService::new();
        service.register_mapping("tenant-1".to_string(), 10);
        service.register_mapping("tenant-2".to_string(), 20);
        
        let all_ids = service.get_all_tenant_ids();
        assert_eq!(all_ids.len(), 2);
        assert!(all_ids.contains(&"tenant-1".to_string()));
        assert!(all_ids.contains(&"tenant-2".to_string()));
    }

    #[test]
    fn test_tenant_mapping_service_default() {
        let service = TenantMappingService::default();
        assert!(!service.has_mapping(&"tenant-1".to_string()));
    }

    #[test]
    fn test_tenant_mapping_service_update_mapping() {
        let service = TenantMappingService::new();
        service.register_mapping("tenant-1".to_string(), 10);
        assert_eq!(service.get_netbox_tenant_id(&"tenant-1".to_string()), Some(10));
        
        // Update mapping
        service.register_mapping("tenant-1".to_string(), 20);
        assert_eq!(service.get_netbox_tenant_id(&"tenant-1".to_string()), Some(20));
    }

    // ========== TenantAccessControl Tests ==========

    #[test]
    fn test_verify_site_access_success() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        
        let access_control = TenantAccessControl::new(mapping_service);
        let site = create_test_site(1, Some(10));
        
        assert!(access_control.verify_site_access(&"tenant-1".to_string(), &site).is_ok());
    }

    #[test]
    fn test_verify_site_access_wrong_tenant() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        
        let access_control = TenantAccessControl::new(mapping_service);
        let site = create_test_site(1, Some(20)); // Different tenant
        
        assert!(access_control.verify_site_access(&"tenant-1".to_string(), &site).is_err());
    }

    #[test]
    fn test_verify_site_access_no_tenant_on_site() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        
        let access_control = TenantAccessControl::new(mapping_service);
        let site = create_test_site(1, None); // No tenant assigned
        
        assert!(access_control.verify_site_access(&"tenant-1".to_string(), &site).is_err());
    }

    #[test]
    fn test_verify_site_access_no_mapping() {
        let mapping_service = TenantMappingService::new();
        let access_control = TenantAccessControl::new(mapping_service);
        let site = create_test_site(1, Some(10));
        
        assert!(access_control.verify_site_access(&"nonexistent".to_string(), &site).is_err());
    }

    #[test]
    fn test_verify_device_access_success() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        
        let access_control = TenantAccessControl::new(mapping_service);
        let device = create_test_device(1, Some(10));
        
        assert!(access_control.verify_device_access(&"tenant-1".to_string(), &device).is_ok());
    }

    #[test]
    fn test_verify_device_access_wrong_tenant() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        
        let access_control = TenantAccessControl::new(mapping_service);
        let device = create_test_device(1, Some(20)); // Different tenant
        
        assert!(access_control.verify_device_access(&"tenant-1".to_string(), &device).is_err());
    }

    #[test]
    fn test_verify_device_access_no_tenant_on_device() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        
        let access_control = TenantAccessControl::new(mapping_service);
        let device = create_test_device(1, None); // No tenant assigned
        
        assert!(access_control.verify_device_access(&"tenant-1".to_string(), &device).is_err());
    }

    #[test]
    fn test_filter_sites_by_tenant() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        mapping_service.register_mapping("tenant-2".to_string(), 20);
        
        let access_control = TenantAccessControl::new(mapping_service);
        
        let sites = vec![
            create_test_site(1, Some(10)), // tenant-1
            create_test_site(2, Some(20)), // tenant-2
            create_test_site(3, Some(10)), // tenant-1
            create_test_site(4, None),     // no tenant
        ];
        
        let filtered = access_control.filter_sites_by_tenant(&"tenant-1".to_string(), sites).unwrap();
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].id, Some(1));
        assert_eq!(filtered[1].id, Some(3));
    }

    #[test]
    fn test_filter_sites_by_tenant_empty_result() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        
        let access_control = TenantAccessControl::new(mapping_service);
        
        let sites = vec![
            create_test_site(1, Some(20)), // Different tenant
            create_test_site(2, Some(30)), // Different tenant
        ];
        
        let filtered = access_control.filter_sites_by_tenant(&"tenant-1".to_string(), sites).unwrap();
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_sites_by_tenant_no_mapping() {
        let mapping_service = TenantMappingService::new();
        let access_control = TenantAccessControl::new(mapping_service);
        
        let sites = vec![create_test_site(1, Some(10))];
        
        assert!(access_control.filter_sites_by_tenant(&"nonexistent".to_string(), sites).is_err());
    }

    #[test]
    fn test_filter_devices_by_tenant() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        mapping_service.register_mapping("tenant-2".to_string(), 20);
        
        let access_control = TenantAccessControl::new(mapping_service);
        
        let devices = vec![
            create_test_device(1, Some(10)), // tenant-1
            create_test_device(2, Some(20)), // tenant-2
            create_test_device(3, Some(10)), // tenant-1
            create_test_device(4, None),     // no tenant
        ];
        
        let filtered = access_control.filter_devices_by_tenant(&"tenant-1".to_string(), devices).unwrap();
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].id, Some(1));
        assert_eq!(filtered[1].id, Some(3));
    }

    #[test]
    fn test_filter_devices_by_tenant_empty_result() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        
        let access_control = TenantAccessControl::new(mapping_service);
        
        let devices = vec![
            create_test_device(1, Some(20)), // Different tenant
            create_test_device(2, Some(30)), // Different tenant
        ];
        
        let filtered = access_control.filter_devices_by_tenant(&"tenant-1".to_string(), devices).unwrap();
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_has_access_to_netbox_tenant() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        
        let access_control = TenantAccessControl::new(mapping_service);
        
        assert!(access_control.has_access_to_netbox_tenant(&"tenant-1".to_string(), 10));
        assert!(!access_control.has_access_to_netbox_tenant(&"tenant-1".to_string(), 20));
        assert!(!access_control.has_access_to_netbox_tenant(&"nonexistent".to_string(), 10));
    }

    #[test]
    fn test_get_netbox_tenant_id() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        
        let access_control = TenantAccessControl::new(mapping_service);
        
        assert_eq!(access_control.get_netbox_tenant_id(&"tenant-1".to_string()), Some(10));
        assert_eq!(access_control.get_netbox_tenant_id(&"nonexistent".to_string()), None);
    }

    // ========== TenantResourceVisibility Tests ==========

    #[test]
    fn test_ensure_site_visible_success() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        let access_control = TenantAccessControl::new(mapping_service);
        let visibility = TenantResourceVisibility::new(access_control);
        
        let site = create_test_site(1, Some(10));
        assert!(visibility.ensure_site_visible(&"tenant-1".to_string(), &site).is_ok());
    }

    #[test]
    fn test_ensure_site_visible_failure() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        let access_control = TenantAccessControl::new(mapping_service);
        let visibility = TenantResourceVisibility::new(access_control);
        
        let site = create_test_site(1, Some(20)); // Wrong tenant
        assert!(visibility.ensure_site_visible(&"tenant-1".to_string(), &site).is_err());
    }

    #[test]
    fn test_ensure_device_visible_success() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        let access_control = TenantAccessControl::new(mapping_service);
        let visibility = TenantResourceVisibility::new(access_control);
        
        let device = create_test_device(1, Some(10));
        assert!(visibility.ensure_device_visible(&"tenant-1".to_string(), &device).is_ok());
    }

    #[test]
    fn test_ensure_device_visible_failure() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        let access_control = TenantAccessControl::new(mapping_service);
        let visibility = TenantResourceVisibility::new(access_control);
        
        let device = create_test_device(1, Some(20)); // Wrong tenant
        assert!(visibility.ensure_device_visible(&"tenant-1".to_string(), &device).is_err());
    }

    #[test]
    fn test_get_tenant_sites() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        let access_control = TenantAccessControl::new(mapping_service);
        let visibility = TenantResourceVisibility::new(access_control);
        
        let sites = vec![
            create_test_site(1, Some(10)), // tenant-1
            create_test_site(2, Some(20)), // tenant-2
            create_test_site(3, Some(10)), // tenant-1
        ];
        
        let filtered = visibility.get_tenant_sites(&"tenant-1".to_string(), sites).unwrap();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_get_tenant_devices() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        let access_control = TenantAccessControl::new(mapping_service);
        let visibility = TenantResourceVisibility::new(access_control);
        
        let devices = vec![
            create_test_device(1, Some(10)), // tenant-1
            create_test_device(2, Some(20)), // tenant-2
            create_test_device(3, Some(10)), // tenant-1
        ];
        
        let filtered = visibility.get_tenant_devices(&"tenant-1".to_string(), devices).unwrap();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_get_tenant_sites_no_mapping() {
        let mapping_service = TenantMappingService::new();
        let access_control = TenantAccessControl::new(mapping_service);
        let visibility = TenantResourceVisibility::new(access_control);
        
        let sites = vec![create_test_site(1, Some(10))];
        
        assert!(visibility.get_tenant_sites(&"nonexistent".to_string(), sites).is_err());
    }

    #[test]
    fn test_tenant_isolation_multiple_tenants() {
        let mapping_service = TenantMappingService::new();
        mapping_service.register_mapping("tenant-1".to_string(), 10);
        mapping_service.register_mapping("tenant-2".to_string(), 20);
        mapping_service.register_mapping("tenant-3".to_string(), 30);
        
        let access_control = TenantAccessControl::new(mapping_service);
        
        let sites = vec![
            create_test_site(1, Some(10)), // tenant-1
            create_test_site(2, Some(20)), // tenant-2
            create_test_site(3, Some(30)), // tenant-3
            create_test_site(4, Some(10)), // tenant-1
        ];
        
        let tenant1_sites = access_control.filter_sites_by_tenant(&"tenant-1".to_string(), sites.clone()).unwrap();
        assert_eq!(tenant1_sites.len(), 2);
        
        let tenant2_sites = access_control.filter_sites_by_tenant(&"tenant-2".to_string(), sites.clone()).unwrap();
        assert_eq!(tenant2_sites.len(), 1);
        
        let tenant3_sites = access_control.filter_sites_by_tenant(&"tenant-3".to_string(), sites).unwrap();
        assert_eq!(tenant3_sites.len(), 1);
    }
}

