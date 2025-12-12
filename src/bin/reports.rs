use std::{collections::HashSet, fs::File, io::BufReader, path::Path};

use chrono::{Days, Months};
use serde::Serialize;
use tera::Tera;
use twitcher::{
    file_safe_metric_name,
    stats::{Stats, find_stats_files},
};

const DATE_LIMIT: chrono::Duration = chrono::Duration::weeks(26);

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

    let crate_names = setup_compile_stats(&stats, &cache_id);
    let stress_tests = setup_stress_tests(&stats, &cache_id);
    let benchmarks = setup_benchmarks(&stats, &cache_id);

    let tera = Tera::new("templates/*").unwrap();
    let mut context = tera::Context::new();

    context.insert("crate_names", &crate_names);
    context.insert("stress_tests", &stress_tests);
    context.insert("benchmarks", &benchmarks);
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

    let rendered = tera.render("stress-tests.html", &context).unwrap();
    std::fs::write("./stress-tests.html", &rendered).unwrap();

    let rendered = tera.render("benchmarks.html", &context).unwrap();
    std::fs::write("./benchmarks.html", &rendered).unwrap();

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

fn setup_stress_tests(stats: &[Stats], cache_id: &str) -> Vec<String> {
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

    stress_tests.iter().for_each(|stress_test| {
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
                                .unwrap() as f64)) as u64,
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

        serde_json::to_writer(
            std::fs::File::create(format!(
                "data/{}_{}{cache_id}.json",
                stress_test.0, stress_test.1
            ))
            .unwrap(),
            &values,
        )
        .unwrap();
    });

    stress_tests
        .into_iter()
        .map(|(name, params)| format!("{name}_{params}"))
        .collect()
}

fn setup_benchmarks(stats: &[Stats], cache_id: &str) -> Vec<String> {
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

    benchmarks.iter().for_each(|(benchmark, safe_name)| {
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

        serde_json::to_writer(
            std::fs::File::create(format!("data/{safe_name}{cache_id}.json")).unwrap(),
            &values,
        )
        .unwrap();
    });

    benchmarks
        .into_iter()
        .map(|(_, safe_name)| safe_name)
        .collect()
}
