pub mod handler;

use crate::backend::ServerPool;
use crate::balancer::LoadBalancerSelector;
use crate::metrics::MetricsCollector;
use handler::RequestHandler;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};

pub struct ProxyServer {
    handler: RequestHandler,
    listen_addr: String,
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
        }
    }

    pub async fn start(self: Arc<Self>) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&self.listen_addr).await?;
        info!("✓ Load balancer listening on {}", self.listen_addr);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
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
        let io = TokioIo::new(stream);
        let handler = self.handler.clone();

        let service = service_fn(move |req| {
            let handler = handler.clone();
            async move { handler.handle(req, addr).await }
        });

        http1::Builder::new()
            .serve_connection(io, service)
            .await?;

        Ok(())
    }
}
