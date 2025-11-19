use crate::netbox::models::{NetBoxDevice, NetBoxSite};
use crate::r#virtual::mapping::{MappingManager, MappingType};
use crate::r#virtual::models::{
    NetBoxDeviceAdapter, NetBoxSiteAdapter, Resource, VirtualDevice, VirtualNetwork, VirtualSite,
    VirtualResourceType,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Virtual resource store
pub struct VirtualResourceStore {
    sites: RwLock<HashMap<String, VirtualSite>>,
    devices: RwLock<HashMap<String, VirtualDevice>>,
    networks: RwLock<HashMap<String, VirtualNetwork>>,
}

impl Default for VirtualResourceStore {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualResourceStore {
    pub fn new() -> Self {
        Self {
            sites: RwLock::new(HashMap::new()),
            devices: RwLock::new(HashMap::new()),
            networks: RwLock::new(HashMap::new()),
        }
    }

    pub fn create_virtual_site(&self, id: String, name: String, tenant_id: String) -> VirtualSite {
        let site = VirtualSite::new(id.clone(), name, tenant_id.clone());
        let mut sites = self.sites.write().unwrap();
        sites.insert(id, site.clone());
        site
    }

    pub fn get_virtual_site(&self, id: &str) -> Option<VirtualSite> {
        let sites = self.sites.read().unwrap();
        sites.get(id).cloned()
    }

