use std::{collections::HashSet, fs::File, io::BufReader, path::Path};

use chrono::{Days, Months};
use serde::Serialize;
use tera::Tera;
use twitcher::stats::{Stats, find_stats_files};

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

    let mut min_timestamp = u128::MAX;
    let mut max_timestamp = 0;
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
                    <= chrono::Duration::weeks(6 * 4)
            })
            .flat_map(|stat| {
                if stat.commit_timestamp < min_timestamp {
                    min_timestamp = stat.commit_timestamp;
                }
                if stat.commit_timestamp > max_timestamp {
                    max_timestamp = stat.commit_timestamp;
                }
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

    let keys: HashSet<_> = stats
        .iter()
        .flat_map(|stat| stat.metrics.keys())
        .filter(|m| m.starts_with("crate-compile-time") && m.ends_with(".mean"))
        .map(|m| m.split('.').nth(1).unwrap())
        .collect();
    let mut crate_names: Vec<_> = keys.iter().collect();
    crate_names.sort();

    let tera = Tera::new("templates/*").unwrap();
    let mut context = tera::Context::new();

    context.insert("crate_names", &crate_names);
    context.insert("start", &min_timestamp);
    context.insert("end", &max_timestamp);
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

    Ok(())
}

#[derive(Serialize)]
struct DataPoint {
    timestamp: u128,
    commit: String,
    value: u64,
}
