use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{info, warn};

pub struct GracefulShutdown {
    is_shutting_down: AtomicBool,
    active_connections: AtomicUsize,
    shutdown_tx: broadcast::Sender<()>,
    drain_timeout: Duration,
}

impl GracefulShutdown {
    pub fn new(drain_timeout_secs: u64) -> (Arc<Self>, broadcast::Receiver<()>) {
        let (tx, rx) = broadcast::channel(1);
        let shutdown = Arc::new(Self {
            is_shutting_down: AtomicBool::new(false),
            active_connections: AtomicUsize::new(0),
            shutdown_tx: tx,
            drain_timeout: Duration::from_secs(drain_timeout_secs),
        });
        (shutdown, rx)
    }

    pub fn is_shutting_down(&self) -> bool {
        self.is_shutting_down.load(Ordering::Acquire)
    }

    pub fn start_shutdown(&self) {
        if self.is_shutting_down.swap(true, Ordering::Release) {
            return;
        }
        info!("Initiating graceful shutdown");
        let _ = self.shutdown_tx.send(());
    }

    pub fn connection_started(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn connection_ended(&self) {
        let _ =
            self.active_connections
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                    current.checked_sub(1)
                });
    }

    pub fn active_connections(&self) -> usize {
        self.active_connections.load(Ordering::Relaxed)
    }

    pub async fn wait_for_connections(&self) {
        let start = std::time::Instant::now();

        while self.active_connections() > 0 {
            if start.elapsed() > self.drain_timeout {
                warn!(
                    "Drain timeout exceeded, {} connections still active",
                    self.active_connections()
                );
                break;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        info!("All connections drained");
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }
}

pub async fn wait_for_shutdown_signal(shutdown: Arc<GracefulShutdown>) {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C"),
        _ = terminate => info!("Received SIGTERM"),
    }

    shutdown.start_shutdown();
    shutdown.wait_for_connections().await;
}
