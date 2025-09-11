use std::{collections::HashMap, path::PathBuf};

pub mod metrics;
pub mod stats;

pub trait Metrics: std::fmt::Debug {
    fn prepare(&self) -> bool;
    fn artifacts(&self) -> HashMap<String, PathBuf> {
        HashMap::new()
    }
    fn collect(&self) -> HashMap<String, u64>;
}

pub fn file_safe_metric_name(metric: &str) -> String {
    metric.replace([':', '/'], "_")
}
