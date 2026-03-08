//! HTTP request parsing
//!
//! Implements HTTP/1.0 request parsing according to RFC 1945.
//! Supports GET method with query parameters.

use crate::error::{ServerError, ServerResult};
use std::collections::HashMap;
use std::str::FromStr;
use std::io::{BufRead, BufReader, Read};

/// HTTP method
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Head,
}

impl FromStr for HttpMethod {
    type Err = ServerError;
    /// Parse method from string
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(HttpMethod::Get),
            "HEAD" => Ok(HttpMethod::Head),
            method => Err(ServerError::MethodNotAllowed(method.to_string())),
        }
    }
}

impl HttpMethod {
    /// Convert to static string form
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Head => "HEAD",
        }
    }
}

/// HTTP request
#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// HTTP method
    pub method: HttpMethod,

    /// Request path (without query string)
    pub path: String,

    /// Query parameters
    pub query_params: HashMap<String, String>,

    /// HTTP version (e.g., "HTTP/1.0")
    pub version: String,

    /// Request headers
    pub headers: HashMap<String, String>,
}

impl HttpRequest {
    /// Parse HTTP request from a reader
    pub fn parse<R: Read>(reader: R) -> ServerResult<Self> {
        let mut buf_reader = BufReader::new(reader);

        // Parse request line
        let mut request_line = String::new();
        buf_reader
            .read_line(&mut request_line)
            .map_err(|e| ServerError::InvalidHttp(format!("Failed to read request line: {}", e)))?;

        let (method, path, query_params, version) = Self::parse_request_line(&request_line)?;

        // Parse headers
        let headers = Self::parse_headers(&mut buf_reader)?;

        Ok(HttpRequest {
            method,
            path,
            query_params,
            version,
            headers,
        })
    }

    /// Parse the request line (e.g., "GET /path?query HTTP/1.0")
    fn parse_request_line(
        line: &str,
    ) -> ServerResult<(HttpMethod, String, HashMap<String, String>, String)> {
        let line = line.trim();

        if line.is_empty() {
            return Err(ServerError::InvalidHttp("Empty request line".to_string()));
        }

        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() != 3 {
            return Err(ServerError::InvalidHttp(format!(
                "Invalid request line format: expected 3 parts, got {}",
                parts.len()
            )));
        }

        let method = parts[0].parse::<HttpMethod>()?;
        let (path, query_params) = Self::parse_uri(parts[1])?;
        let version = parts[2].to_string();

        // Validate HTTP version
        if !version.starts_with("HTTP/1.") {
            return Err(ServerError::InvalidHttp(format!(
                "Unsupported HTTP version: {}",
                version
            )));
        }

        Ok((method, path, query_params, version))
    }

    /// Parse URI into path and query parameters
    fn parse_uri(uri: &str) -> ServerResult<(String, HashMap<String, String>)> {
        if let Some(question_mark_pos) = uri.find('?') {
            let path = uri[..question_mark_pos].to_string();
            let query_string = &uri[question_mark_pos + 1..];
            let query_params = Self::parse_query_string(query_string)?;
            Ok((path, query_params))
        } else {
            Ok((uri.to_string(), HashMap::new()))
        }
    }

    /// Parse query string into key-value pairs
    fn parse_query_string(query: &str) -> ServerResult<HashMap<String, String>> {
        let mut params = HashMap::new();

        if query.is_empty() {
            return Ok(params);
        }

        for pair in query.split('&') {
            if let Some(eq_pos) = pair.find('=') {
                let key = Self::url_decode(&pair[..eq_pos])?;
                let value = Self::url_decode(&pair[eq_pos + 1..])?;
                params.insert(key, value);
            } else {
                // Parameter without value (e.g., "?flag")
                let key = Self::url_decode(pair)?;
                params.insert(key, String::new());
            }
        }

        Ok(params)
    }

