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
        let gpu = gpu_usage();

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

        // Wait for the monitoring threads to start
        let _ = cpu.recv();
        let _ = gpu.recv();
        // Clear channels
        while cpu.try_recv().is_ok() {}
        while gpu.try_recv().is_ok() {}

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
        cpu_usage.remove(0);
        std::mem::drop(cpu);
        let mut gpu_usage = vec![];
        while let Ok(v) = gpu.try_recv() {
            gpu_usage.push(v);
        }
        // remove first element as that was during startup
        gpu_usage.remove(0);
        gpu_usage.remove(0);
        std::mem::drop(gpu);
        let gpu_memory = gpu_usage.iter().map(|v| v.mem as f32).collect::<Vec<_>>();
        let gpu_usage = gpu_usage.iter().map(|v| v.sm as f32).collect::<Vec<_>>();

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
        results.insert(
            format!("{key}.gpu_usage.mean"),
            (statistical::mean(&gpu_usage) * 1000.0) as u64,
        );
        results.insert(
            format!("{key}.gpu_usage.median"),
            (statistical::median(&gpu_usage) * 1000.0) as u64,
        );
        results.insert(
            format!("{key}.gpu_usage.min"),
            gpu_usage.iter().map(|d| (d * 1000.0) as u64).min().unwrap(),
        );
        results.insert(
            format!("{key}.gpu_usage.max"),
            gpu_usage.iter().map(|d| (d * 1000.0) as u64).max().unwrap(),
        );
        results.insert(
            format!("{key}.gpu_usage.std_dev"),
            (statistical::standard_deviation(&gpu_usage, None) * 1000.0) as u64,
        );
        results.insert(
            format!("{key}.gpu_memory.mean"),
            (statistical::mean(&gpu_memory) * 1000.0) as u64,
        );
        results.insert(
            format!("{key}.gpu_memory.median"),
            (statistical::median(&gpu_memory) * 1000.0) as u64,
        );
        results.insert(
            format!("{key}.gpu_memory.min"),
            gpu_memory
                .iter()
                .map(|d| (d * 1000.0) as u64)
                .min()
                .unwrap(),
        );
        results.insert(
            format!("{key}.gpu_memory.max"),
            gpu_memory
                .iter()
                .map(|d| (d * 1000.0) as u64)
                .max()
                .unwrap(),
        );
        results.insert(
            format!("{key}.gpu_memory.std_dev"),
            (statistical::standard_deviation(&gpu_memory, None) * 1000.0) as u64,
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

#[derive(Debug)]
struct GpuUsage {
    sm: u32,
    mem: u32,
}

fn gpu_usage() -> Receiver<GpuUsage> {
    let (tx, rx) = crossbeam::channel::unbounded();
    use nvml_wrapper::{Nvml, error::NvmlError};

    thread::spawn(move || {
        let Ok(nvml) = Nvml::init() else {
            println!("Couldn't load nvidia driver");
            return;
        };
        let device = nvml.device_by_index(0).unwrap();
        let delay = Duration::from_secs(1);

        let mut timestamp = None;

        let _ = tx.try_send(GpuUsage { sm: 0, mem: 0 });

        loop {
            let processes = match device.process_utilization_stats(timestamp) {
                Ok(processes) => processes,
                Err(NvmlError::NotFound) => {
                    // No process using the GPU found
                    continue;
                }
                Err(_) => {
                    println!("Couldn't get process utilization stats");
                    break;
                }
            };

            let process = &processes[0];
            timestamp = Some(process.timestamp);

            if tx
                .send(GpuUsage {
                    sm: process.sm_util,
                    mem: process.mem_util,
                })
                .is_err()
            {
                break;
            }
            std::thread::sleep(delay);
        }
        let _ = tx.try_send(GpuUsage { sm: 0, mem: 0 });
    });

    rx
}
