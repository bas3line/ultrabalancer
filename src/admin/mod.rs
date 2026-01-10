use bytes::Bytes;
use http_body_util::Full;
use hyper::{Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendInfo {
    pub address: String,
    pub weight: u32,
    pub healthy: bool,
    pub active_connections: usize,
    pub total_requests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBackendRequest {
    pub address: String,
    pub weight: Option<u32>,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveBackendRequest {
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWeightRequest {
    pub address: String,
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl AdminResponse {
    pub fn success(message: &str) -> Self {
        Self {
            success: true,
            message: message.to_string(),
            data: None,
        }
    }

    pub fn success_with_data(message: &str, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: message.to_string(),
            data: Some(data),
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            message: message.to_string(),
            data: None,
        }
    }
}

pub trait BackendManager: Send + Sync {
    fn list_backends(&self) -> Vec<BackendInfo>;
    fn add_backend(&self, address: &str, weight: u32) -> bool;
    fn remove_backend(&self, address: &str) -> bool;
    fn update_weight(&self, address: &str, weight: u32) -> bool;
    fn get_backend(&self, address: &str) -> Option<BackendInfo>;
    fn drain_backend(&self, address: &str) -> bool;
    fn undrain_backend(&self, address: &str) -> bool;
}

pub struct AdminApi<M: BackendManager> {
    manager: Arc<M>,
    api_key: Option<String>,
}

impl<M: BackendManager> AdminApi<M> {
    pub fn new(manager: Arc<M>, api_key: Option<String>) -> Self {
        Self { manager, api_key }
    }

    pub async fn handle(&self, req: Request<hyper::body::Incoming>) -> Response<Full<Bytes>> {
        if let Some(ref key) = self.api_key {
            let auth = req
                .headers()
                .get("X-API-Key")
                .and_then(|v| v.to_str().ok());

            if auth != Some(key) {
                return self.json_response(StatusCode::UNAUTHORIZED, &AdminResponse::error("Invalid API key"));
            }
        }

        let path = req.uri().path();
        let method = req.method().as_str();

        match (method, path) {
            ("GET", "/admin/backends") => self.list_backends(),
            ("POST", "/admin/backends") => self.add_backend(req).await,
            ("DELETE", "/admin/backends") => self.remove_backend(req).await,
            ("PUT", "/admin/backends/weight") => self.update_weight(req).await,
            ("POST", "/admin/backends/drain") => self.drain_backend(req).await,
            ("POST", "/admin/backends/undrain") => self.undrain_backend(req).await,
            ("GET", "/admin/health") => self.health_check(),
            ("POST", "/admin/reload") => self.reload_config(),
            _ => self.json_response(StatusCode::NOT_FOUND, &AdminResponse::error("Endpoint not found")),
        }
    }

    fn list_backends(&self) -> Response<Full<Bytes>> {
        let backends = self.manager.list_backends();
        let data = serde_json::to_value(&backends).unwrap_or_default();
        self.json_response(
            StatusCode::OK,
            &AdminResponse::success_with_data("Backends listed", data),
        )
    }

    async fn add_backend(&self, req: Request<hyper::body::Incoming>) -> Response<Full<Bytes>> {
        let body = match self.read_body(req).await {
            Ok(b) => b,
            Err(e) => return self.json_response(StatusCode::BAD_REQUEST, &AdminResponse::error(&e)),
        };

        let request: AddBackendRequest = match serde_json::from_slice(&body) {
            Ok(r) => r,
            Err(_) => return self.json_response(StatusCode::BAD_REQUEST, &AdminResponse::error("Invalid JSON")),
        };

        let weight = request.weight.unwrap_or(100);
        if self.manager.add_backend(&request.address, weight) {
            self.json_response(StatusCode::OK, &AdminResponse::success("Backend added"))
        } else {
            self.json_response(StatusCode::CONFLICT, &AdminResponse::error("Backend already exists"))
        }
    }

    async fn remove_backend(&self, req: Request<hyper::body::Incoming>) -> Response<Full<Bytes>> {
        let body = match self.read_body(req).await {
            Ok(b) => b,
            Err(e) => return self.json_response(StatusCode::BAD_REQUEST, &AdminResponse::error(&e)),
        };

        let request: RemoveBackendRequest = match serde_json::from_slice(&body) {
            Ok(r) => r,
            Err(_) => return self.json_response(StatusCode::BAD_REQUEST, &AdminResponse::error("Invalid JSON")),
        };

        if self.manager.remove_backend(&request.address) {
            self.json_response(StatusCode::OK, &AdminResponse::success("Backend removed"))
        } else {
            self.json_response(StatusCode::NOT_FOUND, &AdminResponse::error("Backend not found"))
        }
    }

    async fn update_weight(&self, req: Request<hyper::body::Incoming>) -> Response<Full<Bytes>> {
        let body = match self.read_body(req).await {
            Ok(b) => b,
            Err(e) => return self.json_response(StatusCode::BAD_REQUEST, &AdminResponse::error(&e)),
        };

        let request: UpdateWeightRequest = match serde_json::from_slice(&body) {
            Ok(r) => r,
            Err(_) => return self.json_response(StatusCode::BAD_REQUEST, &AdminResponse::error("Invalid JSON")),
        };

        if self.manager.update_weight(&request.address, request.weight) {
            self.json_response(StatusCode::OK, &AdminResponse::success("Weight updated"))
        } else {
            self.json_response(StatusCode::NOT_FOUND, &AdminResponse::error("Backend not found"))
        }
    }

    async fn drain_backend(&self, req: Request<hyper::body::Incoming>) -> Response<Full<Bytes>> {
        let body = match self.read_body(req).await {
            Ok(b) => b,
            Err(e) => return self.json_response(StatusCode::BAD_REQUEST, &AdminResponse::error(&e)),
        };

        let request: RemoveBackendRequest = match serde_json::from_slice(&body) {
            Ok(r) => r,
            Err(_) => return self.json_response(StatusCode::BAD_REQUEST, &AdminResponse::error("Invalid JSON")),
        };

        if self.manager.drain_backend(&request.address) {
            self.json_response(StatusCode::OK, &AdminResponse::success("Backend draining"))
        } else {
            self.json_response(StatusCode::NOT_FOUND, &AdminResponse::error("Backend not found"))
        }
    }

    async fn undrain_backend(&self, req: Request<hyper::body::Incoming>) -> Response<Full<Bytes>> {
        let body = match self.read_body(req).await {
            Ok(b) => b,
            Err(e) => return self.json_response(StatusCode::BAD_REQUEST, &AdminResponse::error(&e)),
        };

        let request: RemoveBackendRequest = match serde_json::from_slice(&body) {
            Ok(r) => r,
            Err(_) => return self.json_response(StatusCode::BAD_REQUEST, &AdminResponse::error("Invalid JSON")),
        };

        if self.manager.undrain_backend(&request.address) {
            self.json_response(StatusCode::OK, &AdminResponse::success("Backend undraining"))
        } else {
            self.json_response(StatusCode::NOT_FOUND, &AdminResponse::error("Backend not found"))
        }
    }

    fn health_check(&self) -> Response<Full<Bytes>> {
        self.json_response(StatusCode::OK, &AdminResponse::success("Admin API healthy"))
    }

    fn reload_config(&self) -> Response<Full<Bytes>> {
        self.json_response(StatusCode::OK, &AdminResponse::success("Config reload triggered"))
    }

    async fn read_body(&self, req: Request<hyper::body::Incoming>) -> Result<Bytes, String> {
        use http_body_util::BodyExt;
        req.into_body()
            .collect()
            .await
            .map(|c| c.to_bytes())
            .map_err(|e| e.to_string())
    }

    fn json_response(&self, status: StatusCode, response: &AdminResponse) -> Response<Full<Bytes>> {
        let body = serde_json::to_string(response).unwrap_or_default();
        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body)))
            .unwrap()
    }
}

impl<M: BackendManager> Clone for AdminApi<M> {
    fn clone(&self) -> Self {
        Self {
            manager: Arc::clone(&self.manager),
            api_key: self.api_key.clone(),
        }
    }
}
