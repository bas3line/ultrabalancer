use bytes::Bytes;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error, info};

static WS_CONNECTIONS: AtomicU64 = AtomicU64::new(0);
static WS_MESSAGES: AtomicU64 = AtomicU64::new(0);

pub struct WebSocketProxy {
    backend_url: String,
}

impl WebSocketProxy {
    pub fn new(backend_url: String) -> Self {
        Self { backend_url }
    }

    pub async fn proxy(
        &self,
        client_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> anyhow::Result<()> {
        WS_CONNECTIONS.fetch_add(1, Ordering::Relaxed);

        let backend_result = connect_async(&self.backend_url).await;
        let (backend_stream, _) = match backend_result {
            Ok(result) => result,
            Err(e) => {
                WS_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
                return Err(anyhow::anyhow!("Failed to connect to backend: {}", e));
            }
        };

        let (client_write, client_read) = client_stream.split();
        let (backend_write, backend_read) = backend_stream.split();

        let client_to_backend = Self::forward_messages(client_read, backend_write, "client->backend");
        let backend_to_client = Self::forward_messages(backend_read, client_write, "backend->client");

        tokio::select! {
            result = client_to_backend => {
                if let Err(e) = result {
                    debug!("Client to backend closed: {}", e);
                }
            }
            result = backend_to_client => {
                if let Err(e) = result {
                    debug!("Backend to client closed: {}", e);
                }
            }
        }

        WS_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
        Ok(())
    }

    async fn forward_messages<R, W>(
        mut read: SplitStream<WebSocketStream<R>>,
        mut write: SplitSink<WebSocketStream<W>, Message>,
        direction: &str,
    ) -> anyhow::Result<()>
    where
        R: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
        W: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(message) => {
                    WS_MESSAGES.fetch_add(1, Ordering::Relaxed);
                    if message.is_close() {
                        let _ = write.send(message).await;
                        break;
                    }
                    if let Err(e) = write.send(message).await {
                        debug!("{} send error: {}", direction, e);
                        break;
                    }
                }
                Err(e) => {
                    debug!("{} receive error: {}", direction, e);
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn active_connections() -> u64 {
        WS_CONNECTIONS.load(Ordering::Relaxed)
    }

    pub fn total_messages() -> u64 {
        WS_MESSAGES.load(Ordering::Relaxed)
    }
}

pub fn is_websocket_upgrade(headers: &http::HeaderMap) -> bool {
    if let Some(upgrade) = headers.get(http::header::UPGRADE) {
        if let Ok(value) = upgrade.to_str() {
            return value.eq_ignore_ascii_case("websocket");
        }
    }
    false
}

pub fn is_websocket_request(headers: &http::HeaderMap) -> bool {
    let has_upgrade = headers
        .get(http::header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false);

    let has_connection = headers
        .get(http::header::CONNECTION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_lowercase().contains("upgrade"))
        .unwrap_or(false);

    let has_ws_key = headers.contains_key("sec-websocket-key");

    has_upgrade && has_connection && has_ws_key
}

pub struct WebSocketStats {
    pub active_connections: u64,
    pub total_messages: u64,
}

impl WebSocketStats {
    pub fn current() -> Self {
        Self {
            active_connections: WS_CONNECTIONS.load(Ordering::Relaxed),
            total_messages: WS_MESSAGES.load(Ordering::Relaxed),
        }
    }
}
