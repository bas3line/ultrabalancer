use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Route {
    pub path_prefix: Option<String>,
    pub path_exact: Option<String>,
    pub path_regex: Option<String>,
    pub host: Option<String>,
    pub methods: Option<Vec<String>>,
    pub headers: Option<HashMap<String, String>>,
    pub backend_group: String,
    pub priority: i32,
    pub rewrite_path: Option<String>,
    pub add_headers: HashMap<String, String>,
    pub remove_headers: Vec<String>,
}

impl Route {
    pub fn new(backend_group: String) -> Self {
        Self {
            path_prefix: None,
            path_exact: None,
            path_regex: None,
            host: None,
            methods: None,
            headers: None,
            backend_group,
            priority: 0,
            rewrite_path: None,
            add_headers: HashMap::new(),
            remove_headers: Vec::new(),
        }
    }

    pub fn with_path_prefix(mut self, prefix: &str) -> Self {
        self.path_prefix = Some(prefix.to_string());
        self
    }

    pub fn with_path_exact(mut self, path: &str) -> Self {
        self.path_exact = Some(path.to_string());
        self
    }

    pub fn with_host(mut self, host: &str) -> Self {
        self.host = Some(host.to_string());
        self
    }

    pub fn with_methods(mut self, methods: Vec<&str>) -> Self {
        self.methods = Some(methods.into_iter().map(|m| m.to_string()).collect());
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_rewrite(mut self, rewrite: &str) -> Self {
        self.rewrite_path = Some(rewrite.to_string());
        self
    }

    pub fn matches(&self, method: &str, path: &str, host: Option<&str>, headers: &HashMap<String, String>) -> bool {
        if let Some(ref allowed_methods) = self.methods {
            if !allowed_methods.iter().any(|m| m.eq_ignore_ascii_case(method)) {
                return false;
            }
        }

        if let Some(ref required_host) = self.host {
            match host {
                Some(h) if h == required_host || h.ends_with(&format!(".{}", required_host)) => {}
                _ => return false,
            }
        }

        if let Some(ref exact) = self.path_exact {
            if path != exact {
                return false;
            }
        }

        if let Some(ref prefix) = self.path_prefix {
            if !path.starts_with(prefix) {
                return false;
            }
        }

        if let Some(ref required_headers) = self.headers {
            for (key, value) in required_headers {
                match headers.get(key) {
                    Some(v) if v == value => {}
                    _ => return false,
                }
            }
        }

        true
    }

    pub fn apply_rewrite(&self, path: &str) -> String {
        if let (Some(prefix), Some(rewrite)) = (&self.path_prefix, &self.rewrite_path) {
            if path.starts_with(prefix) {
                return format!("{}{}", rewrite, &path[prefix.len()..]);
            }
        }
        path.to_string()
    }
}

pub struct Router {
    routes: Vec<Route>,
    default_backend: String,
}

impl Router {
    pub fn new(default_backend: String) -> Self {
        Self {
            routes: Vec::new(),
            default_backend,
        }
    }

    pub fn add_route(&mut self, route: Route) {
        self.routes.push(route);
        self.routes.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn remove_route(&mut self, backend_group: &str) -> bool {
        let before = self.routes.len();
        self.routes.retain(|r| r.backend_group != backend_group);
        self.routes.len() < before
    }

    pub fn match_route(&self, method: &str, path: &str, host: Option<&str>, headers: &HashMap<String, String>) -> (&str, String) {
        for route in &self.routes {
            if route.matches(method, path, host, headers) {
                let final_path = route.apply_rewrite(path);
                return (&route.backend_group, final_path);
            }
        }
        (&self.default_backend, path.to_string())
    }

    pub fn routes_count(&self) -> usize {
        self.routes.len()
    }
}

impl Clone for Router {
    fn clone(&self) -> Self {
        Self {
            routes: self.routes.clone(),
            default_backend: self.default_backend.clone(),
        }
    }
}
