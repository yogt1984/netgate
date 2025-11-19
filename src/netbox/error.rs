use crate::resilience::retry::RetryableError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetBoxError {
    #[error("NetBox API error: {0}")]
    ApiError(String),

    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Unexpected response: {0}")]
    UnexpectedResponse(String),
}

impl RetryableError for NetBoxError {
    fn is_retryable(&self) -> bool {
        match self {
            // Network errors are retryable
            NetBoxError::NetworkError(_) => true,
            // Server errors (5xx) are retryable
            NetBoxError::ApiError(msg) => {
                msg.contains("500") || msg.contains("502") || msg.contains("503") || msg.contains("504")
            }
            // Authentication errors are not retryable (need to fix credentials)
            NetBoxError::AuthenticationError(_) => false,
            // Not found is not retryable
            NetBoxError::NotFound(_) => false,
            // Validation errors are not retryable (bad request)
            NetBoxError::ValidationError(_) => false,
            // Serialization errors are not retryable
            NetBoxError::SerializationError(_) => false,
            // Invalid URL is not retryable
            NetBoxError::InvalidUrl(_) => false,
            // Unexpected response might be retryable
            NetBoxError::UnexpectedResponse(_) => true,
        }
    }
}

impl NetBoxError {
    pub fn from_status_code(status: u16, message: String) -> Self {
        match status {
            401 | 403 => NetBoxError::AuthenticationError(message),
            404 => NetBoxError::NotFound(message),
            400 | 422 => NetBoxError::ValidationError(message),
            _ => NetBoxError::ApiError(format!("HTTP {}: {}", status, message)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_status_code_401() {
        let error = NetBoxError::from_status_code(401, "Unauthorized".to_string());
        match error {
            NetBoxError::AuthenticationError(msg) => assert_eq!(msg, "Unauthorized"),
            _ => panic!("Expected AuthenticationError"),
        }
    }

    #[test]
    fn test_from_status_code_403() {
        let error = NetBoxError::from_status_code(403, "Forbidden".to_string());
        match error {
            NetBoxError::AuthenticationError(msg) => assert_eq!(msg, "Forbidden"),
            _ => panic!("Expected AuthenticationError"),
        }
    }

    #[test]
    fn test_from_status_code_404() {
        let error = NetBoxError::from_status_code(404, "Not found".to_string());
        match error {
            NetBoxError::NotFound(msg) => assert_eq!(msg, "Not found"),
            _ => panic!("Expected NotFound"),
        }
    }

    #[test]
    fn test_from_status_code_400() {
        let error = NetBoxError::from_status_code(400, "Bad request".to_string());
        match error {
            NetBoxError::ValidationError(msg) => assert_eq!(msg, "Bad request"),
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_from_status_code_422() {
        let error = NetBoxError::from_status_code(422, "Unprocessable".to_string());
        match error {
            NetBoxError::ValidationError(msg) => assert_eq!(msg, "Unprocessable"),
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_from_status_code_500() {
        let error = NetBoxError::from_status_code(500, "Server error".to_string());
        match error {
            NetBoxError::ApiError(msg) => assert!(msg.contains("500") && msg.contains("Server error")),
            _ => panic!("Expected ApiError"),
        }
    }
}

