use crate::netbox::models::{NetBoxDevice, NetBoxSite, SiteStatus};
use chrono::Utc;
use std::collections::HashMap;

/// Enrichment data from external sources
#[derive(Debug, Clone, Default)]
pub struct EnrichmentData {
    /// Geographic data
    pub geographic: Option<GeographicData>,
    /// Contact information
    pub contact: Option<ContactData>,
    /// Business metadata
    pub business: Option<BusinessMetadata>,
    /// Additional tags
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

/// Geographic enrichment data
#[derive(Debug, Clone, Default)]
pub struct GeographicData {
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: Option<String>,
    pub country: Option<String>,
    pub region: Option<String>,
}

/// Contact enrichment data
#[derive(Debug, Clone, Default)]
pub struct ContactData {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub department: Option<String>,
}

/// Business metadata
#[derive(Debug, Clone, Default)]
pub struct BusinessMetadata {
    pub cost_center: Option<String>,
    pub project_code: Option<String>,
    pub environment: Option<String>, // e.g., "production", "staging", "development"
    pub priority: Option<String>,    // e.g., "critical", "high", "medium", "low"
}

/// Object enrichment service
pub struct ObjectEnricher {
    default_tags: Vec<String>,
    environment_tags: HashMap<String, Vec<String>>,
}

impl Default for ObjectEnricher {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectEnricher {
    /// Create a new enricher with default configuration
    pub fn new() -> Self {
        let mut environment_tags = HashMap::new();
        environment_tags.insert("production".to_string(), vec!["prod".to_string(), "critical".to_string()]);
        environment_tags.insert("staging".to_string(), vec!["staging".to_string(), "test".to_string()]);
        environment_tags.insert("development".to_string(), vec!["dev".to_string(), "non-prod".to_string()]);

        Self {
            default_tags: vec!["netgate".to_string(), "enriched".to_string()],
            environment_tags,
        }
    }

    /// Create an enricher with custom configuration
    pub fn with_config(
        default_tags: Vec<String>,
        environment_tags: HashMap<String, Vec<String>>,
    ) -> Self {
        Self {
            default_tags,
            environment_tags,
        }
    }

    /// Enrich a NetBox site with computed fields and metadata
    pub fn enrich_site(
        &self,
        mut site: NetBoxSite,
        enrichment: &EnrichmentData,
    ) -> NetBoxSite {
        // Add computed/derived fields
        self.add_computed_fields_site(&mut site, enrichment);

        // Merge data from multiple sources
        self.merge_enrichment_data_site(&mut site, enrichment);

        // Add metadata/tags based on business logic
        self.add_business_tags_site(&mut site, enrichment);

        site
    }

    /// Enrich a NetBox device with computed fields and metadata
    pub fn enrich_device(
        &self,
        mut device: NetBoxDevice,
        enrichment: &EnrichmentData,
    ) -> NetBoxDevice {
        // Add computed/derived fields
        self.add_computed_fields_device(&mut device, enrichment);

        // Merge data from multiple sources
        self.merge_enrichment_data_device(&mut device, enrichment);

        // Add metadata/tags based on business logic
        self.add_business_tags_device(&mut device, enrichment);

        device
    }

    /// Add computed/derived fields to a site
    fn add_computed_fields_site(&self, site: &mut NetBoxSite, enrichment: &EnrichmentData) {
        // Compute full address if we have geographic data
        if let Some(ref geo) = enrichment.geographic {
            if site.latitude.is_none() {
                site.latitude = Some(geo.latitude);
            }
            if site.longitude.is_none() {
                site.longitude = Some(geo.longitude);
            }
        }

        // Compute derived description if missing
        if site.description.is_none() {
            let mut desc_parts = Vec::new();
            if let Some(ref env) = enrichment.business.as_ref().and_then(|b| b.environment.as_ref()) {
                desc_parts.push(format!("Environment: {}", env));
            }
            if let Some(ref country) = enrichment.geographic.as_ref().and_then(|g| g.country.as_ref()) {
                desc_parts.push(format!("Country: {}", country));
            }
            if !desc_parts.is_empty() {
                site.description = Some(desc_parts.join(", "));
            }
        }

        // Add computed facility code if we have business metadata
        if site.facility.is_none() {
            if let Some(ref business) = enrichment.business {
                if let Some(ref cost_center) = business.cost_center {
                    // Generate facility code from cost center
                    site.facility = Some(format!("FAC-{}", cost_center.to_uppercase()));
                }
            }
        }
    }

