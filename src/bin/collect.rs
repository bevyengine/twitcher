use std::{
    collections::HashMap,
    fs::File,
    io::BufWriter,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use clap::{Parser, Subcommand};
use strum::{EnumIter, IntoEnumIterator};
use twitcher::{
    Metrics, benchmarks, binary_size, compile_time, crate_compile_time,
    stats::{Host, Rust, Stats},
    stress_tests, wasm_binary_size,
};
use xshell::{Shell, cmd};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Commit to run on. If ommitted, run on the already checked out commit
    #[arg(short, long)]
    commit: Option<String>,

    /// Merge results instead of overwrite
    #[arg(short, long)]
    merge_results: bool,

    /// Target folder for results
    #[arg(short, long, default_value = "results")]
    out: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug, EnumIter)]
enum Commands {
    BinarySize {
        #[arg(short, long, default_value = "breakout")]
        example: String,
    },
    WasmBinarySize {
        #[arg(short, long, default_value = "breakout")]
        example: String,
    },
    CompileTime {
        #[arg(short, long, default_value = "breakout")]
        example: String,
    },
    CrateCompileTime,
    StressTest {
        #[arg(short, long)]
        stress_test: String,
        #[arg(short, long)]
        parameters: String,
        #[arg(short, long)]
        nb_frames: u32,
    },
    Benchmarks,
    All,
}

impl Commands {
    #[allow(clippy::wrong_self_convention)]
    fn to_metrics(self, recur: bool) -> Vec<Box<dyn Metrics>> {
        match self {
            Commands::BinarySize { example } => {
                vec![Box::new(binary_size::BinarySize::on(example))]
            }
            Commands::WasmBinarySize { example } => {
                vec![Box::new(wasm_binary_size::WasmBinarySize::on(example))]
            }
            Commands::CompileTime { example } => {
                vec![
                    Box::new(compile_time::CompileTime::on(example.clone(), 8)),
                    Box::new(compile_time::CompileTime::on(example, 16)),
                ]
            }
            Commands::CrateCompileTime => {
                vec![Box::new(crate_compile_time::CrateCompileTime::on(16))]
            }
            Commands::StressTest {
                stress_test,
                parameters,
                nb_frames,
            } => {
                if stress_test.is_empty() {
                    vec![
                        Box::new(stress_tests::StressTest::on(
                            "bevymark".to_string(),
                            vec![
                                ("waves".to_string(), Some("60".to_string())),
                                ("per-wave".to_string(), Some("500".to_string())),
                                ("benchmark".to_string(), None),
                                ("mode".to_string(), Some("sprite".to_string())),
                            ],
                            10000,
                        )),
                        Box::new(stress_tests::StressTest::on(
                            "bevymark".to_string(),
                            vec![
                                ("waves".to_string(), Some("60".to_string())),
                                ("per-wave".to_string(), Some("500".to_string())),
                                ("benchmark".to_string(), None),
                                ("mode".to_string(), Some("mesh2d".to_string())),
                            ],
                            5000,
                        )),
                        Box::new(stress_tests::StressTest::on(
                            "many_animated_sprites".to_string(),
                            vec![],
                            30000,
                        )),
                        Box::new(stress_tests::StressTest::on(
                            "many_buttons".to_string(),
                            vec![],
                            5000,
                        )),
                        Box::new(stress_tests::StressTest::on(
                            "many_cubes".to_string(),
                            vec![("benchmark".to_string(), None)],
                            15000,
                        )),
                        Box::new(stress_tests::StressTest::on(
                            "many_foxes".to_string(),
                            vec![],
                            15000,
                        )),
                        Box::new(stress_tests::StressTest::on(
                            "many_gizmos".to_string(),
                            vec![],
                            5000,
                        )),
                        Box::new(stress_tests::StressTest::on(
                            "many_glyphs".to_string(),
                            vec![],
                            10000,
                        )),
                        Box::new(stress_tests::StressTest::on(
                            "many_gradients".to_string(),
                            vec![],
                            20000,
                        )),
                        Box::new(stress_tests::StressTest::on(
                            "many_lights".to_string(),
                            vec![],
                            5000,
                        )),
                        Box::new(stress_tests::StressTest::on(
                            "many_materials".to_string(),
                            vec![],
                            20000,
                        )),
                        Box::new(stress_tests::StressTest::on(
                            "many_sprites".to_string(),
                            vec![],
                            30000,
                        )),
                        Box::new(stress_tests::StressTest::on(
                            "many_text2d".to_string(),
                            vec![],
                            20000,
                        )),
                    ]
                } else {
                    let parameters: Vec<String> =
                        parameters.split(' ').map(|s| s.to_string()).collect();
                    let parameters = parameters
                        .chunks(2)
                        .filter(|p| p.len() == 2)
                        .map(|p| {
                            (
                                p[0].clone(),
                                if p[1].is_empty() {
                                    None
                                } else {
                                    Some(p[1].clone())
                                },
                            )
                        })
                        .collect();

                    vec![Box::new(stress_tests::StressTest::on(
                        stress_test,
                        parameters,
                        nb_frames,
                    ))]
                }
            }
            Commands::Benchmarks => {
                vec![Box::new(benchmarks::Benchmarks)]
            }
            Commands::All => {
                if recur {
                    Commands::iter()
                        .flat_map(|command| command.to_metrics(false))
                        .collect()
                } else {
                    vec![]
                }
            }
        }
    }
}

