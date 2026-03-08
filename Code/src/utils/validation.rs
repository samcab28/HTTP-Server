//! Input validation utilities

use crate::error::{ServerError, ServerResult};

/// Validate that a number is within a range
pub fn validate_range<T: PartialOrd + std::fmt::Display>(
    value: T,
    min: T,
    max: T,
    param_name: &str,
) -> ServerResult<T> {
    if value < min || value > max {
        return Err(ServerError::invalid_param(
            param_name,
            format!("must be between {} and {}", min, max),
        ));
    }
    Ok(value)
}

/// Validate that a string is not empty
pub fn validate_not_empty(value: &str, param_name: &str) -> ServerResult<()> {
    if value.is_empty() {
        return Err(ServerError::invalid_param(param_name, "cannot be empty"));
    }
    Ok(())
}

/// Validate filename (no path traversal)
pub fn validate_filename(filename: &str) -> ServerResult<()> {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(ServerError::invalid_param("filename", "invalid characters"));
    }

    if filename.is_empty() {
        return Err(ServerError::invalid_param("filename", "cannot be empty"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_range() {
        assert!(validate_range(5, 0, 10, "test").is_ok());
        assert!(validate_range(15, 0, 10, "test").is_err());
    }
}