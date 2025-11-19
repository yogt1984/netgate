use crate::r#virtual::models::VirtualResourceType;
use std::collections::HashMap;
use std::sync::RwLock;

/// Mapping between virtual and physical resources
#[derive(Debug, Clone)]
pub struct ResourceMapping {
    pub virtual_id: String,
    pub virtual_type: VirtualResourceType,
    pub physical_id: i32,
    pub physical_type: VirtualResourceType,
    pub tenant_id: String,
    pub mapping_type: MappingType,
    pub metadata: HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Type of mapping relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingType {
    /// One-to-one: one virtual resource maps to one physical resource
    OneToOne,
    /// One-to-many: one virtual resource maps to multiple physical resources
    OneToMany,
    /// Many-to-one: multiple virtual resources map to one physical resource
    ManyToOne,
    /// Many-to-many: multiple virtual resources map to multiple physical resources
    ManyToMany,
}

/// Mapping manager for tracking virtual-to-physical resource mappings
pub struct MappingManager {
    // Map from virtual_id to list of physical_ids
    virtual_to_physical: RwLock<HashMap<String, Vec<ResourceMapping>>>,
    // Map from physical_id to list of virtual_ids
    physical_to_virtual: RwLock<HashMap<i32, Vec<ResourceMapping>>>,
    // Map from tenant_id to all mappings
    tenant_mappings: RwLock<HashMap<String, Vec<ResourceMapping>>>,
}

impl Default for MappingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MappingManager {
    /// Create a new mapping manager
    pub fn new() -> Self {
        Self {
            virtual_to_physical: RwLock::new(HashMap::new()),
            physical_to_virtual: RwLock::new(HashMap::new()),
            tenant_mappings: RwLock::new(HashMap::new()),
        }
    }

    /// Create a mapping between virtual and physical resources
    pub fn create_mapping(
        &self,
        virtual_id: String,
        virtual_type: VirtualResourceType,
        physical_id: i32,
        physical_type: VirtualResourceType,
        tenant_id: String,
        mapping_type: MappingType,
    ) -> ResourceMapping {
        let mapping = ResourceMapping {
            virtual_id: virtual_id.clone(),
            virtual_type,
            physical_id,
            physical_type,
            tenant_id: tenant_id.clone(),
            mapping_type,
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
        };

        // Add to virtual -> physical mapping
        let mut vtp = self.virtual_to_physical.write().unwrap();
        vtp.entry(virtual_id.clone())
            .or_insert_with(Vec::new)
            .push(mapping.clone());

        // Add to physical -> virtual mapping
        let mut ptv = self.physical_to_virtual.write().unwrap();
        ptv.entry(physical_id)
            .or_insert_with(Vec::new)
            .push(mapping.clone());

        // Add to tenant mappings
        let mut tm = self.tenant_mappings.write().unwrap();
        tm.entry(tenant_id)
            .or_insert_with(Vec::new)
            .push(mapping.clone());

        mapping
    }

    /// Get all physical resources mapped to a virtual resource
    pub fn get_physical_resources(&self, virtual_id: &str) -> Vec<ResourceMapping> {
        let vtp = self.virtual_to_physical.read().unwrap();
        vtp.get(virtual_id).cloned().unwrap_or_default()
    }

    /// Get all virtual resources mapped to a physical resource
    pub fn get_virtual_resources(&self, physical_id: i32) -> Vec<ResourceMapping> {
        let ptv = self.physical_to_virtual.read().unwrap();
        ptv.get(&physical_id).cloned().unwrap_or_default()
    }

    /// Get all mappings for a tenant
    pub fn get_tenant_mappings(&self, tenant_id: &str) -> Vec<ResourceMapping> {
        let tm = self.tenant_mappings.read().unwrap();
        tm.get(tenant_id).cloned().unwrap_or_default()
    }

    /// Remove a mapping
    pub fn remove_mapping(
        &self,
        virtual_id: &str,
        physical_id: i32,
    ) -> Result<(), MappingError> {
        let mut vtp = self.virtual_to_physical.write().unwrap();
        if let Some(mappings) = vtp.get_mut(virtual_id) {
            mappings.retain(|m| m.physical_id != physical_id);
            if mappings.is_empty() {
                vtp.remove(virtual_id);
            }
        }

        let mut ptv = self.physical_to_virtual.write().unwrap();
        if let Some(mappings) = ptv.get_mut(&physical_id) {
            mappings.retain(|m| m.virtual_id != virtual_id);
            if mappings.is_empty() {
                ptv.remove(&physical_id);
            }
        }

        Ok(())
    }

    /// Check if a virtual resource has any physical mappings
    pub fn has_physical_mapping(&self, virtual_id: &str) -> bool {
        let vtp = self.virtual_to_physical.read().unwrap();
        vtp.contains_key(virtual_id) && !vtp[virtual_id].is_empty()
    }

