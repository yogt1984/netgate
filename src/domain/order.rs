use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct CreateSiteOrder {
    pub name: String,
    pub description: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct Site {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub address: Option<String>,
    pub tenant_id: String,
}

impl Site {
    pub fn from_order(order: CreateSiteOrder, tenant_id: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: order.name,
            description: order.description,
            address: order.address,
            tenant_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_site_from_order() {
        let order = CreateSiteOrder {
            name: "Test Site".to_string(),
            description: Some("Test Description".to_string()),
            address: Some("123 Test St".to_string()),
        };

        let site = Site::from_order(order, "tenant1".to_string());

        assert_eq!(site.name, "Test Site");
        assert_eq!(site.description, Some("Test Description".to_string()));
        assert_eq!(site.address, Some("123 Test St".to_string()));
        assert_eq!(site.tenant_id, "tenant1");
        assert!(!site.id.is_empty());
    }

    #[test]
    fn test_site_from_order_with_optional_fields() {
        let order = CreateSiteOrder {
            name: "Minimal Site".to_string(),
            description: None,
            address: None,
        };

        let site = Site::from_order(order, "tenant2".to_string());

        assert_eq!(site.name, "Minimal Site");
        assert_eq!(site.description, None);
        assert_eq!(site.address, None);
        assert_eq!(site.tenant_id, "tenant2");
    }

    #[test]
    fn test_site_id_is_unique() {
        let order = CreateSiteOrder {
            name: "Site".to_string(),
            description: None,
            address: None,
        };

        let site1 = Site::from_order(order.clone(), "tenant1".to_string());
        let site2 = Site::from_order(order, "tenant1".to_string());

        assert_ne!(site1.id, site2.id);
    }
}