    /// URL decode a string (basic implementation)
    fn url_decode(s: &str) -> ServerResult<String> {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '%' => {
                    // Get next two hex digits
                    let hex: String = chars.by_ref().take(2).collect();

                    if hex.len() != 2 {
                        return Err(ServerError::InvalidHttp(
                            "Invalid URL encoding: incomplete escape sequence".to_string(),
                        ));
                    }

                    let byte = u8::from_str_radix(&hex, 16).map_err(|_| {
                        ServerError::InvalidHttp(format!("Invalid hex in URL encoding: {}", hex))
                    })?;

                    result.push(byte as char);
                }
                '+' => result.push(' '),
                _ => result.push(ch),
            }
        }

        Ok(result)
    }

    /// Parse HTTP headers
    fn parse_headers<R: BufRead>(reader: &mut R) -> ServerResult<HashMap<String, String>> {
        let mut headers = HashMap::new();

        loop {
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| ServerError::InvalidHttp(format!("Failed to read header: {}", e)))?;

            let line = line.trim();

            // Empty line marks end of headers
            if line.is_empty() {
                break;
            }

            // Parse header line (e.g., "Host: example.com")
            if let Some(colon_pos) = line.find(':') {
                let name = line[..colon_pos].trim().to_lowercase();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.insert(name, value);
            } else {
                return Err(ServerError::InvalidHttp(format!(
                    "Invalid header format: {}",
                    line
                )));
            }
        }

        Ok(headers)
    }

    /// Get a query parameter
    pub fn query_param(&self, key: &str) -> Option<&str> {
        self.query_params.get(key).map(|s| s.as_str())
    }

    /// Get a required query parameter
    pub fn require_query_param(&self, key: &str) -> ServerResult<&str> {
        self.query_param(key)
            .ok_or_else(|| ServerError::missing_param(key))
    }

    /// Get a header value
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.get(&key.to_lowercase()).map(|s| s.as_str())
    }

    /// Check if header exists
    pub fn has_header(&self, key: &str) -> bool {
        self.headers.contains_key(&key.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_simple_get() {
        let request_data = b"GET /test HTTP/1.0\r\n\r\n";
        let request = HttpRequest::parse(Cursor::new(request_data)).unwrap();

        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.path, "/test");
        assert!(request.query_params.is_empty());
        assert_eq!(request.version, "HTTP/1.0");
    }

    #[test]
    fn test_parse_get_with_query() {
        let request_data = b"GET /api?name=value&count=5 HTTP/1.0\r\n\r\n";
        let request = HttpRequest::parse(Cursor::new(request_data)).unwrap();

        assert_eq!(request.path, "/api");
        assert_eq!(request.query_param("name"), Some("value"));
        assert_eq!(request.query_param("count"), Some("5"));
    }

    #[test]
    fn test_parse_with_headers() {
        let request_data = b"GET /test HTTP/1.0\r\nHost: localhost\r\nUser-Agent: test\r\n\r\n";
        let request = HttpRequest::parse(Cursor::new(request_data)).unwrap();

        assert_eq!(request.header("host"), Some("localhost"));
        assert_eq!(request.header("user-agent"), Some("test"));
        assert_eq!(request.header("Host"), Some("localhost")); // Case insensitive
    }

    #[test]
    fn test_url_decode() {
        assert_eq!(HttpRequest::url_decode("hello+world").unwrap(), "hello world");
        assert_eq!(HttpRequest::url_decode("hello%20world").unwrap(), "hello world");
        assert_eq!(HttpRequest::url_decode("a%2Bb%3Dc").unwrap(), "a+b=c");
    }

    #[test]
    fn test_invalid_method() {
        let request_data = b"POST /test HTTP/1.0\r\n\r\n";
        let result = HttpRequest::parse(Cursor::new(request_data));

        assert!(result.is_err());
        match result {
            Err(ServerError::MethodNotAllowed(_)) => (),
            _ => panic!("Expected MethodNotAllowed error"),
        }
    }

    #[test]
    fn test_invalid_request_line() {
        let request_data = b"INVALID\r\n\r\n";
        let result = HttpRequest::parse(Cursor::new(request_data));
        assert!(result.is_err());
    }

    #[test]
    fn test_require_query_param() {
        let request_data = b"GET /test?id=123 HTTP/1.0\r\n\r\n";
        let request = HttpRequest::parse(Cursor::new(request_data)).unwrap();

        assert_eq!(request.require_query_param("id").unwrap(), "123");
        assert!(request.require_query_param("missing").is_err());
    }

    #[test]
    fn test_empty_query_value() {
        let request_data = b"GET /test?flag HTTP/1.0\r\n\r\n";
        let request = HttpRequest::parse(Cursor::new(request_data)).unwrap();

        assert_eq!(request.query_param("flag"), Some(""));
    }

    #[test]
    fn test_multiple_headers() {
        let request_data = b"GET /test HTTP/1.0\r\n\
                             Host: example.com\r\n\
                             Accept: text/html\r\n\
                             Connection: close\r\n\
                             \r\n";
        let request = HttpRequest::parse(Cursor::new(request_data)).unwrap();

        assert_eq!(request.headers.len(), 3);
        assert!(request.has_header("host"));
        assert!(request.has_header("accept"));
        assert!(request.has_header("connection"));
    }
}
