use poem::{
    Endpoint, Middleware, Request, Result as PoemResult,
};
use tracing::{info_span, Instrument};
use uuid::Uuid;

/// Middleware to add request ID and correlation ID to requests
pub struct RequestTracingMiddleware;

impl<E: Endpoint> Middleware<E> for RequestTracingMiddleware {
    type Output = RequestTracingEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        RequestTracingEndpoint { ep }
    }
}

/// Endpoint wrapper that adds request tracing
pub struct RequestTracingEndpoint<E> {
    ep: E,
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for RequestTracingEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> PoemResult<Self::Output> {
        // Generate request ID and correlation ID
        let request_id = Uuid::new_v4().to_string();
        let correlation_id = req
            .header("X-Correlation-Id")
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // Add to request headers for downstream use
        req.headers_mut().insert(
            "X-Request-Id",
            request_id.parse().unwrap(),
        );
        req.headers_mut().insert(
            "X-Correlation-Id",
            correlation_id.parse().unwrap(),
        );

        // Create tracing span with request context
        let span = info_span!(
            "http_request",
            request_id = %request_id,
            correlation_id = %correlation_id,
            method = %req.method(),
            path = %req.uri().path(),
        );

        // Execute endpoint within the span
        self.ep.call(req).instrument(span).await
    }
}

/// Extract request ID from request
pub fn extract_request_id(req: &Request) -> Option<String> {
    req.header("X-Request-Id").map(|s| s.to_string())
}

/// Extract correlation ID from request
pub fn extract_correlation_id(req: &Request) -> Option<String> {
    req.header("X-Correlation-Id").map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use poem::EndpointExt;

    #[tokio::test]
    async fn test_request_id_extraction() {
        let req = Request::builder()
            .header("X-Request-Id", "test-request-id")
            .finish();
        
        let request_id = extract_request_id(&req);
        assert_eq!(request_id, Some("test-request-id".to_string()));
    }

    #[tokio::test]
    async fn test_correlation_id_extraction() {
        let req = Request::builder()
            .header("X-Correlation-Id", "test-correlation-id")
            .finish();
        
        let correlation_id = extract_correlation_id(&req);
        assert_eq!(correlation_id, Some("test-correlation-id".to_string()));
    }

    #[tokio::test]
    async fn test_missing_request_id() {
        let req = Request::builder().finish();
        let request_id = extract_request_id(&req);
        assert!(request_id.is_none());
    }
}

