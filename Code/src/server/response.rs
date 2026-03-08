//! HTTP response building
//!
//! Provides builders for creating HTTP/1.0 compliant responses.
//! Supports JSON responses with proper status codes and headers.

use crate::error::ServerError;
use std::collections::HashMap;

/// HTTP response builder
#[derive(Debug)]
pub struct HttpResponse {
    /// HTTP status code
    status_code: u16,

    /// Status message
    status_message: String,

    /// Response headers
    headers: HashMap<String, String>,

    /// Response body
    body: Vec<u8>,
}

impl HttpResponse {
    /// Create a new response with status code
    pub fn new(status_code: u16) -> Self {
        let status_message = Self::status_message(status_code);

        Self {
            status_code,
            status_message,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Create a 200 OK response
    pub fn ok() -> Self {
        Self::new(200)
    }

    /// Create a 400 Bad Request response
    pub fn bad_request() -> Self {
        Self::new(400)
    }

    /// Create a 404 Not Found response
    pub fn not_found() -> Self {
        Self::new(404)
    }

    /// Create a 500 Internal Server Error response
    pub fn internal_error() -> Self {
        Self::new(500)
    }

    /// Create a 503 Service Unavailable response
    pub fn service_unavailable() -> Self {
        Self::new(503)
    }

    /// Create response from error
    pub fn from_error(error: &ServerError) -> Self {
        let status_code = error.status_code();
        let body = format!(
            r#"{{"error":"{}","status":{}}}"#,
            error.error_message(),
            status_code
        );

        Self::new(status_code)
            .with_header("Content-Type", "application/json")
            .with_body(body.into_bytes())
    }

    /// Set a header
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Set the body
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    /// Set JSON body
    pub fn with_json(self, json: impl Into<String>) -> Self {
        let json_str = json.into();
        self.with_header("Content-Type", "application/json")
            .with_body(json_str.into_bytes())
    }

    /// Set plain text body
    pub fn with_text(self, text: impl Into<String>) -> Self {
        let text_str = text.into();
        self.with_header("Content-Type", "text/plain")
            .with_body(text_str.into_bytes())
    }

    /// Add request ID header
    pub fn with_request_id(self, request_id: impl Into<String>) -> Self {
        self.with_header("X-Request-Id", request_id)
    }

    /// Add worker PID header
    pub fn with_worker_pid(self, pid: u32) -> Self {
        self.with_header("X-Worker-Pid", pid.to_string())
    }

    /// Expose the status code for metrics/logging
    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    /// Convert this response to a HEAD-compatible response: preserve
    /// Content-Length for the original body, but clear the body bytes.
    pub fn into_head(mut self) -> Self {
        let len = self.body.len();
        self.headers
            .insert("Content-Length".to_string(), len.to_string());
        self.body.clear();
        self
    }

    /// Build the complete HTTP response as bytes
    pub fn build(mut self) -> Vec<u8> {
        let mut response = Vec::new();

        // Status line
        let status_line = format!(
            "HTTP/1.0 {} {}\r\n",
            self.status_code, self.status_message
        );
        response.extend_from_slice(status_line.as_bytes());

        // Add Content-Length if not present
        if !self.headers.contains_key("Content-Length") {
            self.headers
                .insert("Content-Length".to_string(), self.body.len().to_string());
        }

        // Add Connection: close for HTTP/1.0
        if !self.headers.contains_key("Connection") {
            self.headers
                .insert("Connection".to_string(), "close".to_string());
        }

        // Headers
        for (name, value) in &self.headers {
            let header_line = format!("{}: {}\r\n", name, value);
            response.extend_from_slice(header_line.as_bytes());
        }

        // Empty line between headers and body
        response.extend_from_slice(b"\r\n");

        // Body
        response.extend_from_slice(&self.body);

        response
    }

    /// Get standard status message for a status code
    fn status_message(code: u16) -> String {
        match code {
            200 => "OK",
            400 => "Bad Request",
            404 => "Not Found",
            405 => "Method Not Allowed",
            408 => "Request Timeout",
            409 => "Conflict",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            503 => "Service Unavailable",
            _ => "Unknown",
        }
            .to_string()
    }
}

/// Helper for building JSON responses
pub struct JsonResponseBuilder {
    status_code: u16,
    fields: Vec<(String, String)>,
}

impl JsonResponseBuilder {
    /// Create a new JSON response builder
    pub fn new(status_code: u16) -> Self {
        Self {
            status_code,
            fields: Vec::new(),
        }
    }

    /// Add a string field
    pub fn field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let value_str = value.into();
        let escaped = Self::escape_json_string(&value_str);
        self.fields
            .push((key.into(), format!(r#""{}""#, escaped)));
        self
    }

    /// Add a numeric field
    pub fn field_num(mut self, key: impl Into<String>, value: impl ToString) -> Self {
        self.fields.push((key.into(), value.to_string()));
        self
    }

    /// Add a boolean field
    pub fn field_bool(mut self, key: impl Into<String>, value: bool) -> Self {
        self.fields
            .push((key.into(), value.to_string()));
        self
    }

    /// Add a raw JSON field (already formatted)
    pub fn field_raw(mut self, key: impl Into<String>, json: impl Into<String>) -> Self {
        self.fields.push((key.into(), json.into()));
        self
    }

    /// Build the HTTP response
    pub fn build(self) -> HttpResponse {
        let status_code = self.status_code;
        let json = self.into_json_string();
        HttpResponse::new(status_code).with_json(json)
    }

    /// Build just the JSON string
    pub fn build_json(self) -> String {
        self.into_json_string()
    }

    /// Convert to JSON string (consumes self)
    fn into_json_string(self) -> String {
        if self.fields.is_empty() {
            return "{}".to_string();
        }

        let mut json = String::from("{");

        for (i, (key, value)) in self.fields.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push('"');
            json.push_str(&Self::escape_json_string(key));
            json.push_str("\":");
            json.push_str(value);
        }

        json.push('}');
        json
    }

    /// Escape a string for JSON
    fn escape_json_string(s: &str) -> String {
        let mut result = String::with_capacity(s.len());

        for ch in s.chars() {
            match ch {
                '"' => result.push_str(r#"\""#),
                '\\' => result.push_str(r"\\"),
                '\n' => result.push_str(r"\n"),
                '\r' => result.push_str(r"\r"),
                '\t' => result.push_str(r"\t"),
                _ if ch.is_control() => {
                    result.push_str(&format!(r"\u{:04x}", ch as u32));
                }
                _ => result.push(ch),
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_response() {
        let response = HttpResponse::ok()
            .with_text("Hello, World!")
            .build();

        let response_str = String::from_utf8(response).unwrap();

        assert!(response_str.contains("HTTP/1.0 200 OK"));
        assert!(response_str.contains("Content-Type: text/plain"));
        assert!(response_str.contains("Content-Length: 13"));
        assert!(response_str.contains("Hello, World!"));
    }

    #[test]
    fn test_json_response() {
        let json = JsonResponseBuilder::new(200)
            .field("message", "success")
            .field_num("count", 42)
            .field_bool("active", true)
            .build_json();

        assert!(json.contains(r#""message":"success""#));
        assert!(json.contains(r#""count":42"#));
        assert!(json.contains(r#""active":true"#));
    }

    #[test]
    fn test_error_response() {
        let error = ServerError::invalid_param("num", "must be positive");
        let response = HttpResponse::from_error(&error).build();

        let response_str = String::from_utf8(response).unwrap();

        assert!(response_str.contains("HTTP/1.0 400"));
        assert!(response_str.contains("application/json"));
        assert!(response_str.contains("error"));
    }
}
