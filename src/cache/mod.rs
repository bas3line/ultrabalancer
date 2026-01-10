use bytes::Bytes;
use moka::future::Cache;
use moka::Expiry;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use xxhash_rust::xxh3::xxh3_64;

#[derive(Clone)]
pub struct CachedResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
    pub created_at: u64,
    pub ttl: Duration,
}

struct PerEntryExpiry;

impl Expiry<u64, Arc<CachedResponse>> for PerEntryExpiry {
    fn expire_after_create(
        &self,
        _key: &u64,
        value: &Arc<CachedResponse>,
        _current_time: Instant,
    ) -> Option<Duration> {
        Some(value.ttl)
    }

    fn expire_after_read(
        &self,
        _key: &u64,
        _value: &Arc<CachedResponse>,
        _current_time: Instant,
        _current_duration: Option<Duration>,
        _last_modified_at: Instant,
    ) -> Option<Duration> {
        None
    }

    fn expire_after_update(
        &self,
        _key: &u64,
        value: &Arc<CachedResponse>,
        _current_time: Instant,
        _current_duration: Option<Duration>,
    ) -> Option<Duration> {
        Some(value.ttl)
    }
}

pub struct ResponseCache {
    cache: Cache<u64, Arc<CachedResponse>>,
    hits: AtomicU64,
    misses: AtomicU64,
    max_size: u64,
    default_ttl: Duration,
}

impl ResponseCache {
    pub fn new(max_entries: u64, default_ttl_secs: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_entries)
            .expire_after(PerEntryExpiry)
            .build();

        Self {
            cache,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            max_size: max_entries,
            default_ttl: Duration::from_secs(default_ttl_secs),
        }
    }

    pub fn cache_key(method: &str, uri: &str, vary_headers: Option<&str>) -> u64 {
        let mut key_data = format!("{}:{}", method, uri);
        if let Some(vary) = vary_headers {
            key_data.push_str(vary);
        }
        xxh3_64(key_data.as_bytes())
    }

    pub async fn get(&self, key: u64) -> Option<Arc<CachedResponse>> {
        match self.cache.get(&key).await {
            Some(resp) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(resp)
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    pub async fn set(&self, key: u64, response: CachedResponse) {
        self.cache.insert(key, Arc::new(response)).await;
    }

    pub async fn set_with_ttl(&self, key: u64, response: CachedResponse, _ttl: Duration) {
        self.cache.insert(key, Arc::new(response)).await;
    }

    pub async fn invalidate(&self, key: u64) {
        self.cache.invalidate(&key).await;
    }

    /// Invalidates a cache entry by string key (hashes the string to get the key)
    pub async fn invalidate_by_str(&self, key_str: &str) {
        let key = xxh3_64(key_str.as_bytes());
        self.cache.invalidate(&key).await;
    }

    pub async fn clear(&self) {
        self.cache.invalidate_all();
        self.cache.run_pending_tasks().await;
    }

    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;

        CacheStats {
            hits,
            misses,
            hit_rate: if total > 0 {
                (hits as f64 / total as f64) * 100.0
            } else {
                0.0
            },
            size: self.cache.entry_count(),
            max_size: self.max_size,
        }
    }

    pub fn should_cache(status: u16, method: &str) -> bool {
        matches!(method, "GET" | "HEAD")
            && matches!(status, 200 | 203 | 204 | 206 | 300 | 301 | 308 | 404 | 410)
    }

    pub fn parse_cache_control(header: &str) -> CacheControl {
        let mut cc = CacheControl::default();

        for directive in header.split(',').map(|s| s.trim()) {
            if directive.eq_ignore_ascii_case("no-cache") {
                cc.no_cache = true;
            } else if directive.eq_ignore_ascii_case("no-store") {
                cc.no_store = true;
            } else if directive.eq_ignore_ascii_case("private") {
                cc.private = true;
            } else if directive.eq_ignore_ascii_case("public") {
                cc.public = true;
            } else if let Some(age) = directive.strip_prefix("max-age=") {
                cc.max_age = age.parse().ok();
            } else if let Some(age) = directive.strip_prefix("s-maxage=") {
                cc.s_maxage = age.parse().ok();
            }
        }

        cc
    }
}

impl Clone for ResponseCache {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            hits: AtomicU64::new(self.hits.load(Ordering::Relaxed)),
            misses: AtomicU64::new(self.misses.load(Ordering::Relaxed)),
            max_size: self.max_size,
            default_ttl: self.default_ttl,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CacheControl {
    pub no_cache: bool,
    pub no_store: bool,
    pub private: bool,
    pub public: bool,
    pub max_age: Option<u64>,
    pub s_maxage: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub size: u64,
    pub max_size: u64,
}
