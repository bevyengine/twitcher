use std::{
    collections::HashMap,
    io::{BufRead, Write},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use crossbeam::channel::Receiver;
use xshell::{Shell, cmd};

use crate::Metrics;

#[derive(Debug)]
pub struct StressTest {
    pub stress_test: String,
    pub parameters: Vec<(String, Option<String>)>,
    pub nb_frames: u32,
}

impl StressTest {
    pub fn on(
        stress_test: String,
        parameters: Vec<(String, Option<String>)>,
        nb_frames: u32,
    ) -> Self {
        Self {
            stress_test,
            parameters,
            nb_frames,
        }
    }
}

impl Metrics for StressTest {
    fn prepare(&self) {
        let sh = Shell::new().unwrap();
        let stress_tests = self.stress_test.clone();
        let _ = cmd!(
            sh,
            "cargo build --release --features bevy_ci_testing --example {stress_tests}"
        )
        .run();
    }

    fn artifacts(&self) -> HashMap<String, PathBuf> {
        std::fs::File::create("done").unwrap();
        HashMap::from([(
            format!(
                "stress-test-fps.{}.{}",
                self.stress_test,
                self.parameters
                    .iter()
                    .map(|(p, v)| if let Some(v) = v {
                        format!("{}-{}", p, v)
                    } else {
                        p.clone()
                    })
                    .fold("params".to_string(), |acc, s| format!("{}-{}", acc, s))
            ),
            Path::new("done").to_path_buf(),
        )])
    }

    fn collect(&self) -> HashMap<String, u64> {
        let cpu = cpu_usage();

        let key = format!(
            "stress-test-fps.{}.{}",
            self.stress_test,
            self.parameters
                .iter()
                .map(|(p, v)| if let Some(v) = v {
                    format!("{}-{}", p, v)
                } else {
                    p.clone()
                })
                .fold("params".to_string(), |acc, s| format!("{}-{}", acc, s))
        );
        let config = "twitcher_config.ron";
        let mut config_file = std::fs::File::create(config).unwrap();
        config_file
            .write_fmt(format_args!("(events: [({}, AppExit)])", self.nb_frames))
            .unwrap();
        let sh = Shell::new().unwrap();
        sh.set_var("CI_TESTING_CONFIG", config);

        let parameters = self
            .parameters
            .iter()
            .flat_map(|(p, v)| {
                if let Some(v) = v {
                    vec![format!("--{}", p), v.clone()]
                } else {
                    vec![format!("--{}", p)]
                }
            })
            .collect::<Vec<String>>();
        let stress_tests = self.stress_test.clone();
        let cmd = cmd!(
            sh,
            "xvfb-run cargo run --release --features bevy_ci_testing --example {stress_tests} -- {parameters...}"
        );
        let mut results = HashMap::new();

        // Clear channel
        while cpu.try_recv().is_ok() {}

        let start = Instant::now();
        let Ok(output) = cmd.output() else {
            // ignore failure due to a missing stress test
            return results;
        };
        let elapsed = start.elapsed();

        let mut cpu_usage = vec![];
        while let Ok(v) = cpu.try_recv() {
            cpu_usage.push(v);
        }
        // remove first element as that was during startup
        cpu_usage.remove(0);
        std::mem::drop(cpu);

        let fpss = output
            .stdout
            .lines()
            .chain(output.stderr.lines())
            .map_while(|line| line.ok())
            .filter(|line| line.contains("fps"))
            .filter(|line| line.contains("avg"))
            .map(|line| line.split("fps").nth(1).unwrap().to_string())
            .map(|line| line.split("(").nth(0).unwrap().to_string())
            .map(|line| line.split(":").nth(1).unwrap().to_string())
            .map(|line| line.trim().to_string())
            .map(|line| line.parse::<f32>().unwrap())
            .collect::<Vec<_>>();

        results.insert(
            format!("{key}.mean"),
            (statistical::mean(&fpss) * 1000.0) as u64,
        );
        results.insert(
            format!("{key}.median"),
            (statistical::median(&fpss) * 1000.0) as u64,
        );
        results.insert(
            format!("{key}.min"),
            fpss.iter().map(|d| (d * 1000.0) as u64).min().unwrap(),
        );
        results.insert(
            format!("{key}.max"),
            fpss.iter().map(|d| (d * 1000.0) as u64).max().unwrap(),
        );
        results.insert(
            format!("{key}.std_dev"),
            (statistical::standard_deviation(&fpss, None) * 1000.0) as u64,
        );
        results.insert(
            format!("{key}.cpu_usage.mean"),
            (statistical::mean(&cpu_usage) * 1000.0) as u64,
        );
        results.insert(
            format!("{key}.cpu_usage.median"),
            (statistical::median(&cpu_usage) * 1000.0) as u64,
        );
        results.insert(
            format!("{key}.cpu_usage.min"),
            cpu_usage.iter().map(|d| (d * 1000.0) as u64).min().unwrap(),
        );
        results.insert(
            format!("{key}.cpu_usage.max"),
            cpu_usage.iter().map(|d| (d * 1000.0) as u64).max().unwrap(),
        );
        results.insert(
            format!("{key}.cpu_usage.std_dev"),
            (statistical::standard_deviation(&cpu_usage, None) * 1000.0) as u64,
        );
        results.insert(format!("{key}.duration"), elapsed.as_millis() as u64);
        results.insert(format!("{key}.frames"), self.nb_frames as u64);

        results
    }
}

fn cpu_usage() -> Receiver<f32> {
    let (tx, rx) = crossbeam::channel::unbounded();

    thread::spawn(move || {
        let mut sys = sysinfo::System::new();
        let delay = sysinfo::MINIMUM_CPU_UPDATE_INTERVAL.max(Duration::from_secs(1));

        loop {
            sys.refresh_cpu_usage();
            if tx.send(sys.global_cpu_usage()).is_err() {
                break;
            }
            std::thread::sleep(delay);
        }
    });

    rx
}
