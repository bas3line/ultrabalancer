use crate::backend::{Server, ServerPool};
use crate::balancer::LoadBalancerSelector;
use crate::cache::{CacheControl, CachedResponse, ResponseCache};
use crate::metrics::{MetricsCollector, MetricsExporter};
use crate::middleware::{AccessLogEntry, AccessLogger, CompressionAlgo, CompressionMiddleware, IpFilter, RetryMiddleware, RetryState};
use crate::routing::Router;
use crate::utils::{ConnectionPool, GracefulShutdown, RateLimiter, RequestId, StickySessionManager};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::{Body, Incoming};
use hyper::header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE};
use hyper::{Request, Response, StatusCode};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, warn};

pub struct RequestHandler {
    selector: LoadBalancerSelector,
    pool: ServerPool,
    metrics: Arc<MetricsCollector>,
    http_client: reqwest::Client,
    rate_limiter: Option<RateLimiter>,
    ip_filter: Option<IpFilter>,
    cache: Option<ResponseCache>,
    compression: Option<CompressionMiddleware>,
    retry: Option<RetryMiddleware>,
    sticky_sessions: Option<StickySessionManager>,
    access_logger: Option<AccessLogger>,
    router: Option<Router>,
    connection_pool: Option<ConnectionPool>,
    shutdown: Option<Arc<GracefulShutdown>>,
    request_timeout: Duration,
}

