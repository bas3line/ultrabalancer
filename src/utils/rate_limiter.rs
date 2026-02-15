use dashmap::DashMap;
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

type GlobalLimiter = GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

/// Per-IP limiter entry with last access time for cleanup
struct IpLimiterEntry {
    limiter: Arc<GlobalLimiter>,
    last_access: AtomicU64,
}

pub struct RateLimiter {
    global: Arc<GlobalLimiter>,
    per_ip: Option<Arc<DashMap<String, IpLimiterEntry>>>,
    per_ip_quota: Option<Quota>,
}

impl RateLimiter {
    pub fn new(requests_per_second: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap());
        let limiter = GovernorRateLimiter::direct(quota);

        Self {
            global: Arc::new(limiter),
            per_ip: None,
            per_ip_quota: None,
        }
    }

    pub fn with_burst(requests_per_second: u32, burst_size: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap())
            .allow_burst(NonZeroU32::new(burst_size).unwrap());
        let limiter = GovernorRateLimiter::direct(quota);

        Self {
            global: Arc::new(limiter),
            per_ip: None,
            per_ip_quota: None,
        }
    }

    pub fn with_per_ip_limit(mut self, requests_per_second: u32, burst: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap())
            .allow_burst(NonZeroU32::new(burst).unwrap());
        self.per_ip = Some(Arc::new(DashMap::new()));
        self.per_ip_quota = Some(quota);
        self
    }

    fn current_time_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    pub fn check(&self) -> bool {
        self.global.check().is_ok()
    }

    pub fn check_ip(&self, ip: &str) -> bool {
        if !self.check() {
            return false;
        }

        if let (Some(map), Some(quota)) = (&self.per_ip, &self.per_ip_quota) {
            let entry = map.entry(ip.to_string()).or_insert_with(|| IpLimiterEntry {
                limiter: Arc::new(GovernorRateLimiter::direct(*quota)),
                last_access: AtomicU64::new(Self::current_time_secs()),
            });
            entry
                .last_access
                .store(Self::current_time_secs(), Ordering::Relaxed);
            return entry.limiter.check().is_ok();
        }

        true
    }

    pub async fn wait(&self) {
        self.global.until_ready().await;
    }

    pub async fn wait_ip(&self, ip: &str) {
        self.wait().await;

        if let (Some(map), Some(quota)) = (&self.per_ip, &self.per_ip_quota) {
            let entry = map.entry(ip.to_string()).or_insert_with(|| IpLimiterEntry {
                limiter: Arc::new(GovernorRateLimiter::direct(*quota)),
                last_access: AtomicU64::new(Self::current_time_secs()),
            });
            entry
                .last_access
                .store(Self::current_time_secs(), Ordering::Relaxed);
            entry.limiter.until_ready().await;
        }
    }

    /// Remove per-IP limiters that haven't been accessed in the given duration.
    /// Called automatically by `start_cleanup_task`.
    pub fn cleanup_stale_limiters(&self, max_idle_secs: u64) {
        if let Some(ref map) = self.per_ip {
            let now = Self::current_time_secs();
            map.retain(|_, entry| {
                let last = entry.last_access.load(Ordering::Relaxed);
                now.saturating_sub(last) < max_idle_secs
            });
        }
    }

    /// Returns the number of per-IP limiters currently tracked
    pub fn per_ip_count(&self) -> usize {
        self.per_ip.as_ref().map(|m| m.len()).unwrap_or(0)
    }
}

/// Starts a background task that periodically cleans up stale per-IP limiters.
/// Default: runs every 60 seconds, removes limiters idle for > 300 seconds (5 minutes).
pub fn start_rate_limiter_cleanup_task(
    limiter: Arc<RateLimiter>,
    interval_secs: u64,
    max_idle_secs: u64,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;
            limiter.cleanup_stale_limiters(max_idle_secs);
        }
    });
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            global: Arc::clone(&self.global),
            per_ip: self.per_ip.as_ref().map(Arc::clone),
            per_ip_quota: self.per_ip_quota,
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(10000) // 10k requests/second default
    }
}
