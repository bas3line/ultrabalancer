use crate::backend::{Server, ServerPool};
use crate::balancer::LoadBalancerSelector;
use crate::metrics::{MetricsCollector, MetricsExporter};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, warn};

pub struct RequestHandler {
    selector: LoadBalancerSelector,
    pool: ServerPool,
    metrics: Arc<MetricsCollector>,
}

impl RequestHandler {
    pub fn new(
        selector: LoadBalancerSelector,
        pool: ServerPool,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            selector,
            pool,
            metrics,
        }
    }

    pub async fn handle(
        &self,
        req: Request<Incoming>,
        client_addr: SocketAddr,
    ) -> Result<Response<Full<Bytes>>, hyper::Error> {
        let start = Instant::now();
        self.metrics.increment_total_requests();

        let path = req.uri().path();

        if path == "/health" {
            return Ok(self.health_response());
        }

        if path == "/metrics" {
            return Ok(self.metrics_response());
        }

        if path == "/prometheus" {
            return Ok(self.prometheus_response());
        }

        let servers = self.pool.get_healthy_servers();

        if servers.is_empty() {
            warn!("No healthy backends available");
            self.metrics.increment_failed_requests();
            return Ok(Self::error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "No healthy backends",
            ));
        }

        let client_ip = client_addr.ip().to_string();
        let server = match self.selector.select(&servers, Some(&client_ip)) {
            Ok(s) => s,
            Err(e) => {
                error!("Backend selection failed: {}", e);
                self.metrics.increment_failed_requests();
                return Ok(Self::error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Backend selection failed",
                ));
            }
        };

        server.increment_connections();
        let response = self.proxy_to_backend(req, &server).await;
        server.decrement_connections();

        let duration = start.elapsed();
        self.metrics.record_response_time(duration);

        response
    }

    async fn proxy_to_backend(
        &self,
        req: Request<Incoming>,
        server: &Server,
    ) -> Result<Response<Full<Bytes>>, hyper::Error> {
        let uri = format!("http://{}{}", server.address(), req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or("/"));
        let method = req.method().clone();

        debug!("Proxying {} to {}", method, uri);

        let client = reqwest::Client::new();
        let mut upstream_req = client.request(method.as_str().parse().unwrap(), &uri);

        for (name, value) in req.headers() {
            if let Ok(val) = value.to_str() {
                upstream_req = upstream_req.header(name.as_str(), val);
            }
        }

        let body = req.into_body().collect().await?.to_bytes();
        upstream_req = upstream_req.body(body.to_vec());

        match upstream_req.send().await {
            Ok(upstream_resp) => {
                self.metrics.increment_successful_requests();
                let status = upstream_resp.status();
                let body_bytes = upstream_resp.bytes().await.unwrap_or_default();

                Ok(Response::builder()
                    .status(status.as_u16())
                    .body(Full::new(body_bytes))
                    .unwrap())
            }
            Err(e) => {
                error!("Backend request failed for {}: {}", server.address(), e);
                self.metrics.increment_failed_requests();
                Ok(Self::error_response(
                    StatusCode::BAD_GATEWAY,
                    "Backend request failed",
                ))
            }
        }
    }

    fn health_response(&self) -> Response<Full<Bytes>> {
        let servers = self.pool.get_all_servers();
        let healthy = servers.iter().filter(|s| s.is_healthy()).count();
        let total = servers.len();

        let body = format!(
            r#"{{"status":"ok","healthy_backends":"{}/{}","uptime_seconds":{}}}"#,
            healthy,
            total,
            self.metrics.snapshot().uptime_seconds
        );

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body)))
            .unwrap()
    }

    fn metrics_response(&self) -> Response<Full<Bytes>> {
        let snapshot = self.metrics.snapshot();
        let body = MetricsExporter::export_json(&snapshot);

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body)))
            .unwrap()
    }

    fn prometheus_response(&self) -> Response<Full<Bytes>> {
        let snapshot = self.metrics.snapshot();
        let body = MetricsExporter::export_prometheus(&snapshot);

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/plain; version=0.0.4")
            .body(Full::new(Bytes::from(body)))
            .unwrap()
    }

    fn error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
        Response::builder()
            .status(status)
            .body(Full::new(Bytes::from(message.to_string())))
            .unwrap()
    }
}

impl Clone for RequestHandler {
    fn clone(&self) -> Self {
        Self {
            selector: self.selector.clone(),
            pool: self.pool.clone(),
            metrics: Arc::clone(&self.metrics),
        }
    }
}
