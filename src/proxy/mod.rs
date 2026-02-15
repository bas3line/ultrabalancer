pub mod handler;
pub mod websocket;

use crate::backend::ServerPool;
use crate::balancer::LoadBalancerSelector;
use crate::cache::ResponseCache;
use crate::metrics::MetricsCollector;
use crate::middleware::{
    AccessLogger, CompressionMiddleware, IpFilter, LogFormat, RetryConfig, RetryMiddleware,
};
use crate::utils::{GracefulShutdown, RateLimiter, StickySessionManager};
use handler::RequestHandler;
use hyper::server::conn::{http1, http2};
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, warn};

/// HTTP version mode for the proxy server.
#[derive(Clone)]
pub enum HttpVersion {
    /// HTTP/1.1 only
    Http1Only,
    /// HTTP/2 only (h2c - cleartext HTTP/2)
    Http2Only,
    /// HTTP/1.1 with upgrade support (WebSocket).
    /// Note: This does NOT negotiate HTTP/2 via ALPN; for HTTP/2 use Http2Only or TLS with ALPN.
    Auto,
}

pub struct ProxyServerConfig {
    pub listen_addr: String,
    pub http_version: HttpVersion,
    pub rate_limit_rps: Option<u32>,
    pub rate_limit_burst: Option<u32>,
    pub per_ip_rate_limit: Option<(u32, u32)>,
    pub ip_whitelist: Option<Vec<String>>,
    pub ip_blacklist: Option<Vec<String>>,
    pub cache_max_entries: Option<u64>,
    pub cache_ttl_secs: Option<u64>,
    pub compression_enabled: bool,
    pub compression_min_size: usize,
    pub retry_enabled: bool,
    pub retry_max_attempts: u32,
    pub sticky_sessions_enabled: bool,
    pub sticky_cookie_name: String,
    pub sticky_ttl_secs: u64,
    pub access_log_path: Option<String>,
    pub access_log_format: LogFormat,
    pub request_timeout_secs: u64,
    pub graceful_shutdown_timeout_secs: u64,
}

impl Default for ProxyServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:8080".to_string(),
            http_version: HttpVersion::Auto,
            rate_limit_rps: None,
            rate_limit_burst: None,
            per_ip_rate_limit: None,
            ip_whitelist: None,
            ip_blacklist: None,
            cache_max_entries: None,
            cache_ttl_secs: None,
            compression_enabled: false,
            compression_min_size: 1024,
            retry_enabled: false,
            retry_max_attempts: 3,
            sticky_sessions_enabled: false,
            sticky_cookie_name: "SERVERID".to_string(),
            sticky_ttl_secs: 3600,
            access_log_path: None,
            access_log_format: LogFormat::Combined,
            request_timeout_secs: 30,
            graceful_shutdown_timeout_secs: 30,
        }
    }
}

pub struct ProxyServer {
    handler: RequestHandler,
    listen_addr: String,
    http_version: HttpVersion,
    shutdown: Option<Arc<GracefulShutdown>>,
}

impl ProxyServer {
    pub fn new(
        selector: LoadBalancerSelector,
        pool: ServerPool,
        metrics: Arc<MetricsCollector>,
        listen_addr: String,
    ) -> Self {
        Self {
            handler: RequestHandler::new(selector, pool, metrics),
            listen_addr,
            http_version: HttpVersion::Auto,
            shutdown: None,
        }
    }

