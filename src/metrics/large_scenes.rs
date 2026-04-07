use std::{
    collections::HashMap,
    io::{BufRead, Write},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use xshell::{Shell, cmd};

use crate::Metrics;

#[derive(Debug)]
pub struct LargeScene {
    pub scene: String,
    pub parameters: Vec<(String, Option<String>)>,
    pub nb_frames: u32,
    pub features: Vec<String>,
}

impl LargeScene {
    pub fn on(scene: String, parameters: Vec<(String, Option<String>)>, nb_frames: u32) -> Self {
        Self {
            scene,
            parameters,
            nb_frames,
            features: vec![],
        }
    }

    pub fn with_features(mut self, features: Vec<&str>) -> Self {
        self.features = features.into_iter().map(|f| f.to_string()).collect();
        self
    }
}

impl Metrics for LargeScene {
    fn prepare(&self) -> bool {
        let scene = self.scene.clone();

        if fs_extra::dir::copy(
            format!("/assets/{scene}"),
            format!("examples/large_scenes/{scene}/assets"),
            &fs_extra::dir::CopyOptions::new()
                .copy_inside(true)
                .skip_exist(true),
        )
        .is_err()
        {
            return false;
        }

        let sh = Shell::new().unwrap();
        if self
            .features
            .contains(&"mipmap_generator/compress".to_string())
        {
            sh.set_var("RUSTFLAGS", "-C linker=clang");
        }
        let mut features = self.features.clone();
        features.push("bevy/bevy_ci_testing".to_string());
        let features = features
            .into_iter()
            .flat_map(|f| ["--features".to_string(), f]);

        cmd!(sh, "cargo build --release {features...} --package {scene}")
            .run()
            .is_ok()
    }

    fn artifacts(&self) -> HashMap<String, PathBuf> {
        std::fs::File::create("done").unwrap();
        HashMap::from([(
            format!(
                "large-scene-fps.{}.{}",
                self.scene,
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
            "large-scene-fps.{}.{}",
            self.scene,
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
        if self
            .features
            .contains(&"mipmap_generator/compress".to_string())
        {
            sh.set_var("RUSTFLAGS", "-C linker=clang");
        }

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
        let scene = self.scene.clone();
        let mut features = self.features.clone();
        features.push("bevy/bevy_ci_testing".to_string());
        let features = features
            .into_iter()
            .flat_map(|f| ["--features".to_string(), f]);

        let _ = cmd!(sh, "sudo systemctl start lightdm").run();
        thread::sleep(Duration::from_secs(10));

        let _mangohud_guard = sh.push_env(
            "MANGOHUD_CONFIG",
            format!(
                "output_folder={},autostart_log=10",
                std::env::current_dir().unwrap().display()
            ),
        );
        let _display_guard = sh.push_env("DISPLAY", ":0");

        let cmd = cmd!(
            sh,
            "mangohud cargo run --release {features...} --package {scene} -- {parameters...}"
        );
        let mut results = HashMap::new();

        let start = Instant::now();
        let cmd_result = cmd.run();

        let _ = cmd!(sh, "sudo systemctl stop lightdm").run();
        thread::sleep(Duration::from_secs(5));

        if cmd_result.is_err() {
            // ignore failure due to a missing scene
            return results;
        };
        let elapsed = start.elapsed();

        results.insert(format!("{key}.duration"), elapsed.as_millis() as u64);
        results.insert(format!("{key}.frames"), self.nb_frames as u64);

        if let Some(last_modified_file) = std::fs::read_dir(".")
            .expect("Couldn't access local directory")
            .flatten()
            .filter(|f| {
                f.metadata().unwrap().is_file()
                    && f.file_name().into_string().unwrap().ends_with(".csv")
            })
            .max_by_key(|x| x.metadata().unwrap().modified().unwrap())
        {
            let csv_file = std::fs::File::open(last_modified_file.path()).unwrap();
            // Skip first two lines as they're info about system
            let mut reader = std::io::BufReader::new(csv_file);
            let mut tmp = String::new();
            let _ = reader.read_line(&mut tmp);
            let _ = reader.read_line(&mut tmp);
            let mut rdr = csv::ReaderBuilder::new().from_reader(reader);
            let (frame_times, cpu, gpu, vram, ram): (
                Vec<f32>,
                Vec<f32>,
                Vec<f32>,
                Vec<f32>,
                Vec<f32>,
            ) = rdr
                .records()
                .flatten()
                .map(|record| {
                    (
                        record.get(1).unwrap().parse::<f32>().unwrap_or_default(),
                        record.get(2).unwrap().parse::<f32>().unwrap_or_default(),
                        record.get(3).unwrap().parse::<f32>().unwrap_or_default(),
                        record.get(8).unwrap().parse::<f32>().unwrap_or_default(),
                        record.get(10).unwrap().parse::<f32>().unwrap_or_default(),
                    )
                })
                .collect();

            for (values, name) in [
                (frame_times, "frame_time"),
                (cpu, "cpu_load"),
                (gpu, "gpu_load"),
                (ram, "ram_used"),
                (vram, "vram_used"),
            ] {
                if values.len() > 3 {
                    results.insert(
                        format!("{key}.{name}.mean"),
                        (statistical::mean(&values) * 1000.0) as u64,
                    );
                    results.insert(
                        format!("{key}.{name}.median"),
                        (statistical::median(&values) * 1000.0) as u64,
                    );
                    results.insert(
                        format!("{key}.{name}.min"),
                        values.iter().map(|d| (d * 1000.0) as u64).min().unwrap(),
                    );
                    results.insert(
                        format!("{key}.{name}.max"),
                        values.iter().map(|d| (d * 1000.0) as u64).max().unwrap(),
                    );
                    results.insert(
                        format!("{key}.{name}.std_dev"),
                        (statistical::standard_deviation(&values, None) * 1000.0) as u64,
                    );
                }
            }
        }

        results
    }
}