    pub fn get_tenant_virtual_sites(&self, tenant_id: &str) -> Vec<VirtualSite> {
        let sites = self.sites.read().unwrap();
        sites
            .values()
            .filter(|s| s.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    pub fn create_virtual_device(&self, id: String, name: String, tenant_id: String) -> VirtualDevice {
        let device = VirtualDevice::new(id.clone(), name, tenant_id.clone());
        let mut devices = self.devices.write().unwrap();
        devices.insert(id, device.clone());
        device
    }

    pub fn get_virtual_device(&self, id: &str) -> Option<VirtualDevice> {
        let devices = self.devices.read().unwrap();
        devices.get(id).cloned()
    }

    pub fn get_tenant_virtual_devices(&self, tenant_id: &str) -> Vec<VirtualDevice> {
        let devices = self.devices.read().unwrap();
        devices
            .values()
            .filter(|d| d.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    pub fn create_virtual_network(&self, id: String, name: String, tenant_id: String) -> VirtualNetwork {
        let network = VirtualNetwork::new(id.clone(), name, tenant_id.clone());
        let mut networks = self.networks.write().unwrap();
        networks.insert(id, network.clone());
        network
    }

    pub fn get_virtual_network(&self, id: &str) -> Option<VirtualNetwork> {
        let networks = self.networks.read().unwrap();
        networks.get(id).cloned()
    }

    pub fn get_tenant_virtual_networks(&self, tenant_id: &str) -> Vec<VirtualNetwork> {
        let networks = self.networks.read().unwrap();
        networks
            .values()
            .filter(|n| n.tenant_id == tenant_id)
            .cloned()
            .collect()
    }
}

/// Virtual resource service - abstraction layer over virtual and physical resources
pub struct VirtualResourceService {
    store: Arc<VirtualResourceStore>,
    mapping_manager: Arc<MappingManager>,
}

impl VirtualResourceService {
    pub fn new() -> Self {
        Self {
            store: Arc::new(VirtualResourceStore::new()),
            mapping_manager: Arc::new(MappingManager::new()),
        }
    }

    /// Create a virtual site and optionally map it to physical NetBox sites
    pub fn create_virtual_site(
        &self,
        name: String,
        tenant_id: String,
        physical_site_ids: Vec<i32>,
    ) -> VirtualSite {
        let id = uuid::Uuid::new_v4().to_string();
        let virtual_site = self.store.create_virtual_site(id.clone(), name, tenant_id.clone());

        // Create mappings to physical sites
        for physical_id in physical_site_ids.iter() {
            self.mapping_manager.create_mapping(
                id.clone(),
                VirtualResourceType::Site,
                *physical_id,
                VirtualResourceType::Site,
                tenant_id.clone(),
                if physical_site_ids.is_empty() || physical_site_ids.len() == 1 {
                    MappingType::OneToOne
                } else {
                    MappingType::OneToMany
                },
            );
        }

        virtual_site
    }

    /// Get virtual site with its physical mappings
    pub fn get_virtual_site_with_mappings(&self, virtual_id: &str) -> Option<(VirtualSite, Vec<i32>)> {
        let virtual_site = self.store.get_virtual_site(virtual_id)?;
        let mappings = self.mapping_manager.get_physical_resources(virtual_id);
        let physical_ids: Vec<i32> = mappings.iter().map(|m| m.physical_id).collect();
        Some((virtual_site, physical_ids))
    }

    /// Get physical NetBox sites for a virtual site
    pub fn get_physical_sites_for_virtual(&self, virtual_id: &str) -> Vec<i32> {
        self.mapping_manager
            .get_physical_resources(virtual_id)
            .iter()
            .map(|m| m.physical_id)
            .collect()
    }

    /// Get virtual sites for a physical NetBox site
    pub fn get_virtual_sites_for_physical(&self, physical_id: i32) -> Vec<String> {
        self.mapping_manager
            .get_virtual_resources(physical_id)
            .iter()
            .map(|m| m.virtual_id.clone())
            .collect()
    }

    /// Map a virtual site to a physical NetBox site
    pub fn map_virtual_to_physical_site(
        &self,
        virtual_id: &str,
        physical_id: i32,
        tenant_id: &str,
    ) {
        self.mapping_manager.create_mapping(
            virtual_id.to_string(),
            VirtualResourceType::Site,
            physical_id,
            VirtualResourceType::Site,
            tenant_id.to_string(),
            MappingType::OneToMany,
        );
    }

    /// Create a virtual device and optionally map it to physical NetBox devices
    pub fn create_virtual_device(
        &self,
        name: String,
        tenant_id: String,
        physical_device_ids: Vec<i32>,
    ) -> VirtualDevice {
        let id = uuid::Uuid::new_v4().to_string();
        let virtual_device = self.store.create_virtual_device(id.clone(), name, tenant_id.clone());

        // Create mappings to physical devices
        for physical_id in physical_device_ids.iter() {
            self.mapping_manager.create_mapping(
                id.clone(),
                VirtualResourceType::Device,
                *physical_id,
                VirtualResourceType::Device,
                tenant_id.clone(),
                if physical_device_ids.is_empty() || physical_device_ids.len() == 1 {
                    MappingType::OneToOne
                } else {
                    MappingType::OneToMany
                },
            );
        }

        virtual_device
    }

    /// Get all resources (virtual and physical) for a tenant using the Resource trait
    pub fn get_all_resources_for_tenant(&self, tenant_id: &str) -> Vec<Box<dyn Resource + Send + Sync>> {
        let mut resources: Vec<Box<dyn Resource + Send + Sync>> = Vec::new();

        // Add virtual sites
        for site in self.store.get_tenant_virtual_sites(tenant_id) {
            resources.push(Box::new(site));
        }

        // Add virtual devices
        for device in self.store.get_tenant_virtual_devices(tenant_id) {
            resources.push(Box::new(device));
        }

        // Add virtual networks
        for network in self.store.get_tenant_virtual_networks(tenant_id) {
            resources.push(Box::new(network));
        }

        resources
    }

    /// Convert NetBox site to Resource trait object
    pub fn netbox_site_to_resource(&self, site: NetBoxSite, tenant_id: String) -> Box<dyn Resource + Send + Sync> {
        Box::new(NetBoxSiteAdapter::new(site, tenant_id))
    }

    /// Convert NetBox device to Resource trait object
    pub fn netbox_device_to_resource(&self, device: NetBoxDevice, tenant_id: String) -> Box<dyn Resource + Send + Sync> {
        Box::new(NetBoxDeviceAdapter::new(device, tenant_id))
    }

    /// Get mapping manager reference
    pub fn mapping_manager(&self) -> &Arc<MappingManager> {
        &self.mapping_manager
    }
}

impl Default for VirtualResourceService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_virtual_site() {
        let service = VirtualResourceService::new();
        let virtual_site = service.create_virtual_site(
            "Test Site".to_string(),
            "tenant-1".to_string(),
            vec![123, 456],
        );

        assert_eq!(virtual_site.name, "Test Site");
        assert_eq!(virtual_site.tenant_id, "tenant-1");
        assert!(virtual_site.is_virtual());
    }

    #[test]
    fn test_get_virtual_site_with_mappings() {
        let service = VirtualResourceService::new();
        let virtual_site = service.create_virtual_site(
            "Test Site".to_string(),
            "tenant-1".to_string(),
            vec![123, 456],
        );

        let (site, physical_ids) = service
            .get_virtual_site_with_mappings(&virtual_site.id)
            .unwrap();

        assert_eq!(site.id, virtual_site.id);
        assert_eq!(physical_ids.len(), 2);
        assert!(physical_ids.contains(&123));
        assert!(physical_ids.contains(&456));
    }

    #[test]
    fn test_get_physical_sites_for_virtual() {
        let service = VirtualResourceService::new();
        let virtual_site = service.create_virtual_site(
            "Test Site".to_string(),
            "tenant-1".to_string(),
            vec![123],
        );

        let physical_ids = service.get_physical_sites_for_virtual(&virtual_site.id);
        assert_eq!(physical_ids, vec![123]);
    }

    #[test]
    fn test_map_virtual_to_physical_site() {
        let service = VirtualResourceService::new();
        let virtual_site = service.create_virtual_site(
            "Test Site".to_string(),
            "tenant-1".to_string(),
            vec![],
        );

        service.map_virtual_to_physical_site(&virtual_site.id, 789, "tenant-1");

        let physical_ids = service.get_physical_sites_for_virtual(&virtual_site.id);
        assert_eq!(physical_ids, vec![789]);
    }

    #[test]
    fn test_create_virtual_device() {
        let service = VirtualResourceService::new();
        let virtual_device = service.create_virtual_device(
            "Test Device".to_string(),
            "tenant-1".to_string(),
            vec![100],
        );

        assert_eq!(virtual_device.name, "Test Device");
        assert_eq!(virtual_device.tenant_id, "tenant-1");
        assert!(virtual_device.is_virtual());
    }

    #[test]
    fn test_netbox_site_to_resource() {
        use crate::netbox::models::NetBoxSite;
        let service = VirtualResourceService::new();
        let netbox_site = NetBoxSite {
            id: Some(123),
            name: "Physical Site".to_string(),
            ..Default::default()
        };

        let resource = service.netbox_site_to_resource(netbox_site, "tenant-1".to_string());
        assert_eq!(resource.id(), "123");
        assert_eq!(resource.name(), "Physical Site");
        assert_eq!(resource.tenant_id(), "tenant-1");
        assert!(!resource.is_virtual());
    }

    #[test]
    fn test_get_all_resources_for_tenant() {
        let service = VirtualResourceService::new();
        service.create_virtual_site("Site 1".to_string(), "tenant-1".to_string(), vec![]);
        service.create_virtual_device("Device 1".to_string(), "tenant-1".to_string(), vec![]);
        service.create_virtual_site("Site 2".to_string(), "tenant-2".to_string(), vec![]);

        let resources = service.get_all_resources_for_tenant("tenant-1");
        assert_eq!(resources.len(), 2);
    }

    #[test]
    fn test_virtual_resource_store_default() {
        let store = VirtualResourceStore::default();
        assert!(store.get_virtual_site("nonexistent").is_none());
    }

    #[test]
    fn test_virtual_resource_service_default() {
        let service = VirtualResourceService::default();
        let resources = service.get_all_resources_for_tenant("tenant-1");
        assert_eq!(resources.len(), 0);
    }

    #[test]
    fn test_get_virtual_site_nonexistent() {
        let store = VirtualResourceStore::new();
        assert!(store.get_virtual_site("nonexistent").is_none());
    }

    #[test]
    fn test_get_virtual_device_nonexistent() {
        let store = VirtualResourceStore::new();
        assert!(store.get_virtual_device("nonexistent").is_none());
    }

    #[test]
    fn test_get_virtual_network_nonexistent() {
        let store = VirtualResourceStore::new();
        assert!(store.get_virtual_network("nonexistent").is_none());
    }

    #[test]
    fn test_get_tenant_virtual_sites_empty() {
        let store = VirtualResourceStore::new();
        let sites = store.get_tenant_virtual_sites("tenant-1");
        assert_eq!(sites.len(), 0);
    }

    #[test]
    fn test_get_tenant_virtual_devices_empty() {
        let store = VirtualResourceStore::new();
        let devices = store.get_tenant_virtual_devices("tenant-1");
        assert_eq!(devices.len(), 0);
    }

    #[test]
    fn test_get_tenant_virtual_networks_empty() {
        let store = VirtualResourceStore::new();
        let networks = store.get_tenant_virtual_networks("tenant-1");
        assert_eq!(networks.len(), 0);
    }

    #[test]
    fn test_create_virtual_site_no_physical_mappings() {
        let service = VirtualResourceService::new();
        let virtual_site = service.create_virtual_site(
            "Test Site".to_string(),
            "tenant-1".to_string(),
            vec![],
        );

        assert_eq!(virtual_site.name, "Test Site");
        let physical_ids = service.get_physical_sites_for_virtual(&virtual_site.id);
        assert_eq!(physical_ids.len(), 0);
    }

    #[test]
    fn test_create_virtual_device_no_physical_mappings() {
        let service = VirtualResourceService::new();
        let virtual_device = service.create_virtual_device(
            "Test Device".to_string(),
            "tenant-1".to_string(),
            vec![],
        );

        assert_eq!(virtual_device.name, "Test Device");
    }

    #[test]
    fn test_get_virtual_site_with_mappings_nonexistent() {
        let service = VirtualResourceService::new();
        let result = service.get_virtual_site_with_mappings("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_virtual_sites_for_physical() {
        let service = VirtualResourceService::new();
        let virtual_site1 = service.create_virtual_site(
            "Site 1".to_string(),
            "tenant-1".to_string(),
            vec![123],
        );
        let virtual_site2 = service.create_virtual_site(
            "Site 2".to_string(),
            "tenant-1".to_string(),
            vec![123],
        );

        let virtual_ids = service.get_virtual_sites_for_physical(123);
        assert_eq!(virtual_ids.len(), 2);
        assert!(virtual_ids.contains(&virtual_site1.id));
        assert!(virtual_ids.contains(&virtual_site2.id));
    }

    #[test]
    fn test_get_virtual_sites_for_physical_nonexistent() {
        let service = VirtualResourceService::new();
        let virtual_ids = service.get_virtual_sites_for_physical(999);
        assert_eq!(virtual_ids.len(), 0);
    }

    #[test]
    fn test_create_virtual_site_one_physical() {
        let service = VirtualResourceService::new();
        let virtual_site = service.create_virtual_site(
            "Test Site".to_string(),
            "tenant-1".to_string(),
            vec![123],
        );

        let physical_ids = service.get_physical_sites_for_virtual(&virtual_site.id);
        assert_eq!(physical_ids.len(), 1);
        assert_eq!(physical_ids[0], 123);
    }

    #[test]
    fn test_create_virtual_device_one_physical() {
        let service = VirtualResourceService::new();
        let virtual_device = service.create_virtual_device(
            "Test Device".to_string(),
            "tenant-1".to_string(),
            vec![100],
        );

        assert_eq!(virtual_device.name, "Test Device");
    }

    #[test]
    fn test_create_virtual_device_multiple_physical() {
        let service = VirtualResourceService::new();
        let virtual_device = service.create_virtual_device(
            "Test Device".to_string(),
            "tenant-1".to_string(),
            vec![100, 200, 300],
        );

        assert_eq!(virtual_device.name, "Test Device");
    }

    #[test]
    fn test_get_all_resources_includes_networks() {
        let service = VirtualResourceService::new();
        let store = &service.store;
        store.create_virtual_network("vn-1".to_string(), "Network 1".to_string(), "tenant-1".to_string());
        store.create_virtual_site("vs-1".to_string(), "Site 1".to_string(), "tenant-1".to_string());
        store.create_virtual_device("vd-1".to_string(), "Device 1".to_string(), "tenant-1".to_string());

        let resources = service.get_all_resources_for_tenant("tenant-1");
        assert_eq!(resources.len(), 3);
    }

    #[test]
    fn test_netbox_device_to_resource() {
        use crate::netbox::models::NetBoxDevice;
        let service = VirtualResourceService::new();
        let netbox_device = NetBoxDevice {
            id: Some(789),
            name: Some("Physical Device".to_string()),
            ..Default::default()
        };

        let resource = service.netbox_device_to_resource(netbox_device, "tenant-1".to_string());
        assert_eq!(resource.id(), "789");
        assert_eq!(resource.name(), "Physical Device");
        assert_eq!(resource.tenant_id(), "tenant-1");
        assert!(!resource.is_virtual());
    }

    #[test]
    fn test_mapping_manager_access() {
        let service = VirtualResourceService::new();
        let manager = service.mapping_manager();
        
        manager.create_mapping(
            "vs-1".to_string(),
            VirtualResourceType::Site,
            123,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::OneToOne,
        );

        assert!(manager.has_physical_mapping("vs-1"));
    }

    #[test]
    fn test_virtual_resource_store_create_and_get() {
        let store = VirtualResourceStore::new();
        let site = store.create_virtual_site("vs-1".to_string(), "Site".to_string(), "tenant-1".to_string());
        
        let retrieved = store.get_virtual_site("vs-1").unwrap();
        assert_eq!(retrieved.id, site.id);
        assert_eq!(retrieved.name, site.name);
    }

    #[test]
    fn test_virtual_resource_store_tenant_isolation() {
        let store = VirtualResourceStore::new();
        store.create_virtual_site("vs-1".to_string(), "Site 1".to_string(), "tenant-1".to_string());
        store.create_virtual_site("vs-2".to_string(), "Site 2".to_string(), "tenant-2".to_string());

        let tenant1_sites = store.get_tenant_virtual_sites("tenant-1");
        assert_eq!(tenant1_sites.len(), 1);
        assert_eq!(tenant1_sites[0].name, "Site 1");

        let tenant2_sites = store.get_tenant_virtual_sites("tenant-2");
        assert_eq!(tenant2_sites.len(), 1);
        assert_eq!(tenant2_sites[0].name, "Site 2");
    }

    #[test]
    fn test_map_virtual_to_physical_multiple_times() {
        let service = VirtualResourceService::new();
        let virtual_site = service.create_virtual_site(
            "Test Site".to_string(),
            "tenant-1".to_string(),
            vec![],
        );

        service.map_virtual_to_physical_site(&virtual_site.id, 100, "tenant-1");
        service.map_virtual_to_physical_site(&virtual_site.id, 200, "tenant-1");

        let physical_ids = service.get_physical_sites_for_virtual(&virtual_site.id);
        assert_eq!(physical_ids.len(), 2);
        assert!(physical_ids.contains(&100));
        assert!(physical_ids.contains(&200));
    }
}