    pub fn with_config(
        selector: LoadBalancerSelector,
        pool: ServerPool,
        metrics: Arc<MetricsCollector>,
        config: ProxyServerConfig,
    ) -> Self {
        let mut handler = RequestHandler::new(selector, pool, metrics)
            .with_timeout(Duration::from_secs(config.request_timeout_secs));

        if let Some(rps) = config.rate_limit_rps {
            let burst = config.rate_limit_burst.unwrap_or(rps * 2);
            let mut limiter = RateLimiter::with_burst(rps, burst);
            if let Some((ip_rps, ip_burst)) = config.per_ip_rate_limit {
                limiter = limiter.with_per_ip_limit(ip_rps, ip_burst);
            }
            handler = handler.with_rate_limiter(limiter);
        }

        if config.ip_whitelist.is_some() || config.ip_blacklist.is_some() {
            let whitelist = config.ip_whitelist.unwrap_or_default();
            let blacklist = config.ip_blacklist.unwrap_or_default();
            let filter = IpFilter::new()
                .with_whitelist(whitelist)
                .with_blacklist(blacklist);
            handler = handler.with_ip_filter(filter);
        }

        if let Some(max_entries) = config.cache_max_entries {
            let ttl = config.cache_ttl_secs.unwrap_or(300);
            let cache = ResponseCache::new(max_entries, ttl);
            handler = handler.with_cache(cache);
        }

        if config.compression_enabled {
            let compression = CompressionMiddleware::new(config.compression_min_size, 4);
            handler = handler.with_compression(compression);
        }

        if config.retry_enabled {
            let retry_config = RetryConfig {
                max_attempts: config.retry_max_attempts,
                ..Default::default()
            };
            let retry = RetryMiddleware::new(retry_config);
            handler = handler.with_retry(retry);
        }

        if config.sticky_sessions_enabled {
            let sticky =
                StickySessionManager::new(&config.sticky_cookie_name, config.sticky_ttl_secs);
            handler = handler.with_sticky_sessions(sticky);
        }

        let logger = AccessLogger::new(config.access_log_path, config.access_log_format);
        handler = handler.with_access_logger(logger);

        let (shutdown, _rx) = GracefulShutdown::new(config.graceful_shutdown_timeout_secs);
        handler = handler.with_shutdown(Arc::clone(&shutdown));

        Self {
            handler,
            listen_addr: config.listen_addr,
            http_version: config.http_version,
            shutdown: Some(shutdown),
        }
    }

    pub async fn start(self: Arc<Self>) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&self.listen_addr).await?;
        info!("Load balancer listening on {}", self.listen_addr);

        let http_ver = match &self.http_version {
            HttpVersion::Http1Only => "HTTP/1.1",
            HttpVersion::Http2Only => "HTTP/2 (h2c)",
            HttpVersion::Auto => "HTTP/1.1 with WebSocket upgrades",
        };
        info!("Protocol: {}", http_ver);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    if let Some(ref shutdown) = self.shutdown {
                        if shutdown.is_shutting_down() {
                            warn!("Rejecting new connection during shutdown");
                            continue;
                        }
                    }

                    let server = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_connection(stream, addr).await {
                            error!("Connection error from {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn handle_connection(
        &self,
        stream: TcpStream,
        addr: std::net::SocketAddr,
    ) -> anyhow::Result<()> {
        stream.set_nodelay(true)?;
        let io = TokioIo::new(stream);
        let handler = self.handler.clone();

        let service = service_fn(move |req| {
            let handler = handler.clone();
            async move { handler.handle(req, addr).await }
        });

        match &self.http_version {
            HttpVersion::Http1Only => {
                http1::Builder::new()
                    .keep_alive(true)
                    .serve_connection(io, service)
                    .await?;
            }
            HttpVersion::Http2Only => {
                http2::Builder::new(TokioExecutor::new())
                    .max_concurrent_streams(250)
                    .initial_stream_window_size(65535 * 16)
                    .initial_connection_window_size(65535 * 64)
                    .serve_connection(io, service)
                    .await?;
            }
            HttpVersion::Auto => {
                http1::Builder::new()
                    .keep_alive(true)
                    .serve_connection(io, service)
                    .with_upgrades()
                    .await?;
            }
        }

        Ok(())
    }

    pub fn shutdown(&self) {
        if let Some(ref shutdown) = self.shutdown {
            shutdown.start_shutdown();
        }
    }
}
