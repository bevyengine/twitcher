use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Stats {
    pub metrics: HashMap<String, u64>,
    pub commit: String,
    pub timestamp: u128,
    pub commit_timestamp: u128,
    pub rust: Rust,
    pub host: Host,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Rust {
    pub stable: String,
    pub nightly: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Host {
    pub hostname: String,
    pub os_version: String,
}

pub fn find_stats_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if entry.file_type().unwrap().is_file()
                && name.starts_with("stats")
                && name.ends_with(".json")
            {
                files.push(entry.path());
            }
            if entry.file_type().unwrap().is_dir() {
                files.extend(find_stats_files(&entry.path()));
            }
        }
    }
    files
}
