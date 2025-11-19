use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Virtual resource types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VirtualResourceType {
    Site,
    Device,
    Network,
    Service,
}

/// Virtual site - a logical grouping that may map to multiple NetBox sites
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualSite {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tenant_id: String,
    pub virtual_type: VirtualResourceType,
    pub metadata: HashMap<String, String>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl VirtualSite {
    pub fn new(id: String, name: String, tenant_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name,
            description: None,
            tenant_id,
            virtual_type: VirtualResourceType::Site,
            metadata: HashMap::new(),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Virtual device - a logical device that may map to multiple NetBox devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualDevice {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tenant_id: String,
    pub virtual_type: VirtualResourceType,
    pub metadata: HashMap<String, String>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl VirtualDevice {
    pub fn new(id: String, name: String, tenant_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name,
            description: None,
            tenant_id,
            virtual_type: VirtualResourceType::Device,
            metadata: HashMap::new(),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Virtual network - a logical network abstraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualNetwork {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tenant_id: String,
    pub virtual_type: VirtualResourceType,
    pub cidr: Option<String>,
    pub metadata: HashMap<String, String>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl VirtualNetwork {
    pub fn new(id: String, name: String, tenant_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name,
            description: None,
            tenant_id,
            virtual_type: VirtualResourceType::Network,
            cidr: None,
            metadata: HashMap::new(),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Abstraction trait for resources (both virtual and physical)
pub trait Resource {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn tenant_id(&self) -> &str;
    fn resource_type(&self) -> VirtualResourceType;
    fn is_virtual(&self) -> bool;
}

impl Resource for VirtualSite {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    fn resource_type(&self) -> VirtualResourceType {
        self.virtual_type
    }

    fn is_virtual(&self) -> bool {
        true
    }
}

impl Resource for VirtualDevice {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    fn resource_type(&self) -> VirtualResourceType {
        self.virtual_type
    }

    fn is_virtual(&self) -> bool {
        true
    }
}

impl Resource for VirtualNetwork {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    fn resource_type(&self) -> VirtualResourceType {
        self.virtual_type
    }

    fn is_virtual(&self) -> bool {
        true
    }
}

/// Adapter for NetBox Site to implement Resource trait
pub struct NetBoxSiteAdapter {
    pub site: crate::netbox::models::NetBoxSite,
    pub tenant_id: String,
    id_string: String,
}

impl NetBoxSiteAdapter {
    pub fn new(site: crate::netbox::models::NetBoxSite, tenant_id: String) -> Self {
        let id_string = site.id.map(|i| i.to_string()).unwrap_or_default();
        Self {
            site,
            tenant_id,
            id_string,
        }
    }
}

impl Resource for NetBoxSiteAdapter {
    fn id(&self) -> &str {
        &self.id_string
    }

    fn name(&self) -> &str {
        &self.site.name
    }

    fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    fn resource_type(&self) -> VirtualResourceType {
        VirtualResourceType::Site
    }

    fn is_virtual(&self) -> bool {
        false
    }
}

/// Adapter for NetBox Device to implement Resource trait
pub struct NetBoxDeviceAdapter {
    pub device: crate::netbox::models::NetBoxDevice,
    pub tenant_id: String,
    id_string: String,
}

impl NetBoxDeviceAdapter {
    pub fn new(device: crate::netbox::models::NetBoxDevice, tenant_id: String) -> Self {
        let id_string = device.id.map(|i| i.to_string()).unwrap_or_default();
        Self {
            device,
            tenant_id,
            id_string,
        }
    }
}

impl Resource for NetBoxDeviceAdapter {
    fn id(&self) -> &str {
        &self.id_string
    }

    fn name(&self) -> &str {
        self.device.name.as_deref().unwrap_or("Unnamed Device")
    }

    fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    fn resource_type(&self) -> VirtualResourceType {
        VirtualResourceType::Device
    }

    fn is_virtual(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_site_creation() {
        let site = VirtualSite::new(
            "vs-1".to_string(),
            "Virtual Site 1".to_string(),
            "tenant-1".to_string(),
        );

        assert_eq!(site.id, "vs-1");
        assert_eq!(site.name, "Virtual Site 1");
        assert_eq!(site.tenant_id, "tenant-1");
        assert_eq!(site.virtual_type, VirtualResourceType::Site);
        assert!(site.is_virtual());
    }

    #[test]
    fn test_virtual_device_creation() {
        let device = VirtualDevice::new(
            "vd-1".to_string(),
            "Virtual Device 1".to_string(),
            "tenant-1".to_string(),
        );

        assert_eq!(device.id, "vd-1");
        assert_eq!(device.name, "Virtual Device 1");
        assert_eq!(device.tenant_id, "tenant-1");
        assert_eq!(device.virtual_type, VirtualResourceType::Device);
        assert!(device.is_virtual());
    }

    #[test]
    fn test_virtual_network_creation() {
        let network = VirtualNetwork::new(
            "vn-1".to_string(),
            "Virtual Network 1".to_string(),
            "tenant-1".to_string(),
        );

        assert_eq!(network.id, "vn-1");
        assert_eq!(network.name, "Virtual Network 1");
        assert_eq!(network.tenant_id, "tenant-1");
        assert_eq!(network.virtual_type, VirtualResourceType::Network);
        assert!(network.is_virtual());
    }

    #[test]
    fn test_netbox_site_adapter() {
        use crate::netbox::models::NetBoxSite;
        let netbox_site = NetBoxSite {
            id: Some(123),
            name: "Physical Site".to_string(),
            ..Default::default()
        };

        let adapter = NetBoxSiteAdapter::new(netbox_site, "tenant-1".to_string());

        assert_eq!(adapter.id(), "123");
        assert_eq!(adapter.name(), "Physical Site");
        assert_eq!(adapter.tenant_id(), "tenant-1");
        assert_eq!(adapter.resource_type(), VirtualResourceType::Site);
        assert!(!adapter.is_virtual());
    }

    #[test]
    fn test_netbox_device_adapter() {
        use crate::netbox::models::NetBoxDevice;
        let netbox_device = NetBoxDevice {
            id: Some(456),
            name: Some("Physical Device".to_string()),
            ..Default::default()
        };

        let adapter = NetBoxDeviceAdapter::new(netbox_device, "tenant-1".to_string());

        assert_eq!(adapter.id(), "456");
        assert_eq!(adapter.name(), "Physical Device");
        assert_eq!(adapter.tenant_id(), "tenant-1");
        assert_eq!(adapter.resource_type(), VirtualResourceType::Device);
        assert!(!adapter.is_virtual());
    }

    #[test]
    fn test_netbox_site_adapter_no_id() {
        use crate::netbox::models::NetBoxSite;
        let netbox_site = NetBoxSite {
            id: None,
            name: "Site Without ID".to_string(),
            ..Default::default()
        };

        let adapter = NetBoxSiteAdapter::new(netbox_site, "tenant-1".to_string());
        assert_eq!(adapter.id(), "");
        assert_eq!(adapter.name(), "Site Without ID");
    }

    #[test]
    fn test_netbox_device_adapter_no_id() {
        use crate::netbox::models::NetBoxDevice;
        let netbox_device = NetBoxDevice {
            id: None,
            name: Some("Device Without ID".to_string()),
            ..Default::default()
        };

        let adapter = NetBoxDeviceAdapter::new(netbox_device, "tenant-1".to_string());
        assert_eq!(adapter.id(), "");
        assert_eq!(adapter.name(), "Device Without ID");
    }

    #[test]
    fn test_netbox_device_adapter_no_name() {
        use crate::netbox::models::NetBoxDevice;
        let netbox_device = NetBoxDevice {
            id: Some(999),
            name: None,
            ..Default::default()
        };

        let adapter = NetBoxDeviceAdapter::new(netbox_device, "tenant-1".to_string());
        assert_eq!(adapter.id(), "999");
        assert_eq!(adapter.name(), "Unnamed Device");
    }

    #[test]
    fn test_virtual_site_resource_trait() {
        let site = VirtualSite::new("vs-1".to_string(), "Site".to_string(), "tenant-1".to_string());
        
        assert_eq!(site.id(), "vs-1");
        assert_eq!(site.name(), "Site");
        assert_eq!(site.tenant_id(), "tenant-1");
        assert_eq!(site.resource_type(), VirtualResourceType::Site);
        assert!(site.is_virtual());
    }

    #[test]
    fn test_virtual_device_resource_trait() {
        let device = VirtualDevice::new("vd-1".to_string(), "Device".to_string(), "tenant-1".to_string());
        
        assert_eq!(device.id(), "vd-1");
        assert_eq!(device.name(), "Device");
        assert_eq!(device.tenant_id(), "tenant-1");
        assert_eq!(device.resource_type(), VirtualResourceType::Device);
        assert!(device.is_virtual());
    }

    #[test]
    fn test_virtual_network_resource_trait() {
        let network = VirtualNetwork::new("vn-1".to_string(), "Network".to_string(), "tenant-1".to_string());
        
        assert_eq!(network.id(), "vn-1");
        assert_eq!(network.name(), "Network");
        assert_eq!(network.tenant_id(), "tenant-1");
        assert_eq!(network.resource_type(), VirtualResourceType::Network);
        assert!(network.is_virtual());
    }

    #[test]
    fn test_virtual_site_with_metadata() {
        let mut site = VirtualSite::new("vs-1".to_string(), "Site".to_string(), "tenant-1".to_string());
        site.metadata.insert("key1".to_string(), "value1".to_string());
        site.tags.push("tag1".to_string());
        
        assert_eq!(site.metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(site.tags.len(), 1);
    }

    #[test]
    fn test_virtual_resource_type_variants() {
        assert_eq!(VirtualResourceType::Site, VirtualResourceType::Site);
        assert_eq!(VirtualResourceType::Device, VirtualResourceType::Device);
        assert_eq!(VirtualResourceType::Network, VirtualResourceType::Network);
        assert_eq!(VirtualResourceType::Service, VirtualResourceType::Service);
    }
}