    /// Add computed/derived fields to a device
    fn add_computed_fields_device(&self, device: &mut NetBoxDevice, enrichment: &EnrichmentData) {
        // Compute device name if missing and we have metadata
        if device.name.is_none() {
            if let Some(ref business) = enrichment.business {
                if let Some(ref project) = business.project_code {
                    // Generate device name from project code
                    device.name = Some(format!("DEV-{}-{}", project, Utc::now().format("%Y%m%d")));
                }
            }
        }

        // Add computed asset tag if missing
        if device.asset_tag.is_none() {
            if let Some(ref business) = enrichment.business {
                if let Some(ref cost_center) = business.cost_center {
                    device.asset_tag = Some(format!("AT-{}", cost_center));
                }
            }
        }
    }

    /// Merge enrichment data from multiple sources into a site
    fn merge_enrichment_data_site(&self, site: &mut NetBoxSite, enrichment: &EnrichmentData) {
        // Merge geographic data
        if let Some(ref geo) = enrichment.geographic {
            if site.latitude.is_none() {
                site.latitude = Some(geo.latitude);
            }
            if site.longitude.is_none() {
                site.longitude = Some(geo.longitude);
            }
        }

        // Merge contact data
        if let Some(ref contact) = enrichment.contact {
            if site.contact_name.is_none() {
                site.contact_name = contact.name.clone();
            }
            if site.contact_email.is_none() {
                site.contact_email = contact.email.clone();
            }
            if site.contact_phone.is_none() {
                site.contact_phone = contact.phone.clone();
            }
        }

        // Merge business metadata into custom fields
        if let Some(ref business) = enrichment.business {
            let mut custom_fields = site.custom_fields.clone().unwrap_or_default();
            
            if let Some(ref cost_center) = business.cost_center {
                custom_fields["cost_center"] = serde_json::Value::String(cost_center.clone());
            }
            if let Some(ref project) = business.project_code {
                custom_fields["project_code"] = serde_json::Value::String(project.clone());
            }
            if let Some(ref env) = business.environment {
                custom_fields["environment"] = serde_json::Value::String(env.clone());
            }
            if let Some(ref priority) = business.priority {
                custom_fields["priority"] = serde_json::Value::String(priority.clone());
            }

            // Merge additional metadata
            for (key, value) in &enrichment.metadata {
                custom_fields[key] = serde_json::Value::String(value.clone());
            }

            site.custom_fields = Some(custom_fields);
        }
    }

    /// Merge enrichment data from multiple sources into a device
    fn merge_enrichment_data_device(&self, device: &mut NetBoxDevice, enrichment: &EnrichmentData) {
        // Merge business metadata into custom fields
        if let Some(ref business) = enrichment.business {
            let mut custom_fields = device.custom_fields.clone().unwrap_or_default();
            
            if let Some(ref cost_center) = business.cost_center {
                custom_fields["cost_center"] = serde_json::Value::String(cost_center.clone());
            }
            if let Some(ref project) = business.project_code {
                custom_fields["project_code"] = serde_json::Value::String(project.clone());
            }
            if let Some(ref env) = business.environment {
                custom_fields["environment"] = serde_json::Value::String(env.clone());
            }
            if let Some(ref priority) = business.priority {
                custom_fields["priority"] = serde_json::Value::String(priority.clone());
            }

            // Merge additional metadata
            for (key, value) in &enrichment.metadata {
                custom_fields[key] = serde_json::Value::String(value.clone());
            }

            device.custom_fields = Some(custom_fields);
        }
    }

