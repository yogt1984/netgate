use crate::domain::CreateSiteOrder;
use std::collections::HashSet;

/// Validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    EmptyName,
    NameTooLong,
    InvalidNameFormat,
    DescriptionTooLong,
    AddressTooLong,
    InvalidCharacters(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::EmptyName => write!(f, "Site name cannot be empty"),
            ValidationError::NameTooLong => write!(f, "Site name exceeds maximum length of 100 characters"),
            ValidationError::InvalidNameFormat => write!(f, "Site name contains invalid characters"),
            ValidationError::DescriptionTooLong => write!(f, "Description exceeds maximum length of 500 characters"),
            ValidationError::AddressTooLong => write!(f, "Address exceeds maximum length of 200 characters"),
            ValidationError::InvalidCharacters(field) => write!(f, "Invalid characters in field: {}", field),
        }
    }
}

/// Business rules for order validation
pub struct OrderValidator {
    max_name_length: usize,
    max_description_length: usize,
    max_address_length: usize,
    allowed_name_chars: HashSet<char>,
}

impl Default for OrderValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderValidator {
    /// Create a new validator with default rules
    pub fn new() -> Self {
        let mut allowed_chars = HashSet::new();
        // Allow alphanumeric, spaces, hyphens, underscores, dots, and parentheses
        for c in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 -_.()".chars() {
            allowed_chars.insert(c);
        }

        Self {
            max_name_length: 100,
            max_description_length: 500,
            max_address_length: 200,
            allowed_name_chars: allowed_chars,
        }
    }

    /// Create a validator with custom rules
    pub fn with_rules(
        max_name_length: usize,
        max_description_length: usize,
        max_address_length: usize,
    ) -> Self {
        let mut allowed_chars = HashSet::new();
        for c in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 -_.()".chars() {
            allowed_chars.insert(c);
        }

        Self {
            max_name_length,
            max_description_length,
            max_address_length,
            allowed_name_chars: allowed_chars,
        }
    }

    /// Validate a site order
    pub fn validate_site_order(&self, order: &CreateSiteOrder) -> Result<(), ValidationError> {
        // Validate name
        self.validate_name(&order.name)?;

        // Validate description if present
        if let Some(ref desc) = order.description {
            self.validate_description(desc)?;
        }

        // Validate address if present
        if let Some(ref addr) = order.address {
            self.validate_address(addr)?;
        }

        Ok(())
    }

    /// Validate site name
    pub fn validate_name(&self, name: &str) -> Result<(), ValidationError> {
        let trimmed = name.trim();

        if trimmed.is_empty() {
            return Err(ValidationError::EmptyName);
        }

        if trimmed.len() > self.max_name_length {
            return Err(ValidationError::NameTooLong);
        }

        // Check for invalid characters
        for c in trimmed.chars() {
            if !self.allowed_name_chars.contains(&c) {
                return Err(ValidationError::InvalidNameFormat);
            }
        }

        Ok(())
    }

    /// Validate description
    pub fn validate_description(&self, description: &str) -> Result<(), ValidationError> {
        if description.len() > self.max_description_length {
            return Err(ValidationError::DescriptionTooLong);
        }
        Ok(())
    }

    /// Validate address
    pub fn validate_address(&self, address: &str) -> Result<(), ValidationError> {
        if address.len() > self.max_address_length {
            return Err(ValidationError::AddressTooLong);
        }
        Ok(())
    }
}

use crate::error::AppError;

impl From<ValidationError> for AppError {
    fn from(err: ValidationError) -> Self {
        AppError::ValidationError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_empty() {
        let validator = OrderValidator::new();
        let result = validator.validate_name("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ValidationError::EmptyName);
    }

    #[test]
    fn test_validate_name_whitespace_only() {
        let validator = OrderValidator::new();
        let result = validator.validate_name("   ");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ValidationError::EmptyName);
    }

    #[test]
    fn test_validate_name_too_long() {
        let validator = OrderValidator::new();
        let long_name = "a".repeat(101);
        let result = validator.validate_name(&long_name);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ValidationError::NameTooLong);
    }

    #[test]
    fn test_validate_name_invalid_characters() {
        let validator = OrderValidator::new();
        let result = validator.validate_name("Site@Name");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ValidationError::InvalidNameFormat);
    }

    #[test]
    fn test_validate_name_valid() {
        let validator = OrderValidator::new();
        let valid_names = vec![
            "Site-1",
            "Site_1",
            "Site.1",
            "Site (Main)",
            "Site-Name_123",
        ];

        for name in valid_names {
            assert!(validator.validate_name(name).is_ok(), "Failed for: {}", name);
        }
    }

    #[test]
    fn test_validate_description_too_long() {
        let validator = OrderValidator::new();
        let long_desc = "a".repeat(501);
        let result = validator.validate_description(&long_desc);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ValidationError::DescriptionTooLong);
    }

    #[test]
    fn test_validate_address_too_long() {
        let validator = OrderValidator::new();
        let long_addr = "a".repeat(201);
        let result = validator.validate_address(&long_addr);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ValidationError::AddressTooLong);
    }

    #[test]
    fn test_validate_site_order_success() {
        let validator = OrderValidator::new();
        let order = CreateSiteOrder {
            name: "Valid Site".to_string(),
            description: Some("Valid description".to_string()),
            address: Some("123 Main St".to_string()),
        };
        assert!(validator.validate_site_order(&order).is_ok());
    }

    #[test]
    fn test_validate_site_order_invalid_name() {
        let validator = OrderValidator::new();
        let order = CreateSiteOrder {
            name: "".to_string(),
            description: None,
            address: None,
        };
        assert!(validator.validate_site_order(&order).is_err());
    }

    #[test]
    fn test_validate_site_order_with_optional_fields() {
        let validator = OrderValidator::new();
        let order = CreateSiteOrder {
            name: "Minimal Site".to_string(),
            description: None,
            address: None,
        };
        assert!(validator.validate_site_order(&order).is_ok());
    }
}

