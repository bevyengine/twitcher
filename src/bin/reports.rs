use std::{collections::HashSet, fs::File, io::BufReader, path::Path};

use chrono::{Days, Months};
use git2::{Repository, Sort};
use regex::Regex;
use serde::Serialize;
use tera::Tera;
use twitcher::{
    file_safe_metric_name,
    stats::{Stats, find_stats_files},
};

const DATE_LIMIT: chrono::Duration = chrono::Duration::weeks(26);

#[derive(Serialize)]
struct Commit {
    id: String,
    summary: String,
    pr: u32,
    timestamp: i64,
    done: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = std::fs::create_dir("data");

    let args: Vec<String> = std::env::args().collect();
    let cache_id = args
        .get(1)
        .map(|id| format!(".{id}"))
        .unwrap_or("".to_string());

    let stats: Vec<Stats> = find_stats_files(Path::new("results"))
        .iter()
        .map(|path| {
            let file = File::open(path).unwrap();
            let reader = BufReader::new(file);
            serde_json::from_reader(reader).unwrap()
        })
        .collect();

    let repo = match Repository::open("bevy") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open: {}", e),
    };
    let summary_regex = Regex::new("(.*) \\(#([0-9]+)\\)").unwrap();
    let mut revwalk = repo.revwalk().unwrap();
    revwalk.set_sorting(Sort::TIME).unwrap();
    revwalk.push_head().unwrap();
    let commits = revwalk
        .filter_map(|c| repo.find_commit(*c.as_ref().unwrap()).ok())
        .take(5000)
        .map(|commit| {
            let (summary, pr) =
                if let Some(captures) = summary_regex.captures(commit.summary().unwrap()) {
                    (
                        captures.get(1).unwrap().as_str().to_string(),
                        captures.get(2).unwrap().as_str().parse().unwrap(),
                    )
                } else {
                    (commit.summary().unwrap().to_string(), 0)
                };
            let id = commit.id().to_string();
            Commit {
                id,
                timestamp: commit.time().seconds(),
                summary,
                pr,
                done: false,
            }
        })
        .filter(|commit| {
            (chrono::Utc::now()
                - chrono::DateTime::from_timestamp_millis(commit.timestamp * 1000).unwrap())
                <= DATE_LIMIT
        })
        .map(|mut commit| {
            commit.done = stats.iter().any(|s| commit.id == s.commit);
            commit
        })
        .collect::<Vec<_>>();

    let crate_names = setup_compile_stats(&stats, &cache_id);
    let mut stress_tests = setup_stress_tests(&stats, &cache_id);
    let mut benchmarks = setup_benchmarks(&stats, &cache_id);

    let stress_tests_alpha = stress_tests
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    stress_tests.sort_by(|a, b| a.1.total_cmp(&b.1));
    stress_tests.reverse();
    let stress_tests_z = stress_tests
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();

    let benchmarks_alpha = benchmarks
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    benchmarks.sort_by(|a, b| a.1.total_cmp(&b.1));
    benchmarks.reverse();
    let benchmarks_z = benchmarks
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();

    let tera = Tera::new("templates/*").unwrap();
    let mut context = tera::Context::new();

    context.insert("crate_names", &crate_names);
    context.insert("commits", &commits);
    context.insert("stress_tests", &stress_tests_alpha);
    context.insert("benchmarks", &benchmarks_alpha);
    context.insert(
        "start",
        &((chrono::Utc::now() - DATE_LIMIT).timestamp() * 1000),
    );
    context.insert("end", &(chrono::Utc::now().timestamp() * 1000));
    context.insert(
        "threemonthsago",
        &(chrono::Utc::now()
            .checked_sub_months(Months::new(3))
            .unwrap()
            .timestamp()
            * 1000),
    );
    context.insert(
        "onemonthago",
        &(chrono::Utc::now()
            .checked_sub_months(Months::new(1))
            .unwrap()
            .timestamp()
            * 1000),
    );
    context.insert(
        "twoweeksago",
        &(chrono::Utc::now()
            .checked_sub_days(Days::new(14))
            .unwrap()
            .timestamp()
            * 1000),
    );
    context.insert(
        "oneweekago",
        &(chrono::Utc::now()
            .checked_sub_days(Days::new(7))
            .unwrap()
            .timestamp()
            * 1000),
    );
    context.insert("cache_id", &cache_id);

    let rendered = tera.render("compile-stats.html", &context).unwrap();
    std::fs::write("./compile-stats.html", &rendered).unwrap();

    context.insert("stress_tests", &stress_tests_alpha);
    context.insert("benchmarks", &benchmarks_alpha);

    let rendered = tera.render("stress-tests.html", &context).unwrap();
    std::fs::write("./stress-tests_alpha.html", &rendered).unwrap();

    let rendered = tera.render("benchmarks.html", &context).unwrap();
    std::fs::write("./benchmarks_alpha.html", &rendered).unwrap();

    context.insert("stress_tests", &stress_tests_z);
    context.insert("benchmarks", &benchmarks_z);

    let rendered = tera.render("stress-tests.html", &context).unwrap();
    std::fs::write("./stress-tests_z.html", &rendered).unwrap();

    let rendered = tera.render("benchmarks.html", &context).unwrap();
    std::fs::write("./benchmarks_z.html", &rendered).unwrap();

    Ok(())
}

