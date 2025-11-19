use crate::domain::CreateSiteOrder;
use crate::netbox::models::{CreateSiteRequest, SiteStatus};

/// Transform a CreateSiteOrder to a NetBox CreateSiteRequest
pub struct OrderTransformer {
    default_status: SiteStatus,
}

impl Default for OrderTransformer {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderTransformer {
    /// Create a new transformer with default settings
    pub fn new() -> Self {
        Self {
            default_status: SiteStatus::Planned,
        }
    }

    /// Create a transformer with custom default status
    pub fn with_default_status(status: SiteStatus) -> Self {
        Self {
            default_status: status,
        }
    }

    /// Transform a CreateSiteOrder to CreateSiteRequest
    pub fn transform_site_order(
        &self,
        order: CreateSiteOrder,
        tenant_id: Option<i32>,
    ) -> CreateSiteRequest {
        // Generate slug from name (lowercase, replace spaces with hyphens, remove special chars)
        let slug = self.generate_slug(&order.name);

        CreateSiteRequest {
            name: order.name,
            slug: Some(slug),
            description: order.description,
            status: Some(self.default_status.clone()),
            region: None, // Can be enriched later based on business rules
            tenant: tenant_id,
            facility: None,
            physical_address: order.address.clone(),
            shipping_address: order.address,
            latitude: None, // Can be enriched from address geocoding
            longitude: None,
            contact_name: None,
            contact_phone: None,
            contact_email: None,
            comments: Some(format!("Created via NetGate order portal")),
            tags: Some(vec!["netgate".to_string(), "order-portal".to_string()]),
        }
    }

    /// Generate a URL-friendly slug from a name
    fn generate_slug(&self, name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| match c {
                'a'..='z' | '0'..='9' => c,
                ' ' | '-' | '_' => '-',
                _ => '-',
            })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
            .chars()
            .take(50) // NetBox slug max length
            .collect()
    }

    /// Enrich site request with additional business logic
    pub fn enrich_site_request(
        &self,
        mut request: CreateSiteRequest,
        enrichment_data: &SiteEnrichmentData,
    ) -> CreateSiteRequest {
        if let Some(ref region) = enrichment_data.region_id {
            request.region = Some(*region);
        }

        if let Some(ref facility) = enrichment_data.facility {
            request.facility = Some(facility.clone());
        }

        if let Some(ref contact) = enrichment_data.contact_name {
            request.contact_name = Some(contact.clone());
        }

        if let Some(ref email) = enrichment_data.contact_email {
            request.contact_email = Some(email.clone());
        }

        if let Some(ref phone) = enrichment_data.contact_phone {
            request.contact_phone = Some(phone.clone());
        }

        // Merge tags
        if let Some(ref enrichment_tags) = enrichment_data.tags {
            let mut tags = request.tags.unwrap_or_default();
            tags.extend(enrichment_tags.clone());
            tags.sort();
            tags.dedup();
            request.tags = Some(tags);
        }

        request
    }
}

/// Data for enriching site requests
#[derive(Debug, Clone, Default)]
pub struct SiteEnrichmentData {
    pub region_id: Option<i32>,
    pub facility: Option<String>,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_site_order_basic() {
        let transformer = OrderTransformer::new();
        let order = CreateSiteOrder {
            name: "Test Site".to_string(),
            description: Some("Test Description".to_string()),
            address: Some("123 Main St".to_string()),
        };

        let request = transformer.transform_site_order(order, Some(10));

        assert_eq!(request.name, "Test Site");
        assert_eq!(request.description, Some("Test Description".to_string()));
        assert_eq!(request.physical_address, Some("123 Main St".to_string()));
        assert_eq!(request.shipping_address, Some("123 Main St".to_string()));
        assert_eq!(request.tenant, Some(10));
        assert_eq!(request.status, Some(SiteStatus::Planned));
        assert!(request.slug.is_some());
        assert_eq!(request.slug.unwrap(), "test-site");
    }

    #[test]
    fn test_generate_slug() {
        let transformer = OrderTransformer::new();
        
        assert_eq!(transformer.generate_slug("Test Site"), "test-site");
        assert_eq!(transformer.generate_slug("Site-Name_123"), "site-name-123");
        assert_eq!(transformer.generate_slug("Site  Name"), "site-name");
        assert_eq!(transformer.generate_slug("UPPERCASE"), "uppercase");
    }

    #[test]
    fn test_transform_with_custom_status() {
        let transformer = OrderTransformer::with_default_status(SiteStatus::Active);
        let order = CreateSiteOrder {
            name: "Active Site".to_string(),
            description: None,
            address: None,
        };

        let request = transformer.transform_site_order(order, None);
        assert_eq!(request.status, Some(SiteStatus::Active));
    }

    #[test]
    fn test_enrich_site_request() {
        let transformer = OrderTransformer::new();
        let order = CreateSiteOrder {
            name: "Test Site".to_string(),
            description: None,
            address: None,
        };

        let mut request = transformer.transform_site_order(order, None);

        let enrichment = SiteEnrichmentData {
            region_id: Some(5),
            facility: Some("DC-1".to_string()),
            contact_name: Some("John Doe".to_string()),
            contact_email: Some("john@example.com".to_string()),
            contact_phone: Some("+1-555-0123".to_string()),
            tags: Some(vec!["production".to_string()]),
        };

        let enriched = transformer.enrich_site_request(request, &enrichment);

        assert_eq!(enriched.region, Some(5));
        assert_eq!(enriched.facility, Some("DC-1".to_string()));
        assert_eq!(enriched.contact_name, Some("John Doe".to_string()));
        assert_eq!(enriched.contact_email, Some("john@example.com".to_string()));
        assert_eq!(enriched.contact_phone, Some("+1-555-0123".to_string()));
        assert!(enriched.tags.as_ref().unwrap().contains(&"production".to_string()));
        assert!(enriched.tags.as_ref().unwrap().contains(&"netgate".to_string()));
    }
}