    /// Check if a physical resource has any virtual mappings
    pub fn has_virtual_mapping(&self, physical_id: i32) -> bool {
        let ptv = self.physical_to_virtual.read().unwrap();
        ptv.contains_key(&physical_id) && !ptv[&physical_id].is_empty()
    }

    /// Get mapping count for a virtual resource
    pub fn get_mapping_count(&self, virtual_id: &str) -> usize {
        let vtp = self.virtual_to_physical.read().unwrap();
        vtp.get(virtual_id).map(|v| v.len()).unwrap_or(0)
    }
}

/// Mapping errors
#[derive(Debug, Clone, PartialEq)]
pub enum MappingError {
    MappingNotFound,
    InvalidMapping,
}

impl std::fmt::Display for MappingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MappingError::MappingNotFound => write!(f, "Mapping not found"),
            MappingError::InvalidMapping => write!(f, "Invalid mapping"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_one_to_one_mapping() {
        let manager = MappingManager::new();
        let mapping = manager.create_mapping(
            "vs-1".to_string(),
            VirtualResourceType::Site,
            123,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::OneToOne,
        );

        assert_eq!(mapping.virtual_id, "vs-1");
        assert_eq!(mapping.physical_id, 123);
        assert_eq!(mapping.mapping_type, MappingType::OneToOne);
    }

    #[test]
    fn test_get_physical_resources() {
        let manager = MappingManager::new();
        manager.create_mapping(
            "vs-1".to_string(),
            VirtualResourceType::Site,
            123,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::OneToOne,
        );

        let physical = manager.get_physical_resources("vs-1");
        assert_eq!(physical.len(), 1);
        assert_eq!(physical[0].physical_id, 123);
    }

    #[test]
    fn test_get_virtual_resources() {
        let manager = MappingManager::new();
        manager.create_mapping(
            "vs-1".to_string(),
            VirtualResourceType::Site,
            123,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::OneToOne,
        );

        let virtual_resources = manager.get_virtual_resources(123);
        assert_eq!(virtual_resources.len(), 1);
        assert_eq!(virtual_resources[0].virtual_id, "vs-1");
    }

    #[test]
    fn test_one_to_many_mapping() {
        let manager = MappingManager::new();
        manager.create_mapping(
            "vs-1".to_string(),
            VirtualResourceType::Site,
            123,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::OneToMany,
        );
        manager.create_mapping(
            "vs-1".to_string(),
            VirtualResourceType::Site,
            456,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::OneToMany,
        );

        let physical = manager.get_physical_resources("vs-1");
        assert_eq!(physical.len(), 2);
        assert!(physical.iter().any(|m| m.physical_id == 123));
        assert!(physical.iter().any(|m| m.physical_id == 456));
    }

    #[test]
    fn test_many_to_one_mapping() {
        let manager = MappingManager::new();
        manager.create_mapping(
            "vs-1".to_string(),
            VirtualResourceType::Site,
            123,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::ManyToOne,
        );
        manager.create_mapping(
            "vs-2".to_string(),
            VirtualResourceType::Site,
            123,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::ManyToOne,
        );

        let virtual_resources = manager.get_virtual_resources(123);
        assert_eq!(virtual_resources.len(), 2);
        assert!(virtual_resources.iter().any(|m| m.virtual_id == "vs-1"));
        assert!(virtual_resources.iter().any(|m| m.virtual_id == "vs-2"));
    }

    #[test]
    fn test_get_tenant_mappings() {
        let manager = MappingManager::new();
        manager.create_mapping(
            "vs-1".to_string(),
            VirtualResourceType::Site,
            123,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::OneToOne,
        );
        manager.create_mapping(
            "vs-2".to_string(),
            VirtualResourceType::Site,
            456,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::OneToOne,
        );
        manager.create_mapping(
            "vs-3".to_string(),
            VirtualResourceType::Site,
            789,
            VirtualResourceType::Site,
            "tenant-2".to_string(),
            MappingType::OneToOne,
        );

        let tenant1_mappings = manager.get_tenant_mappings("tenant-1");
        assert_eq!(tenant1_mappings.len(), 2);

        let tenant2_mappings = manager.get_tenant_mappings("tenant-2");
        assert_eq!(tenant2_mappings.len(), 1);
    }

    #[test]
    fn test_remove_mapping() {
        let manager = MappingManager::new();
        manager.create_mapping(
            "vs-1".to_string(),
            VirtualResourceType::Site,
            123,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::OneToOne,
        );

        assert!(manager.has_physical_mapping("vs-1"));
        assert!(manager.has_virtual_mapping(123));

        manager.remove_mapping("vs-1", 123).unwrap();

        assert!(!manager.has_physical_mapping("vs-1"));
        assert!(!manager.has_virtual_mapping(123));
    }

    #[test]
    fn test_has_physical_mapping() {
        let manager = MappingManager::new();
        assert!(!manager.has_physical_mapping("vs-1"));

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
    fn test_get_mapping_count() {
        let manager = MappingManager::new();
        assert_eq!(manager.get_mapping_count("vs-1"), 0);

        manager.create_mapping(
            "vs-1".to_string(),
            VirtualResourceType::Site,
            123,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::OneToMany,
        );
        manager.create_mapping(
            "vs-1".to_string(),
            VirtualResourceType::Site,
            456,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::OneToMany,
        );

        assert_eq!(manager.get_mapping_count("vs-1"), 2);
    }

    #[test]
    fn test_get_physical_resources_nonexistent() {
        let manager = MappingManager::new();
        let physical = manager.get_physical_resources("nonexistent");
        assert_eq!(physical.len(), 0);
    }

    #[test]
    fn test_get_virtual_resources_nonexistent() {
        let manager = MappingManager::new();
        let virtual_resources = manager.get_virtual_resources(999);
        assert_eq!(virtual_resources.len(), 0);
    }

    #[test]
    fn test_get_tenant_mappings_nonexistent() {
        let manager = MappingManager::new();
        let mappings = manager.get_tenant_mappings("nonexistent");
        assert_eq!(mappings.len(), 0);
    }

    #[test]
    fn test_has_virtual_mapping_nonexistent() {
        let manager = MappingManager::new();
        assert!(!manager.has_virtual_mapping(999));
    }

    #[test]
    fn test_remove_mapping_nonexistent() {
        let manager = MappingManager::new();
        // Should not panic, just return Ok
        assert!(manager.remove_mapping("nonexistent", 999).is_ok());
    }

    #[test]
    fn test_remove_mapping_multiple_physical() {
        let manager = MappingManager::new();
        manager.create_mapping(
            "vs-1".to_string(),
            VirtualResourceType::Site,
            123,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::OneToMany,
        );
        manager.create_mapping(
            "vs-1".to_string(),
            VirtualResourceType::Site,
            456,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::OneToMany,
        );

        assert_eq!(manager.get_mapping_count("vs-1"), 2);
        manager.remove_mapping("vs-1", 123).unwrap();
        assert_eq!(manager.get_mapping_count("vs-1"), 1);
        assert!(manager.has_physical_mapping("vs-1"));
    }

    #[test]
    fn test_many_to_many_mapping() {
        let manager = MappingManager::new();
        manager.create_mapping(
            "vs-1".to_string(),
            VirtualResourceType::Site,
            123,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::ManyToMany,
        );
        manager.create_mapping(
            "vs-2".to_string(),
            VirtualResourceType::Site,
            123,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::ManyToMany,
        );
        manager.create_mapping(
            "vs-1".to_string(),
            VirtualResourceType::Site,
            456,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::ManyToMany,
        );

        let physical = manager.get_physical_resources("vs-1");
        assert_eq!(physical.len(), 2);

        let virtual_resources = manager.get_virtual_resources(123);
        assert_eq!(virtual_resources.len(), 2);
    }

    #[test]
    fn test_mapping_error_display() {
        let error1 = MappingError::MappingNotFound;
        assert_eq!(error1.to_string(), "Mapping not found");

        let error2 = MappingError::InvalidMapping;
        assert_eq!(error2.to_string(), "Invalid mapping");
    }

    #[test]
    fn test_mapping_type_variants() {
        assert_eq!(MappingType::OneToOne, MappingType::OneToOne);
        assert_eq!(MappingType::OneToMany, MappingType::OneToMany);
        assert_eq!(MappingType::ManyToOne, MappingType::ManyToOne);
        assert_eq!(MappingType::ManyToMany, MappingType::ManyToMany);
    }

    #[test]
    fn test_resource_mapping_clone() {
        let mapping = ResourceMapping {
            virtual_id: "vs-1".to_string(),
            virtual_type: VirtualResourceType::Site,
            physical_id: 123,
            physical_type: VirtualResourceType::Site,
            tenant_id: "tenant-1".to_string(),
            mapping_type: MappingType::OneToOne,
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
        };

        let cloned = mapping.clone();
        assert_eq!(cloned.virtual_id, "vs-1");
        assert_eq!(cloned.physical_id, 123);
    }

    #[test]
    fn test_mapping_manager_default() {
        let manager = MappingManager::default();
        assert_eq!(manager.get_mapping_count("vs-1"), 0);
    }

    #[test]
    fn test_has_physical_mapping_empty_list() {
        let manager = MappingManager::new();
        // Create a mapping then remove it to test empty list case
        manager.create_mapping(
            "vs-1".to_string(),
            VirtualResourceType::Site,
            123,
            VirtualResourceType::Site,
            "tenant-1".to_string(),
            MappingType::OneToOne,
        );
        manager.remove_mapping("vs-1", 123).unwrap();
        assert!(!manager.has_physical_mapping("vs-1"));
    }
}