fn setup_compile_stats<'a>(stats: &'a [Stats], cache_id: &str) -> Vec<&'a str> {
    #[derive(Serialize)]
    struct DataPoint {
        timestamp: u128,
        commit: String,
        value: u64,
    }

    let compilation_keys: HashSet<_> = stats
        .iter()
        .flat_map(|stat| stat.metrics.keys())
        .filter(|k| (k.contains("compile-time") && k.ends_with("mean")) || k.ends_with("size"))
        .collect();
    compilation_keys.into_iter().for_each(|metric| {
        let values = stats
            .iter()
            .filter(|stat| {
                (chrono::Utc::now()
                    - chrono::DateTime::from_timestamp_millis(stat.commit_timestamp as i64)
                        .unwrap())
                    <= DATE_LIMIT
            })
            .flat_map(|stat| {
                stat.metrics.get(metric).map(|value| DataPoint {
                    timestamp: stat.commit_timestamp,
                    commit: stat.commit.clone(),
                    value: *value,
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_writer(
            std::fs::File::create(format!("data/{metric}{cache_id}.json")).unwrap(),
            &values,
        )
        .unwrap();
    });

    let mut crate_names = stats
        .iter()
        .flat_map(|stat| stat.metrics.keys())
        .filter(|m| m.starts_with("crate-compile-time") && m.ends_with(".mean"))
        .map(|m| m.split('.').nth(1).unwrap())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    crate_names.sort();
    crate_names
}

fn setup_stress_tests(stats: &[Stats], cache_id: &str) -> Vec<(String, f64)> {
    #[derive(Serialize)]
    struct DataPoint {
        timestamp: u128,
        commit: String,
        frame_time: u64,
        cpu: u64,
        gpu: u64,
    }

    let mut stress_tests = stats
        .iter()
        .flat_map(|stat| stat.metrics.keys())
        .filter(|m| m.starts_with("stress-test-fps") && m.ends_with(".mean"))
        .map(|m| {
            let mut split = m.split('.');
            (split.nth(1).unwrap(), split.next().unwrap())
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    stress_tests.sort();

    let with_z_scores = stress_tests
        .into_iter()
        .flat_map(|stress_test| {
            let values = stats
                .iter()
                .filter(|stat| {
                    (chrono::Utc::now()
                        - chrono::DateTime::from_timestamp_millis(stat.commit_timestamp as i64)
                            .unwrap())
                        <= DATE_LIMIT
                        && chrono::DateTime::from_timestamp_millis(stat.commit_timestamp as i64)
                            .unwrap()
                            > chrono::DateTime::parse_from_rfc3339("2025-08-27T00:00:00Z").unwrap() // Data before this date is not with the same format
                })
                .flat_map(|stat| {
                    stat.metrics
                        .get(&format!(
                            "stress-test-fps.{}.{}.duration",
                            stress_test.0, stress_test.1
                        ))
                        .map(|value| DataPoint {
                            timestamp: stat.commit_timestamp,
                            commit: stat.commit.clone(),
                            frame_time: (1000.0 * (*value as f64)
                                / (stat
                                    .metrics
                                    .get(&format!(
                                        "stress-test-fps.{}.{}.frames",
                                        stress_test.0, stress_test.1
                                    ))
                                    .cloned()
                                    .unwrap() as f64))
                                as u64,
                            cpu: stat
                                .metrics
                                .get(&format!(
                                    "stress-test-fps.{}.{}.cpu_usage.mean",
                                    stress_test.0, stress_test.1
                                ))
                                .cloned()
                                .unwrap_or(0),
                            gpu: stat
                                .metrics
                                .get(&format!(
                                    "stress-test-fps.{}.{}.gpu_usage.mean",
                                    stress_test.0, stress_test.1
                                ))
                                .cloned()
                                .unwrap_or(0),
                        })
                })
                .collect::<Vec<_>>();

            if values.is_empty() {
                return None;
            }

            serde_json::to_writer(
                std::fs::File::create(format!(
                    "data/{}_{}{cache_id}.json",
                    stress_test.0, stress_test.1
                ))
                .unwrap(),
                &values,
            )
            .unwrap();

            let raw_values = values
                .iter()
                .map(|v| v.frame_time as f64)
                .collect::<Vec<_>>();
            let mean = statistical::mean(raw_values.as_slice());
            let standard_deviation = statistical::standard_deviation(raw_values.as_slice(), None);

            let last_week_z_score = values
                .iter()
                .filter(|data| {
                    (chrono::Utc::now()
                        - chrono::DateTime::from_timestamp_millis(data.timestamp as i64).unwrap())
                        <= chrono::Duration::days(7)
                })
                .map(|data| data.frame_time)
                .map(|v| ((v as f64 - mean) / standard_deviation).abs())
                .max_by(|a, b| a.total_cmp(b))
                .unwrap_or_default();
            Some((stress_test.0, stress_test.1, last_week_z_score))
        })
        .collect::<Vec<_>>();

    with_z_scores
        .into_iter()
        .map(|(name, params, z_score)| (format!("{name}_{params}"), z_score))
        .collect()
}

fn setup_benchmarks(stats: &[Stats], cache_id: &str) -> Vec<(String, f64)> {
    #[derive(Serialize)]
    struct DataPoint {
        timestamp: u128,
        commit: String,
        duration: u64,
    }

    let mut benchmarks = stats
        .iter()
        .flat_map(|stat| stat.metrics.keys())
        .filter(|m| m.starts_with("benchmarks.") && m.ends_with(".mean"))
        .map(|m| {
            let mut split = m.split('.');
            split.nth(1).unwrap()
        })
        .map(|benchmark| (benchmark.to_string(), file_safe_metric_name(benchmark)))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    benchmarks.sort();

    let with_z_scores = benchmarks
        .into_iter()
        .flat_map(|(benchmark, safe_name)| {
            let values = stats
                .iter()
                .filter(|stat| {
                    (chrono::Utc::now()
                        - chrono::DateTime::from_timestamp_millis(stat.commit_timestamp as i64)
                            .unwrap())
                        <= DATE_LIMIT
                })
                .flat_map(|stat| {
                    stat.metrics
                        .get(&format!("benchmarks.{benchmark}.mean"))
                        .map(|value| DataPoint {
                            timestamp: stat.commit_timestamp,
                            commit: stat.commit.clone(),
                            duration: *value,
                        })
                })
                .collect::<Vec<_>>();

            if values.is_empty() {
                return None;
            }

            serde_json::to_writer(
                std::fs::File::create(format!("data/{safe_name}{cache_id}.json")).unwrap(),
                &values,
            )
            .unwrap();

            let raw_values = values.iter().map(|v| v.duration as f64).collect::<Vec<_>>();
            let mean = statistical::mean(raw_values.as_slice());
            let standard_deviation = statistical::standard_deviation(raw_values.as_slice(), None);

            let last_week_z_score = values
                .iter()
                .filter(|data| {
                    (chrono::Utc::now()
                        - chrono::DateTime::from_timestamp_millis(data.timestamp as i64).unwrap())
                        <= chrono::Duration::days(7)
                })
                .map(|data| data.duration)
                .map(|v| ((v as f64 - mean) / standard_deviation).abs())
                .max_by(|a, b| a.total_cmp(b))
                .unwrap_or_default();
            Some((benchmark, safe_name, last_week_z_score))
        })
        .collect::<Vec<_>>();

    with_z_scores
        .into_iter()
        .map(|(_, safe_name, z_score)| (safe_name, z_score))
        .collect()
}