impl RequestHandler {
    pub fn new(
        selector: LoadBalancerSelector,
        pool: ServerPool,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .pool_max_idle_per_host(500)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(60))
            .timeout(Duration::from_secs(30))
            .tcp_nodelay(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            selector,
            pool,
            metrics,
            http_client,
            rate_limiter: None,
            ip_filter: None,
            cache: None,
            compression: None,
            retry: None,
            sticky_sessions: None,
            access_logger: None,
            router: None,
            connection_pool: None,
            shutdown: None,
            request_timeout: Duration::from_secs(30),
        }
    }

    pub fn with_rate_limiter(mut self, limiter: RateLimiter) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }

    pub fn with_ip_filter(mut self, filter: IpFilter) -> Self {
        self.ip_filter = Some(filter);
        self
    }

    pub fn with_cache(mut self, cache: ResponseCache) -> Self {
        self.cache = Some(cache);
        self
    }

    pub fn with_compression(mut self, compression: CompressionMiddleware) -> Self {
        self.compression = Some(compression);
        self
    }

    pub fn with_retry(mut self, retry: RetryMiddleware) -> Self {
        self.retry = Some(retry);
        self
    }

    pub fn with_sticky_sessions(mut self, manager: StickySessionManager) -> Self {
        self.sticky_sessions = Some(manager);
        self
    }

    pub fn with_access_logger(mut self, logger: AccessLogger) -> Self {
        self.access_logger = Some(logger);
        self
    }

    pub fn with_router(mut self, router: Router) -> Self {
        self.router = Some(router);
        self
    }

    pub fn with_connection_pool(mut self, pool: ConnectionPool) -> Self {
        self.connection_pool = Some(pool);
        self
    }

    pub fn with_shutdown(mut self, shutdown: Arc<GracefulShutdown>) -> Self {
        self.shutdown = Some(shutdown);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    pub async fn handle(
        &self,
        req: Request<Incoming>,
        client_addr: SocketAddr,
    ) -> Result<Response<Full<Bytes>>, hyper::Error> {
        let start = Instant::now();
        let request_id = RequestId::extract_from_headers(req.headers())
            .unwrap_or_else(RequestId::generate_short);

        if let Some(ref shutdown) = self.shutdown {
            if shutdown.is_shutting_down() {
                return Ok(Self::error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Server shutting down",
                ));
            }
            shutdown.connection_started();
        }

        let mut log_entry = if self.access_logger.is_some() {
            Some(AccessLogEntry::new(
                client_addr,
                req.method().as_str(),
                req.uri().path(),
                request_id.clone(),
            ))
        } else {
            None
        };

        self.metrics.increment_total_requests();
        let client_ip = client_addr.ip().to_string();

        if let Some(ref filter) = self.ip_filter {
            if !filter.is_allowed(client_addr.ip()) {
                self.metrics.increment_failed_requests();
                return Ok(Self::error_response(StatusCode::FORBIDDEN, "Forbidden"));
            }
        }

        if let Some(ref limiter) = self.rate_limiter {
            if !limiter.check_ip(&client_ip) {
                warn!("Rate limit exceeded for {}", client_ip);
                self.metrics.increment_failed_requests();
                return Ok(Self::error_response(
                    StatusCode::TOO_MANY_REQUESTS,
                    "Rate limit exceeded",
                ));
            }
        }

        let path = req.uri().path();
        let method = req.method().as_str();

        match path {
            "/health" => return Ok(self.health_response()),
            "/metrics" => return Ok(self.metrics_response()),
            "/prometheus" => return Ok(self.prometheus_response()),
            "/ready" => return Ok(self.ready_response()),
            _ => {}
        }

        let accept_encoding = req
            .headers()
            .get(ACCEPT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let compression_algo = if self.compression.is_some() {
            CompressionAlgo::from_accept_encoding(accept_encoding)
        } else {
            CompressionAlgo::None
        };

        if let Some(ref cache) = self.cache {
            if method == "GET" || method == "HEAD" {
                let cache_key = ResponseCache::cache_key(method, path, None);
                if let Some(cached) = cache.get(cache_key).await {
                    self.metrics.increment_successful_requests();
                    let duration = start.elapsed();
                    self.metrics.record_response_time(duration);

                    if let Some(entry) = log_entry {
                        let entry = entry.complete(cached.status, cached.body.len() as u64, duration, Some("CACHE".to_string()));
                        if let Some(ref logger) = self.access_logger {
                            logger.log(entry);
                        }
                    }

                    return Ok(self.build_cached_response(&cached, compression_algo).await);
                }
            }
        }

        let response = self.proxy_request(req, &client_ip, &request_id, compression_algo).await;

        let duration = start.elapsed();
        self.metrics.record_response_time(duration);

        if let Some(ref shutdown) = self.shutdown {
            shutdown.connection_ended();
        }

        if let Some(entry) = log_entry {
            let status = response.as_ref().map(|r| r.status().as_u16()).unwrap_or(500);
            let bytes = response.as_ref().map(|r| r.body().size_hint().exact().unwrap_or(0)).unwrap_or(0);
            let entry = entry.complete(status, bytes, duration, None);
            if let Some(ref logger) = self.access_logger {
                logger.log(entry);
            }
        }

        response
    }

    async fn proxy_request(
        &self,
        req: Request<Incoming>,
        client_ip: &str,
        request_id: &str,
        compression_algo: CompressionAlgo,
    ) -> Result<Response<Full<Bytes>>, hyper::Error> {
        let method = req.method().clone();
        let uri = req.uri().clone();
        let headers = req.headers().clone();
        let path = uri.path();

        let sticky_backend = if let Some(ref sticky) = self.sticky_sessions {
            if let Some(cookie) = headers.get("cookie").and_then(|v| v.to_str().ok()) {
                if let Some(session_id) = sticky.extract_session_from_cookie(cookie) {
                    sticky.get_backend(&session_id)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let servers = self.pool.get_healthy_servers();
        if servers.is_empty() {
            warn!("No healthy backends available");
            self.metrics.increment_failed_requests();
            return Ok(Self::error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "No healthy backends",
            ));
        }

        let server = if let Some(ref backend_addr) = sticky_backend {
            servers.iter().find(|s| &s.address() == backend_addr).cloned()
        } else {
            None
        };

        let server = match server {
            Some(s) => s,
            None => match self.selector.select(&servers, Some(client_ip)) {
                Ok(s) => s,
                Err(e) => {
                    error!("Backend selection failed: {}", e);
                    self.metrics.increment_failed_requests();
                    return Ok(Self::error_response(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "Backend selection failed",
                    ));
                }
            },
        };

        server.increment_connections();
        let backend_addr = server.address();

        let body = req.into_body().collect().await?.to_bytes();
        let target_uri = format!(
            "http://{}{}",
            backend_addr,
            uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/")
        );

        let mut retry_state = RetryState::new();
        let max_attempts = self.retry.as_ref().map(|r| r.max_attempts()).unwrap_or(1);

        let response = loop {
            let mut upstream_req = self.http_client.request(
                method.as_str().parse().unwrap(),
                &target_uri,
            );

            for (name, value) in headers.iter() {
                if let Ok(val) = value.to_str() {
                    if !matches!(name.as_str(), "host" | "connection" | "keep-alive" | "transfer-encoding") {
                        upstream_req = upstream_req.header(name.as_str(), val);
                    }
                }
            }

            upstream_req = upstream_req
                .header("X-Forwarded-For", client_ip)
                .header("X-Real-IP", client_ip)
                .header("X-Request-ID", request_id)
                .header("Host", backend_addr.split(':').next().unwrap_or(&backend_addr))
                .body(body.to_vec());

            match upstream_req.send().await {
                Ok(resp) => {
                    let status = resp.status().as_u16();

                    if let Some(ref retry) = self.retry {
                        if retry.should_retry(retry_state.attempt, Some(status), false) && retry_state.attempt < max_attempts {
                            retry_state.increment(None, Some(status));
                            retry.wait(retry_state.attempt).await;
                            continue;
                        }
                    }

                    self.metrics.increment_successful_requests();
                    let resp_headers = resp.headers().clone();
                    let body_bytes = resp.bytes().await.unwrap_or_default();

                    if let Some(ref cache) = self.cache {
                        if ResponseCache::should_cache(status, method.as_str()) {
                            if let Some(cc_header) = resp_headers.get("cache-control").and_then(|v| v.to_str().ok()) {
                                let cc = ResponseCache::parse_cache_control(cc_header);
                                if !cc.no_store && !cc.private {
                                    let ttl = cc.s_maxage.or(cc.max_age).unwrap_or(300);
                                    let cached = CachedResponse {
                                        status,
                                        headers: resp_headers.iter()
                                            .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.as_str().to_string(), v.to_string())))
                                            .collect(),
                                        body: body_bytes.clone(),
                                        created_at: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs(),
                                    };
                                    let cache_key = ResponseCache::cache_key(method.as_str(), path, None);
                                    cache.set_with_ttl(cache_key, cached, Duration::from_secs(ttl)).await;
                                }
                            }
                        }
                    }

                    let mut response_body = body_bytes;
                    let mut encoding_header = None;

                    if let Some(ref compressor) = self.compression {
                        if let Some(content_type) = resp_headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
                            if CompressionMiddleware::should_compress(Some(content_type)) {
                                if let Ok(compressed) = compressor.compress(response_body.clone(), compression_algo).await {
                                    if compressed.len() < response_body.len() {
                                        response_body = compressed;
                                        encoding_header = compression_algo.content_encoding();
                                    }
                                }
                            }
                        }
                    }

                    let mut builder = Response::builder().status(status);

                    for (name, value) in resp_headers.iter() {
                        if name.as_str() != "transfer-encoding" && name.as_str() != "content-length" {
                            if let Ok(val) = value.to_str() {
                                builder = builder.header(name.as_str(), val);
                            }
                        }
                    }

                    if let Some(encoding) = encoding_header {
                        builder = builder.header(CONTENT_ENCODING, encoding);
                    }

                    builder = builder.header("X-Request-ID", request_id);

                    if let Some(ref sticky) = self.sticky_sessions {
                        if sticky_backend.is_none() {
                            let session_id = sticky.generate_session_id();
                            sticky.set_backend(&session_id, &backend_addr);
                            builder = builder.header("Set-Cookie", sticky.create_cookie(&session_id));
                        }
                    }

                    break Ok(builder.body(Full::new(response_body)).unwrap());
                }
                Err(e) => {
                    if let Some(ref retry) = self.retry {
                        if retry.should_retry(retry_state.attempt, None, true) && retry_state.attempt < max_attempts {
                            retry_state.increment(Some(e.to_string()), None);
                            retry.wait(retry_state.attempt).await;
                            continue;
                        }
                    }

                    error!("Backend request failed for {}: {}", backend_addr, e);
                    self.metrics.increment_failed_requests();
                    break Ok(Self::error_response(StatusCode::BAD_GATEWAY, "Backend request failed"));
                }
            }
        };

        server.decrement_connections();
        response
    }

    async fn build_cached_response(
        &self,
        cached: &CachedResponse,
        compression_algo: CompressionAlgo,
    ) -> Response<Full<Bytes>> {
        let mut response_body = cached.body.clone();
        let mut encoding_header = None;

        if let Some(ref compressor) = self.compression {
            let content_type = cached.headers.iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                .map(|(_, v)| v.as_str());

            if CompressionMiddleware::should_compress(content_type) {
                if let Ok(compressed) = compressor.compress(response_body.clone(), compression_algo).await {
                    if compressed.len() < response_body.len() {
                        response_body = compressed;
                        encoding_header = compression_algo.content_encoding();
                    }
                }
            }
        }

        let mut builder = Response::builder().status(cached.status);

        for (name, value) in &cached.headers {
            if name != "transfer-encoding" && name != "content-length" {
                builder = builder.header(name.as_str(), value.as_str());
            }
        }

        if let Some(encoding) = encoding_header {
            builder = builder.header(CONTENT_ENCODING, encoding);
        }

        builder = builder.header("X-Cache", "HIT");
        builder.body(Full::new(response_body)).unwrap()
    }

    fn health_response(&self) -> Response<Full<Bytes>> {
        let servers = self.pool.get_all_servers();
        let healthy = servers.iter().filter(|s| s.is_healthy()).count();
        let total = servers.len();
        let snapshot = self.metrics.snapshot();

        let body = serde_json::json!({
            "status": "ok",
            "healthy_backends": format!("{}/{}", healthy, total),
            "uptime_seconds": snapshot.uptime_seconds,
            "requests_per_second": snapshot.requests_per_second
        });

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap()
    }

    fn ready_response(&self) -> Response<Full<Bytes>> {
        let healthy = self.pool.get_healthy_servers().len();
        if healthy > 0 {
            Response::builder()
                .status(StatusCode::OK)
                .body(Full::new(Bytes::from("ready")))
                .unwrap()
        } else {
            Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(Full::new(Bytes::from("not ready")))
                .unwrap()
        }
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
        let body = serde_json::json!({"error": message});
        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap()
    }
}

impl Clone for RequestHandler {
    fn clone(&self) -> Self {
        Self {
            selector: self.selector.clone(),
            pool: self.pool.clone(),
            metrics: Arc::clone(&self.metrics),
            http_client: self.http_client.clone(),
            rate_limiter: self.rate_limiter.clone(),
            ip_filter: self.ip_filter.clone(),
            cache: self.cache.clone(),
            compression: self.compression.clone(),
            retry: self.retry.clone(),
            sticky_sessions: self.sticky_sessions.clone(),
            access_logger: self.access_logger.clone(),
            router: self.router.clone(),
            connection_pool: self.connection_pool.clone(),
            shutdown: self.shutdown.clone(),
            request_timeout: self.request_timeout,
        }
    }
}
