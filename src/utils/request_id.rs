use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct RequestId;

impl RequestId {
    pub fn generate() -> String {
        Uuid::new_v4().to_string()
    }

    pub fn generate_short() -> String {
        let uuid = Uuid::new_v4();
        let bytes = uuid.as_bytes();
        format!(
            "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]
        )
    }

    pub fn generate_sequential() -> String {
        let count = COUNTER.fetch_add(1, Ordering::Relaxed);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        format!("{:016x}-{:08x}", timestamp, count as u32)
    }

    pub fn extract_from_headers(headers: &http::HeaderMap) -> Option<String> {
        for header_name in &[
            "X-Request-ID",
            "X-Request-Id",
            "x-request-id",
            "X-Correlation-ID",
        ] {
            if let Some(value) = headers.get(*header_name) {
                if let Ok(s) = value.to_str() {
                    return Some(s.to_string());
                }
            }
        }
        None
    }
}
