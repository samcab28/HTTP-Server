//! Handler traits and utilities
//!
//! Common functionality for request handlers.

use crate::error::{ServerError, ServerResult};
use crate::server::requests::HttpRequest;
use std::str::FromStr;

/// Helper trait for parsing query parameters
pub trait QueryParamExt {
    /// Parse a required query parameter
    fn parse_param<T>(&self, key: &str) -> ServerResult<T>
    where
        T: FromStr,
        T::Err: std::fmt::Display;

    /// Parse an optional query parameter
    fn parse_param_optional<T>(&self, key: &str) -> ServerResult<Option<T>>
    where
        T: FromStr,
        T::Err: std::fmt::Display;

    /// Parse a parameter with a default value
    fn parse_param_or<T>(&self, key: &str, default: T) -> ServerResult<T>
    where
        T: FromStr,
        T::Err: std::fmt::Display;
}

impl QueryParamExt for HttpRequest {
    fn parse_param<T>(&self, key: &str) -> ServerResult<T>
    where
        T: FromStr,
        T::Err: std::fmt::Display,
    {
        let value = self.require_query_param(key)?;

        value.parse::<T>().map_err(|e| {
            ServerError::invalid_param(key, format!("invalid value: {}", e))
        })
    }

    fn parse_param_optional<T>(&self, key: &str) -> ServerResult<Option<T>>
    where
        T: FromStr,
        T::Err: std::fmt::Display,
    {
        match self.query_param(key) {
            Some(value) => {
                let parsed = value.parse::<T>().map_err(|e| {
                    ServerError::invalid_param(key, format!("invalid value: {}", e))
                })?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    fn parse_param_or<T>(&self, key: &str, default: T) -> ServerResult<T>
    where
        T: FromStr,
        T::Err: std::fmt::Display,
    {
        match self.parse_param_optional(key)? {
            Some(value) => Ok(value),
            None => Ok(default),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::requests::HttpMethod;
    use std::collections::HashMap;

    #[test]
    fn test_parse_param() {
        let mut params = HashMap::new();
        params.insert("num".to_string(), "42".to_string());

        let request = HttpRequest {
            method: HttpMethod::Get,
            path: "/test".to_string(),
            query_params: params,
            version: "HTTP/1.0".to_string(),
            headers: HashMap::new(),
        };

        let num: i32 = request.parse_param("num").unwrap();
        assert_eq!(num, 42);
    }
}