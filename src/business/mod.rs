pub mod enrichment;
pub mod extensible_order_service;
pub mod order_service;
pub mod plugin;
pub mod processors;
pub mod transformation;
pub mod validation;
pub mod workflow;

pub use enrichment::*;
// Note: extensible_order_service and order_service both export ProcessedOrderResult and OrderStatus
// We only export from order_service to avoid ambiguity
pub use order_service::*;
pub use transformation::*;
pub use validation::*;
pub use workflow::*;

// Re-export plugin and processor types explicitly (public API)
#[allow(unused_imports)] // These are public APIs for external use
pub use plugin::{OrderPayload, OrderProcessor, OrderType, OrderTypeRegistry, NetBoxResource, NetBoxResourceRequest};
#[allow(unused_imports)]
pub use processors::SiteOrderProcessor;
#[allow(unused_imports)]
pub use extensible_order_service::{ExtensibleOrderService, ExtensibleOrderServiceBuilder};

