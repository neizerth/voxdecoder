//! Resource Class leases.

use std::collections::BTreeMap;

use crate::config::ResourceClassConfig;

#[derive(Debug, Clone)]
pub struct ResourceManager {
    capacity: BTreeMap<String, u32>,
    leased: BTreeMap<String, u32>,
}

impl ResourceManager {
    pub fn new(classes: &BTreeMap<String, ResourceClassConfig>) -> Self {
        let capacity = classes
            .iter()
            .map(|(k, v)| (k.clone(), v.capacity))
            .collect();
        Self {
            capacity,
            leased: BTreeMap::new(),
        }
    }

    pub fn can_lease(&self, need: &BTreeMap<String, u32>) -> bool {
        for (k, n) in need {
            let cap = self.capacity.get(k).copied().unwrap_or(0);
            let used = self.leased.get(k).copied().unwrap_or(0);
            if used.saturating_add(*n) > cap {
                return false;
            }
        }
        true
    }

    pub fn lease(&mut self, need: &BTreeMap<String, u32>) -> bool {
        if !self.can_lease(need) {
            return false;
        }
        for (k, n) in need {
            *self.leased.entry(k.clone()).or_insert(0) += *n;
        }
        true
    }

    pub fn release(&mut self, need: &BTreeMap<String, u32>) {
        for (k, n) in need {
            if let Some(v) = self.leased.get_mut(k) {
                *v = v.saturating_sub(*n);
            }
        }
    }

    pub fn snapshot(&self) -> BTreeMap<String, (u32, u32)> {
        let mut out = BTreeMap::new();
        for (k, cap) in &self.capacity {
            let used = self.leased.get(k).copied().unwrap_or(0);
            out.insert(k.clone(), (used, *cap));
        }
        out
    }
}

/// Aggregate resource needs for a Job (sum of non-terminal nodes).
pub fn job_resource_need(
    nodes: &[crate::store::NodeRecord],
) -> BTreeMap<String, u32> {
    let mut need = BTreeMap::new();
    for n in nodes {
        if n.status.is_terminal() {
            continue;
        }
        for (k, v) in &n.resources {
            *need.entry(k.clone()).or_insert(0) = (*need.get(k).unwrap_or(&0)).max(*v);
        }
    }
    if need.is_empty() {
        need.insert("cpu".into(), 1);
    }
    need
}
