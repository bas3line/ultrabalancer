use dashmap::DashMap;
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;

type GlobalLimiter = GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

pub struct RateLimiter {
    global: Arc<GlobalLimiter>,
    per_ip: Option<Arc<DashMap<String, Arc<GlobalLimiter>>>>,
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

    pub fn check(&self) -> bool {
        self.global.check().is_ok()
    }

    pub fn check_ip(&self, ip: &str) -> bool {
        if !self.check() {
            return false;
        }

        if let (Some(map), Some(quota)) = (&self.per_ip, &self.per_ip_quota) {
            let limiter = map
                .entry(ip.to_string())
                .or_insert_with(|| Arc::new(GovernorRateLimiter::direct(quota.clone())))
                .clone();
            return limiter.check().is_ok();
        }

        true
    }

    pub async fn wait(&self) {
        self.global.until_ready().await;
    }

    pub async fn wait_ip(&self, ip: &str) {
        self.wait().await;

        if let (Some(map), Some(quota)) = (&self.per_ip, &self.per_ip_quota) {
            let limiter = map
                .entry(ip.to_string())
                .or_insert_with(|| Arc::new(GovernorRateLimiter::direct(quota.clone())))
                .clone();
            limiter.until_ready().await;
        }
    }

    pub fn cleanup_stale_limiters(&self) {
        if let Some(ref map) = self.per_ip {
            map.retain(|_, limiter| Arc::strong_count(limiter) > 1);
        }
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            global: Arc::clone(&self.global),
            per_ip: self.per_ip.as_ref().map(Arc::clone),
            per_ip_quota: self.per_ip_quota.clone(),
        }
    }
}
