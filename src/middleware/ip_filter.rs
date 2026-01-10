use ipnet::IpNet;
use std::collections::HashSet;
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Clone)]
pub struct IpFilter {
    whitelist: Vec<IpNet>,
    blacklist: Vec<IpNet>,
    whitelist_enabled: bool,
    blacklist_enabled: bool,
}

impl IpFilter {
    pub fn new() -> Self {
        Self {
            whitelist: Vec::new(),
            blacklist: Vec::new(),
            whitelist_enabled: false,
            blacklist_enabled: false,
        }
    }

    pub fn with_whitelist(mut self, cidrs: Vec<String>) -> Self {
        self.whitelist = cidrs
            .into_iter()
            .filter_map(|c| IpNet::from_str(&c).ok())
            .collect();
        self.whitelist_enabled = !self.whitelist.is_empty();
        self
    }

    pub fn with_blacklist(mut self, cidrs: Vec<String>) -> Self {
        self.blacklist = cidrs
            .into_iter()
            .filter_map(|c| IpNet::from_str(&c).ok())
            .collect();
        self.blacklist_enabled = !self.blacklist.is_empty();
        self
    }

    pub fn add_to_whitelist(&mut self, cidr: &str) -> bool {
        if let Ok(net) = IpNet::from_str(cidr) {
            self.whitelist.push(net);
            self.whitelist_enabled = true;
            true
        } else {
            false
        }
    }

    pub fn add_to_blacklist(&mut self, cidr: &str) -> bool {
        if let Ok(net) = IpNet::from_str(cidr) {
            self.blacklist.push(net);
            self.blacklist_enabled = true;
            true
        } else {
            false
        }
    }

    pub fn remove_from_whitelist(&mut self, cidr: &str) -> bool {
        if let Ok(net) = IpNet::from_str(cidr) {
            let before = self.whitelist.len();
            self.whitelist.retain(|n| n != &net);
            self.whitelist_enabled = !self.whitelist.is_empty();
            self.whitelist.len() < before
        } else {
            false
        }
    }

    // pub fn remove_from_whitelist_all(&mut self) {
    //     self.whitelist.clear();
    //     self.whitelist_enabled = false;
    // }

    pub fn remove_from_blacklist(&mut self, cidr: &str) -> bool {
        if let Ok(net) = IpNet::from_str(cidr) {
            let before = self.blacklist.len();
            self.blacklist.retain(|n| n != &net);
            self.blacklist_enabled = !self.blacklist.is_empty();
            self.blacklist.len() < before
        } else {
            false
        }
    }

    pub fn is_allowed(&self, ip: IpAddr) -> bool {
        if self.blacklist_enabled {
            for net in &self.blacklist {
                if net.contains(&ip) {
                    return false;
                }
            }
        }

        if self.whitelist_enabled {
            for net in &self.whitelist {
                if net.contains(&ip) {
                    return true;
                }
            }
            return false;
        }

        true
    }

    pub fn is_allowed_str(&self, ip_str: &str) -> bool {
        match IpAddr::from_str(ip_str) {
            Ok(ip) => self.is_allowed(ip),
            Err(_) => false,
        }
    }

    pub fn whitelist_count(&self) -> usize {
        self.whitelist.len()
    }

    pub fn blacklist_count(&self) -> usize {
        self.blacklist.len()
    }
}

impl Default for IpFilter {
    fn default() -> Self {
        Self::new()
    }
}
