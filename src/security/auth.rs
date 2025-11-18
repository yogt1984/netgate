use poem::Request;
use crate::error::AppError;

pub const TENANT_HEADER: &str = "X-Tenant-Id";

pub fn extract_tenant_id(req: &Request) -> Result<String, AppError> {
    req.header(TENANT_HEADER)
        .map(|s| s.to_string())
        .ok_or(AppError::Unauthorized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use poem::Request;

    #[test]
    fn test_extract_tenant_id_success() {
        let req = Request::builder()
            .header(TENANT_HEADER, "tenant123")
            .finish();

        let result = extract_tenant_id(&req);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "tenant123");
    }

    #[test]
    fn test_extract_tenant_id_missing() {
        let req = Request::builder().finish();

        let result = extract_tenant_id(&req);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Unauthorized => {}
            _ => panic!("Expected Unauthorized error"),
        }
    }
}