    /// Add business logic-based tags to a site
    fn add_business_tags_site(&self, site: &mut NetBoxSite, enrichment: &EnrichmentData) {
        let mut tags = site.tags.clone().unwrap_or_default();

        // Add default tags
        tags.extend(self.default_tags.clone());

        // Add environment-based tags
        if let Some(ref business) = enrichment.business {
            if let Some(ref env) = business.environment {
                if let Some(env_tags) = self.environment_tags.get(env) {
                    tags.extend(env_tags.clone());
                }
            }

            // Add priority-based tags
            if let Some(ref priority) = business.priority {
                tags.push(format!("priority-{}", priority.to_lowercase()));
            }

            // Add cost center tag
            if let Some(ref cost_center) = business.cost_center {
                tags.push(format!("cost-center-{}", cost_center.to_lowercase()));
            }
        }

        // Add geographic tags
        if let Some(ref geo) = enrichment.geographic {
            if let Some(ref country) = geo.country {
                tags.push(format!("country-{}", country.to_lowercase()));
            }
            if let Some(ref region) = geo.region {
                tags.push(format!("region-{}", region.to_lowercase()));
            }
        }

        // Add enrichment tags
        tags.extend(enrichment.tags.clone());

        // Add status-based tags
        if let Some(ref status) = site.status {
            match status {
                SiteStatus::Active => tags.push("status-active".to_string()),
                SiteStatus::Planned => tags.push("status-planned".to_string()),
                SiteStatus::Retired => tags.push("status-retired".to_string()),
                SiteStatus::Staging => tags.push("status-staging".to_string()),
            }
        }

        // Deduplicate and sort
        tags.sort();
        tags.dedup();
        site.tags = Some(tags);
    }

    /// Add business logic-based tags to a device
    fn add_business_tags_device(&self, device: &mut NetBoxDevice, enrichment: &EnrichmentData) {
        let mut tags = device.tags.clone().unwrap_or_default();

        // Add default tags
        tags.extend(self.default_tags.clone());

        // Add environment-based tags
        if let Some(ref business) = enrichment.business {
            if let Some(ref env) = business.environment {
                if let Some(env_tags) = self.environment_tags.get(env) {
                    tags.extend(env_tags.clone());
                }
            }

            // Add priority-based tags
            if let Some(ref priority) = business.priority {
                tags.push(format!("priority-{}", priority.to_lowercase()));
            }

            // Add cost center tag
            if let Some(ref cost_center) = business.cost_center {
                tags.push(format!("cost-center-{}", cost_center.to_lowercase()));
            }
        }

        // Add enrichment tags
        tags.extend(enrichment.tags.clone());

        // Deduplicate and sort
        tags.sort();
        tags.dedup();
        device.tags = Some(tags);
    }

    /// Compute derived status based on business rules
    pub fn compute_status(&self, enrichment: &EnrichmentData) -> Option<SiteStatus> {
        if let Some(ref business) = enrichment.business {
            if let Some(ref env) = business.environment {
                match env.to_lowercase().as_str() {
                    "production" => return Some(SiteStatus::Active),
                    "staging" => return Some(SiteStatus::Staging),
                    "development" => return Some(SiteStatus::Planned),
                    _ => {}
                }
            }
        }
        None
    }

