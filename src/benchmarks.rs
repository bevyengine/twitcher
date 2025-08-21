use std::{
    collections::HashMap,
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use xshell::{Shell, cmd};

use crate::Metrics;

#[derive(Debug)]
pub struct Benchmarks;

impl Metrics for Benchmarks {
    fn prepare(&self) {
        let sh = Shell::new().unwrap();
        sh.change_dir("benches");
        cmd!(sh, "cargo clean").run().unwrap();
        let out = cmd!(sh, "cargo criterion --message-format json")
            .read()
            .unwrap();
        let benchmarks = out
            .lines()
            .filter(|line| {
                serde_json::from_str::<Message>(line).unwrap().reason == "benchmark-complete"
            })
            .map(|line| serde_json::from_str::<Benchmark>(line).unwrap())
            .collect::<Vec<_>>();

        let file = File::create("benchmarks.json").unwrap();
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &benchmarks).unwrap();
    }

    fn artifacts(&self) -> HashMap<String, PathBuf> {
        HashMap::from([(
            "benchmarks.stats".to_string(),
            Path::new("benchmarks.json").to_path_buf(),
        )])
    }

    fn collect(&self) -> HashMap<String, u64> {
        let timings: Vec<Benchmark> =
            serde_json::from_reader(std::fs::File::open("benchmarks.json").unwrap()).unwrap();
        timings
            .iter()
            .flat_map(|benchmark| {
                let bench_id = benchmark.id.clone();

                ["mean", "slope", "median", "typical", "median_abs_dev"]
                    .iter()
                    .filter_map(|metric| match *metric {
                        "slope" => benchmark.slope.as_ref().map(|slope| (metric, slope)),
                        "mean" => Some((metric, &benchmark.mean)),
                        "median" => Some((metric, &benchmark.median)),
                        "typical" => Some((metric, &benchmark.typical)),
                        "median_abs_dev" => Some((metric, &benchmark.median_abs_dev)),
                        _ => unreachable!(),
                    })
                    .flat_map(move |(metric, timings)| {
                        vec![
                            (
                                format!("benchmarks.{bench_id}.{metric}"),
                                timings.estimate(),
                            ),
                            (
                                format!("benchmarks.{bench_id}.{metric}_lower"),
                                timings.lower_bound(),
                            ),
                            (
                                format!("benchmarks.{bench_id}.{metric}_upper"),
                                timings.upper_bound(),
                            ),
                        ]
                    })
            })
            .collect()
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    reason: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Benchmark {
    id: String,
    typical: Timings,
    mean: Timings,
    median: Timings,
    median_abs_dev: Timings,
    slope: Option<Timings>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Timings {
    estimate: f32,
    lower_bound: f32,
    upper_bound: f32,
    unit: String,
}

impl Timings {
    fn get(&self, value: f32) -> u64 {
        match self.unit.as_str() {
            "ns" => (value * 1_000.0) as u64,
            "us" => (value * 1_000_000.0) as u64,
            "ms" => (value * 1_000_000_000.0) as u64,
            "s" => (value * 1_000_000_000_000.0) as u64,
            _ => panic!("Unsupported unit"),
        }
    }

    fn estimate(&self) -> u64 {
        self.get(self.estimate)
    }
    fn lower_bound(&self) -> u64 {
        self.get(self.lower_bound)
    }
    fn upper_bound(&self) -> u64 {
        self.get(self.upper_bound)
    }
}
