use crate::business::plugin::{NetBoxResource, NetBoxResourceRequest, OrderPayload, OrderProcessor};
use crate::business::enrichment::EnrichmentData;
use crate::business::{ObjectEnricher, OrderTransformer, OrderValidator};
use crate::error::AppError;
use crate::netbox::ResilientNetBoxClient;
use async_trait::async_trait;
use std::sync::Arc;

/// Site order processor implementation
pub struct SiteOrderProcessor {
    validator: OrderValidator,
    transformer: OrderTransformer,
    enricher: ObjectEnricher,
}

impl SiteOrderProcessor {
    pub fn new() -> Self {
        Self {
            validator: OrderValidator::new(),
            transformer: OrderTransformer::new(),
            enricher: ObjectEnricher::new(),
        }
    }
}

impl Default for SiteOrderProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OrderProcessor for SiteOrderProcessor {
    fn order_type(&self) -> &'static str {
        "site"
    }

    fn validate(&self, order: &OrderPayload) -> Result<(), AppError> {
        match order {
            OrderPayload::Site(site_order) => {
                self.validator.validate_site_order(site_order)
                    .map_err(|e| AppError::ValidationError(e.to_string()))
            }
        }
    }

    fn transform(
        &self,
        order: OrderPayload,
        tenant_id: Option<i32>,
    ) -> Result<NetBoxResourceRequest, AppError> {
        match order {
            OrderPayload::Site(site_order) => {
                let request = self.transformer.transform_site_order(site_order, tenant_id);
                Ok(NetBoxResourceRequest::Site(request))
            }
        }
    }

    fn enrich_request(
        &self,
        request: &mut NetBoxResourceRequest,
        _enrichment_data: &EnrichmentData,
    ) -> Result<(), AppError> {
        match request {
            NetBoxResourceRequest::Site(site_request) => {
                let mut tags = site_request.tags.take().unwrap_or_default();
                tags.push("netgate".to_string());
                tags.push("enriched".to_string());
                site_request.tags = Some(tags);
                Ok(())
            }
        }
    }

    async fn create_resource(
        &self,
        client: &Arc<ResilientNetBoxClient>,
        request: NetBoxResourceRequest,
    ) -> Result<NetBoxResource, AppError> {
        match request {
            NetBoxResourceRequest::Site(site_request) => {
                let site = client.create_site(site_request).await?;
                Ok(NetBoxResource::Site(site))
            }
        }
    }

    fn enrich_resource(
        &self,
        resource: NetBoxResource,
        enrichment_data: &EnrichmentData,
    ) -> NetBoxResource {
        match resource {
            NetBoxResource::Site(site) => {
                let enriched = self.enricher.enrich_site(site, enrichment_data);
                NetBoxResource::Site(enriched)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::CreateSiteOrder;

    #[test]
    fn test_site_order_processor_creation() {
        let processor = SiteOrderProcessor::new();
        assert_eq!(processor.order_type(), "site");
    }

    #[test]
    fn test_site_order_processor_validate() {
        let processor = SiteOrderProcessor::new();
        let order = OrderPayload::Site(CreateSiteOrder {
            name: "Test Site".to_string(),
            description: Some("Test".to_string()),
            address: None,
        });
        
        let result = processor.validate(&order);
        assert!(result.is_ok());
    }

    #[test]
    fn test_site_order_processor_validate_failure() {
        let processor = SiteOrderProcessor::new();
        let order = OrderPayload::Site(CreateSiteOrder {
            name: "".to_string(), // Invalid: empty name
            description: None,
            address: None,
        });
        
        let result = processor.validate(&order);
        assert!(result.is_err());
    }

    #[test]
    fn test_site_order_processor_transform() {
        let processor = SiteOrderProcessor::new();
        let order = OrderPayload::Site(CreateSiteOrder {
            name: "Test Site".to_string(),
            description: Some("Test".to_string()),
            address: None,
        });
        
        let result = processor.transform(order, None);
        assert!(result.is_ok());
        match result.unwrap() {
            NetBoxResourceRequest::Site(_) => {}
        }
    }
}

