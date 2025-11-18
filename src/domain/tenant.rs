use std::collections::HashMap;
use std::sync::RwLock;

use crate::domain::Site;

pub type TenantId = String;

pub struct TenantStore {
    // Map from tenant_id to Vec<Site>
    sites: RwLock<HashMap<TenantId, Vec<Site>>>,
}

impl TenantStore {
    pub fn new() -> Self {
        Self {
            sites: RwLock::new(HashMap::new()),
        }
    }

    pub fn add_site(&self, tenant_id: TenantId, site: Site) {
        let mut sites = self.sites.write().unwrap();
        sites.entry(tenant_id).or_insert_with(Vec::new).push(site);
    }

    pub fn get_sites(&self, tenant_id: &TenantId) -> Vec<Site> {
        let sites = self.sites.read().unwrap();
        sites.get(tenant_id).cloned().unwrap_or_default()
    }
}

impl Default for TenantStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Site;

    fn create_test_site(id: &str, name: &str, tenant_id: &str) -> Site {
        Site {
            id: id.to_string(),
            name: name.to_string(),
            description: None,
            address: None,
            tenant_id: tenant_id.to_string(),
        }
    }

    #[test]
    fn test_tenant_store_new() {
        let store = TenantStore::new();
        let sites = store.get_sites(&"tenant1".to_string());
        assert!(sites.is_empty());
    }

    #[test]
    fn test_add_and_get_sites() {
        let store = TenantStore::new();
        let site1 = create_test_site("1", "Site 1", "tenant1");
        let site2 = create_test_site("2", "Site 2", "tenant1");

        store.add_site("tenant1".to_string(), site1.clone());
        store.add_site("tenant1".to_string(), site2.clone());

        let sites = store.get_sites(&"tenant1".to_string());
        assert_eq!(sites.len(), 2);
        assert_eq!(sites[0].id, "1");
        assert_eq!(sites[1].id, "2");
    }

    #[test]
    fn test_tenant_isolation() {
        let store = TenantStore::new();
        let site1 = create_test_site("1", "Site 1", "tenant1");
        let site2 = create_test_site("2", "Site 2", "tenant2");

        store.add_site("tenant1".to_string(), site1);
        store.add_site("tenant2".to_string(), site2);

        let tenant1_sites = store.get_sites(&"tenant1".to_string());
        let tenant2_sites = store.get_sites(&"tenant2".to_string());

        assert_eq!(tenant1_sites.len(), 1);
        assert_eq!(tenant2_sites.len(), 1);
        assert_eq!(tenant1_sites[0].tenant_id, "tenant1");
        assert_eq!(tenant2_sites[0].tenant_id, "tenant2");
    }

    #[test]
    fn test_get_sites_for_nonexistent_tenant() {
        let store = TenantStore::new();
        let sites = store.get_sites(&"nonexistent".to_string());
        assert!(sites.is_empty());
    }
}

