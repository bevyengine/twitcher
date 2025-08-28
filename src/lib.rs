use std::{collections::HashMap, path::PathBuf};

pub mod benchmarks;
pub mod binary_size;
pub mod compile_time;
pub mod crate_compile_time;
pub mod stats;
pub mod stress_tests;
pub mod wasm_binary_size;

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
