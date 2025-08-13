use std::{
    collections::HashMap,
    io::{BufRead, Write},
    path::{Path, PathBuf},
};

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
        cmd!(
            sh,
            "cargo build --release --features bevy_ci_testing --example {stress_tests}"
        )
        .run()
        .unwrap();
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
            // .flat_map(|(p, v)| [format!("--{}", p), v.clone()])
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
            "cargo run --release --features bevy_ci_testing --example {stress_tests} -- {parameters...}"
        );
        let output = cmd.output().unwrap();
        let fpss = output
            .stderr
            .lines()
            .map_while(|line| line.ok())
            .filter(|line| line.contains("fps"))
            .map(|line| line.split("fps").nth(1).unwrap().to_string())
            .map(|line| line.split("(").nth(0).unwrap().to_string())
            .map(|line| line.split(":").nth(1).unwrap().to_string())
            .map(|line| line.trim().to_string())
            .map(|line| line.parse::<f32>().unwrap())
            .collect::<Vec<_>>();

        let mut results = HashMap::new();
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

        // .collect()
        results
    }
}