fn main() {
    let cli = Cli::parse();

    let commit = if let Some(commit) = cli.commit {
        let sh = Shell::new().unwrap();
        cmd!(sh, "git checkout {commit}").run().unwrap();
        commit
    } else {
        let sh = Shell::new().unwrap();
        let out = cmd!(sh, "git rev-parse HEAD").output().unwrap();
        let mut output = out.stdout;
        output.pop();
        String::from_utf8(output).unwrap()
    };
    let commit_timestamp = {
        let sh = Shell::new().unwrap();
        let out = cmd!(sh, "git show --no-patch --format=%ct HEAD")
            .output()
            .unwrap();
        let mut output = out.stdout;
        output.pop();
        String::from_utf8(output).unwrap().parse::<u128>().unwrap() * 1000
    };

    let metrics_to_run = cli.command.to_metrics(true);

    let output_prefix = Path::new(&cli.out)
        .join(commit.chars().next().unwrap().to_string())
        .join(commit.chars().nth(1).unwrap().to_string())
        .join(&commit);

    let mut metrics: HashMap<String, u64> = metrics_to_run
        .iter()
        .filter(|m| m.prepare())
        .flat_map(|m| {
            for (save_as, file_name) in m.artifacts() {
                let target_folder = output_prefix.join(save_as);
                std::fs::create_dir_all(&target_folder).unwrap();
                std::fs::copy(file_name.clone(), target_folder.join(file_name)).unwrap();
            }
            let metrics = m.collect();
            std::thread::sleep(Duration::from_secs(5));
            metrics
        })
        .collect();

    let sh = Shell::new().unwrap();
    let stable = String::from_utf8(cmd!(sh, "rustc --version").output().unwrap().stdout)
        .unwrap()
        .trim()
        .to_string();
    let nightly = String::from_utf8(
        cmd!(sh, "rustc +nightly --version")
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    let hostname = String::from_utf8(cmd!(sh, "hostname").output().unwrap().stdout)
        .unwrap()
        .trim()
        .to_string();
    let os_version = String::from_utf8(cmd!(sh, "uname -r").output().unwrap().stdout)
        .unwrap()
        .trim()
        .to_string();

    if cli.merge_results
        && let Ok(file) = File::open(output_prefix.join("stats.json"))
    {
        let previous_stats: Result<Stats, _> = serde_json::from_reader(file);
        if let Ok(mut previous_stats) = previous_stats {
            for (key, value) in metrics {
                previous_stats.metrics.insert(key, value);
            }
            metrics = previous_stats.metrics;
        }
    }

    let file = File::create(output_prefix.join("stats.json")).unwrap();
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(
        &mut writer,
        &Stats {
            metrics,
            commit,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            commit_timestamp,
            rust: Rust { stable, nightly },
            host: Host {
                hostname,
                os_version,
            },
        },
    )
    .unwrap();
}