    /// Merge multiple enrichment data sources
    pub fn merge_enrichment_sources(sources: Vec<EnrichmentData>) -> EnrichmentData {
        let mut merged = EnrichmentData::default();

        for source in sources {
            // Merge geographic data (take first non-None)
            if merged.geographic.is_none() && source.geographic.is_some() {
                merged.geographic = source.geographic;
            }

            // Merge contact data (take first non-None)
            if merged.contact.is_none() && source.contact.is_some() {
                merged.contact = source.contact;
            }

            // Merge business metadata (take first non-None)
            if merged.business.is_none() && source.business.is_some() {
                merged.business = source.business;
            }

            // Merge tags (union)
            merged.tags.extend(source.tags);
            merged.tags.sort();
            merged.tags.dedup();

            // Merge metadata (later sources override earlier)
            for (key, value) in source.metadata {
                merged.metadata.insert(key, value);
            }
        }

        merged
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_site() -> NetBoxSite {
        NetBoxSite {
            id: Some(1),
            name: "Test Site".to_string(),
            description: None,
            slug: None,
            status: Some(SiteStatus::Active),
            region: None,
            tenant: None,
            facility: None,
            physical_address: Some("123 Main St".to_string()),
            shipping_address: None,
            latitude: None,
            longitude: None,
            contact_name: None,
            contact_phone: None,
            contact_email: None,
            comments: None,
            tags: None,
            custom_fields: None,
            created: None,
            last_updated: None,
        }
    }

    fn create_test_device() -> NetBoxDevice {
        NetBoxDevice {
            id: Some(1),
            name: None,
            device_type: Some(1),
            device_role: Some(1),
            tenant: None,
            platform: None,
            serial: None,
            asset_tag: None,
            site: Some(1),
            location: None,
            rack: None,
            position: None,
            face: None,
            status: None,
            primary_ip4: None,
            primary_ip6: None,
            cluster: None,
            virtual_chassis: None,
            vc_position: None,
            vc_priority: None,
            comments: None,
            tags: None,
            custom_fields: None,
            created: None,
            last_updated: None,
        }
    }

    #[test]
    fn test_enrich_site_with_geographic_data() {
        let enricher = ObjectEnricher::new();
        let mut site = create_test_site();

        let enrichment = EnrichmentData {
            geographic: Some(GeographicData {
                latitude: 40.7128,
                longitude: -74.0060,
                timezone: Some("America/New_York".to_string()),
                country: Some("USA".to_string()),
                region: Some("North America".to_string()),
            }),
            ..Default::default()
        };

        let enriched = enricher.enrich_site(site, &enrichment);

        assert_eq!(enriched.latitude, Some(40.7128));
        assert_eq!(enriched.longitude, Some(-74.0060));
        assert!(enriched.tags.as_ref().unwrap().contains(&"country-usa".to_string()));
        assert!(enriched.tags.as_ref().unwrap().contains(&"region-north america".to_string()));
    }

    #[test]
    fn test_enrich_site_with_contact_data() {
        let enricher = ObjectEnricher::new();
        let mut site = create_test_site();

        let enrichment = EnrichmentData {
            contact: Some(ContactData {
                name: Some("John Doe".to_string()),
                email: Some("john@example.com".to_string()),
                phone: Some("+1-555-0123".to_string()),
                department: Some("IT".to_string()),
            }),
            ..Default::default()
        };

        let enriched = enricher.enrich_site(site, &enrichment);

        assert_eq!(enriched.contact_name, Some("John Doe".to_string()));
        assert_eq!(enriched.contact_email, Some("john@example.com".to_string()));
        assert_eq!(enriched.contact_phone, Some("+1-555-0123".to_string()));
    }

    #[test]
    fn test_enrich_site_with_business_metadata() {
        let enricher = ObjectEnricher::new();
        let mut site = create_test_site();

        let enrichment = EnrichmentData {
            business: Some(BusinessMetadata {
                cost_center: Some("CC-123".to_string()),
                project_code: Some("PROJ-456".to_string()),
                environment: Some("production".to_string()),
                priority: Some("critical".to_string()),
            }),
            ..Default::default()
        };

        let enriched = enricher.enrich_site(site, &enrichment);

        // Check computed facility
        assert_eq!(enriched.facility, Some("FAC-CC-123".to_string()));

        // Check custom fields
        let custom_fields = enriched.custom_fields.as_ref().unwrap();
        assert_eq!(custom_fields["cost_center"], "CC-123");
        assert_eq!(custom_fields["project_code"], "PROJ-456");
        assert_eq!(custom_fields["environment"], "production");
        assert_eq!(custom_fields["priority"], "critical");

        // Check tags
        let tags = enriched.tags.as_ref().unwrap();
        assert!(tags.contains(&"prod".to_string()));
        assert!(tags.contains(&"critical".to_string()));
        assert!(tags.contains(&"priority-critical".to_string()));
        assert!(tags.contains(&"cost-center-cc-123".to_string()));
    }

    #[test]
    fn test_enrich_site_computed_description() {
        let enricher = ObjectEnricher::new();
        let mut site = create_test_site();
        site.description = None;

        let enrichment = EnrichmentData {
            business: Some(BusinessMetadata {
                environment: Some("production".to_string()),
                ..Default::default()
            }),
            geographic: Some(GeographicData {
                latitude: 0.0,
                longitude: 0.0,
                country: Some("USA".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let enriched = enricher.enrich_site(site, &enrichment);

        assert!(enriched.description.is_some());
        let desc = enriched.description.unwrap();
        assert!(desc.contains("Environment: production"));
        assert!(desc.contains("Country: USA"));
    }

    #[test]
    fn test_enrich_device_with_business_metadata() {
        let enricher = ObjectEnricher::new();
        let mut device = create_test_device();

        let enrichment = EnrichmentData {
            business: Some(BusinessMetadata {
                cost_center: Some("CC-789".to_string()),
                project_code: Some("PROJ-ABC".to_string()),
                environment: Some("staging".to_string()),
                priority: Some("high".to_string()),
            }),
            ..Default::default()
        };

        let enriched = enricher.enrich_device(device, &enrichment);

        // Check computed name
        assert!(enriched.name.is_some());
        assert!(enriched.name.as_ref().unwrap().starts_with("DEV-PROJ-ABC-"));

        // Check computed asset tag
        assert_eq!(enriched.asset_tag, Some("AT-CC-789".to_string()));

        // Check custom fields
        let custom_fields = enriched.custom_fields.as_ref().unwrap();
        assert_eq!(custom_fields["cost_center"], "CC-789");
        assert_eq!(custom_fields["environment"], "staging");

        // Check tags
        let tags = enriched.tags.as_ref().unwrap();
        assert!(tags.contains(&"staging".to_string()));
        assert!(tags.contains(&"priority-high".to_string()));
    }

    #[test]
    fn test_enrich_site_status_based_tags() {
        let enricher = ObjectEnricher::new();
        let mut site = create_test_site();
        site.status = Some(SiteStatus::Planned);

        let enrichment = EnrichmentData::default();
        let enriched = enricher.enrich_site(site, &enrichment);

        let tags = enriched.tags.as_ref().unwrap();
        assert!(tags.contains(&"status-planned".to_string()));
    }

    #[test]
    fn test_compute_status_from_environment() {
        let enricher = ObjectEnricher::new();

        let prod_enrichment = EnrichmentData {
            business: Some(BusinessMetadata {
                environment: Some("production".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(enricher.compute_status(&prod_enrichment), Some(SiteStatus::Active));

        let staging_enrichment = EnrichmentData {
            business: Some(BusinessMetadata {
                environment: Some("staging".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(enricher.compute_status(&staging_enrichment), Some(SiteStatus::Staging));

        let dev_enrichment = EnrichmentData {
            business: Some(BusinessMetadata {
                environment: Some("development".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(enricher.compute_status(&dev_enrichment), Some(SiteStatus::Planned));
    }

    #[test]
    fn test_merge_enrichment_sources() {
        let source1 = EnrichmentData {
            geographic: Some(GeographicData {
                latitude: 40.7128,
                longitude: -74.0060,
                timezone: None,
                country: Some("USA".to_string()),
                region: None,
            }),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
            metadata: {
                let mut m = HashMap::new();
                m.insert("key1".to_string(), "value1".to_string());
                m
            },
            ..Default::default()
        };

        let source2 = EnrichmentData {
            contact: Some(ContactData {
                name: Some("Jane Doe".to_string()),
                email: None,
                phone: None,
                department: None,
            }),
            tags: vec!["tag2".to_string(), "tag3".to_string()],
            metadata: {
                let mut m = HashMap::new();
                m.insert("key2".to_string(), "value2".to_string());
                m.insert("key1".to_string(), "value1-override".to_string());
                m
            },
            ..Default::default()
        };

        let merged = ObjectEnricher::merge_enrichment_sources(vec![source1, source2]);

        assert!(merged.geographic.is_some());
        assert!(merged.contact.is_some());
        assert_eq!(merged.contact.unwrap().name, Some("Jane Doe".to_string()));

        // Tags should be merged and deduplicated
        assert_eq!(merged.tags.len(), 3);
        assert!(merged.tags.contains(&"tag1".to_string()));
        assert!(merged.tags.contains(&"tag2".to_string()));
        assert!(merged.tags.contains(&"tag3".to_string()));

        // Metadata: later source overrides earlier
        assert_eq!(merged.metadata.get("key1"), Some(&"value1-override".to_string()));
        assert_eq!(merged.metadata.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_enrich_site_preserves_existing_data() {
        let enricher = ObjectEnricher::new();
        let mut site = create_test_site();
        site.latitude = Some(50.0);
        site.longitude = Some(10.0);
        site.contact_name = Some("Existing Contact".to_string());
        site.tags = Some(vec!["existing-tag".to_string()]);

        let enrichment = EnrichmentData {
            geographic: Some(GeographicData {
                latitude: 40.7128,
                longitude: -74.0060,
                timezone: None,
                country: None,
                region: None,
            }),
            contact: Some(ContactData {
                name: Some("New Contact".to_string()),
                email: None,
                phone: None,
                department: None,
            }),
            ..Default::default()
        };

        let enriched = enricher.enrich_site(site, &enrichment);

        // Existing data should be preserved
        assert_eq!(enriched.latitude, Some(50.0));
        assert_eq!(enriched.longitude, Some(10.0));
        assert_eq!(enriched.contact_name, Some("Existing Contact".to_string()));

        // Tags should be merged
        let tags = enriched.tags.as_ref().unwrap();
        assert!(tags.contains(&"existing-tag".to_string()));
        assert!(tags.contains(&"netgate".to_string()));
    }

    #[test]
    fn test_enrich_site_with_custom_tags() {
        let enricher = ObjectEnricher::new();
        let mut site = create_test_site();

        let enrichment = EnrichmentData {
            tags: vec!["custom-tag1".to_string(), "custom-tag2".to_string()],
            ..Default::default()
        };

        let enriched = enricher.enrich_site(site, &enrichment);

        let tags = enriched.tags.as_ref().unwrap();
        assert!(tags.contains(&"custom-tag1".to_string()));
        assert!(tags.contains(&"custom-tag2".to_string()));
        assert!(tags.contains(&"netgate".to_string()));
    }

    #[test]
    fn test_enrich_device_computed_name() {
        let enricher = ObjectEnricher::new();
        let mut device = create_test_device();
        device.name = None;

        let enrichment = EnrichmentData {
            business: Some(BusinessMetadata {
                project_code: Some("TEST-PROJ".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let enriched = enricher.enrich_device(device, &enrichment);

        assert!(enriched.name.is_some());
        let name = enriched.name.unwrap();
        assert!(name.starts_with("DEV-TEST-PROJ-"));
    }

    #[test]
    fn test_enrichment_with_empty_data() {
        let enricher = ObjectEnricher::new();
        let site = create_test_site();
        let enrichment = EnrichmentData::default();

        let enriched = enricher.enrich_site(site, &enrichment);

        // Should still add default tags
        assert!(enriched.tags.is_some());
        let tags = enriched.tags.unwrap();
        assert!(tags.contains(&"netgate".to_string()));
        assert!(tags.contains(&"enriched".to_string()));
    }
}

