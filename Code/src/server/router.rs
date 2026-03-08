//! HTTP request routing
//!
//! Routes incoming requests to appropriate handlers based on path matching.

use crate::error::{ServerError, ServerResult};
use crate::server::requests::HttpRequest;
use crate::server::response::HttpResponse;
use std::collections::HashMap;

/// Handler function type
pub type HandlerFn = fn(&HttpRequest) -> ServerResult<HttpResponse>;

/// Router for HTTP requests
#[derive(Clone)]
pub struct Router {
    routes: HashMap<String, HandlerFn>,
}

impl Router {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    /// Register a route with a handler
    pub fn route(mut self, path: impl Into<String>, handler: HandlerFn) -> Self {
        self.routes.insert(path.into(), handler);
        self
    }

    /// Add a route (mutable version)
    pub fn add_route(&mut self, path: impl Into<String>, handler: HandlerFn) {
        self.routes.insert(path.into(), handler);
    }

    /// Route a request to the appropriate handler
    pub fn handle(&self, request: &HttpRequest) -> ServerResult<HttpResponse> {
        // Look up exact path match
        if let Some(handler) = self.routes.get(&request.path) {
            return handler(request);
        }

        // No match found
        Err(ServerError::not_found(&request.path))
    }

    /// Get all registered routes
    pub fn routes(&self) -> Vec<&str> {
        self.routes.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a route exists
    pub fn has_route(&self, path: &str) -> bool {
        self.routes.contains_key(path)
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::requests::HttpMethod;

    fn dummy_handler(_req: &HttpRequest) -> ServerResult<HttpResponse> {
        Ok(HttpResponse::ok().with_text("dummy"))
    }

    #[test]
    fn test_route_registration() {
        let router = Router::new()
            .route("/test", dummy_handler)
            .route("/api", dummy_handler);

        assert!(router.has_route("/test"));
        assert!(router.has_route("/api"));
        assert!(!router.has_route("/missing"));
    }

    #[test]
    fn test_route_not_found() {
        let router = Router::new();
        let request = HttpRequest {
            method: HttpMethod::Get,
            path: "/missing".to_string(),
            query_params: HashMap::new(),
            version: "HTTP/1.0".to_string(),
            headers: HashMap::new(),
        };

        let result = router.handle(&request);
        assert!(matches!(result, Err(ServerError::NotFound(_))));
    }
}
