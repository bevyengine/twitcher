use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use xshell::{Shell, cmd};

use crate::Metrics;

#[derive(Debug)]
pub struct CompileTime {
    pub example_name: String,
    pub nb_jobs: u32,
}

impl CompileTime {
    pub fn on(example_name: String, nb_jobs: u32) -> Self {
        Self {
            example_name: if example_name.is_empty() {
                "breakout".to_string()
            } else {
                example_name
            },
            nb_jobs: if nb_jobs == 0 { 8 } else { nb_jobs },
        }
    }
}

impl Metrics for CompileTime {
    fn prepare(&self) -> bool {
        let command = format!(
            "cargo build --jobs {} --release --example {}",
            self.nb_jobs, self.example_name
        );
        let sh = Shell::new().unwrap();
        let json = format!("build-{}.json", self.nb_jobs);
        cmd!(
            sh,
            "hyperfine --export-json {json} --prepare 'cargo clean; sleep 2' {command}"
        )
        .run()
        .is_ok()
    }

    fn artifacts(&self) -> HashMap<String, PathBuf> {
        HashMap::from([(
            "compile-time.stats".to_string(),
            Path::new(&format!("build-{}.json", self.nb_jobs)).to_path_buf(),
        )])
    }

    fn collect(&self) -> HashMap<String, u64> {
        let key = format!(
            "compile-time-{}-{}-{}",
            std::env::consts::FAMILY,
            std::env::consts::ARCH,
            self.nb_jobs
        );
        let results: Hyperfine = serde_json::from_reader(
            std::fs::File::open(format!("build-{}.json", self.nb_jobs)).unwrap(),
        )
        .unwrap();
        HashMap::from([
            (
                format!("{key}.mean"),
                (results.results[0].mean * 1000.0) as u64,
            ),
            (
                format!("{key}.stddev"),
                (results.results[0].stddev.unwrap_or_default() * 1000.0) as u64,
            ),
            (
                format!("{key}.median"),
                (results.results[0].median * 1000.0) as u64,
            ),
            (
                format!("{key}.user"),
                (results.results[0].user * 1000.0) as u64,
            ),
            (
                format!("{key}.system"),
                (results.results[0].system * 1000.0) as u64,
            ),
            (
                format!("{key}.min"),
                (results.results[0].min * 1000.0) as u64,
            ),
            (
                format!("{key}.max"),
                (results.results[0].max * 1000.0) as u64,
            ),
        ])
    }
}

#[derive(Deserialize)]
struct Hyperfine {
    results: Vec<HyperfineResults>,
}
#[derive(Deserialize)]
struct HyperfineResults {
    mean: f32,
    max: f32,
    min: f32,
    stddev: Option<f32>,
    median: f32,
    user: f32,
    system: f32,
}
