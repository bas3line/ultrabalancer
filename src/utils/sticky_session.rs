use ahash::AHashMap;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone)]
pub struct SessionEntry {
    pub backend: String,
    pub created_at: Instant,
    pub last_access: Instant,
}

pub struct StickySessionManager {
    sessions: DashMap<String, SessionEntry>,
    cookie_name: String,
    ttl: Duration,
    /// If true, cookies will include the Secure flag (for TLS deployments)
    secure_cookies: bool,
}

impl StickySessionManager {
    pub fn new(cookie_name: &str, ttl_secs: u64) -> Self {
        Self {
            sessions: DashMap::new(),
            cookie_name: cookie_name.to_string(),
            ttl: Duration::from_secs(ttl_secs),
            secure_cookies: false,
        }
    }

    /// Enable Secure flag on cookies (should be true when TLS is enabled)
    pub fn with_secure_cookies(mut self, secure: bool) -> Self {
        self.secure_cookies = secure;
        self
    }

    pub fn get_backend(&self, session_id: &str) -> Option<String> {
        if let Some(mut entry) = self.sessions.get_mut(session_id) {
            if entry.last_access.elapsed() < self.ttl {
                entry.last_access = Instant::now();
                return Some(entry.backend.clone());
            } else {
                drop(entry);
                self.sessions.remove(session_id);
            }
        }
        None
    }

    pub fn set_backend(&self, session_id: &str, backend: &str) {
        let now = Instant::now();
        self.sessions.insert(
            session_id.to_string(),
            SessionEntry {
                backend: backend.to_string(),
                created_at: now,
                last_access: now,
            },
        );
    }

    pub fn remove_session(&self, session_id: &str) {
        self.sessions.remove(session_id);
    }

    pub fn generate_session_id(&self) -> String {
        Uuid::new_v4().simple().to_string()
    }

    pub fn extract_session_from_cookie(&self, cookie_header: &str) -> Option<String> {
        for cookie in cookie_header.split(';') {
            let cookie = cookie.trim();
            if let Some(value) = cookie.strip_prefix(&format!("{}=", self.cookie_name)) {
                return Some(value.to_string());
            }
        }
        None
    }

    /// Creates a session cookie. Includes Secure flag if `secure_cookies` is enabled.
    pub fn create_cookie(&self, session_id: &str) -> String {
        if self.secure_cookies {
            format!(
                "{}={}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age={}",
                self.cookie_name,
                session_id,
                self.ttl.as_secs()
            )
        } else {
            format!(
                "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
                self.cookie_name,
                session_id,
                self.ttl.as_secs()
            )
        }
    }

    /// Returns whether secure cookies are enabled
    pub fn is_secure(&self) -> bool {
        self.secure_cookies
    }

    pub fn cookie_name(&self) -> &str {
        &self.cookie_name
    }

    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        self.sessions.retain(|_, entry| {
            now.duration_since(entry.last_access) < self.ttl
        });
    }

    pub fn active_sessions(&self) -> usize {
        self.sessions.len()
    }

    pub fn sessions_per_backend(&self) -> AHashMap<String, usize> {
        let mut counts = AHashMap::new();
        for entry in self.sessions.iter() {
            *counts.entry(entry.backend.clone()).or_insert(0) += 1;
        }
        counts
    }
}

impl Clone for StickySessionManager {
    fn clone(&self) -> Self {
        Self {
            sessions: self.sessions.clone(),
            cookie_name: self.cookie_name.clone(),
            ttl: self.ttl,
            secure_cookies: self.secure_cookies,
        }
    }
}

pub fn start_cleanup_task(manager: Arc<StickySessionManager>, interval_secs: u64) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;
            manager.cleanup_expired();
        }
    });
}
