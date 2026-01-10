use bytes::Bytes;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::net::TcpListener;

static REQUEST_COUNT: AtomicU64 = AtomicU64::new(0);

async fn handle_request(
    _req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let count = REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);

    let body = format!(r#"{{"status":"ok","request_id":{}}}"#, count);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let port = std::env::args()
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(9001);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("Mock backend listening on port {}", port);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::spawn(async move {
            if let Err(e) = http1::Builder::new()
                .keep_alive(true)
                .serve_connection(io, service_fn(handle_request))
                .await
            {
                eprintln!("Connection error: {}", e);
            }
        });
    }
}
