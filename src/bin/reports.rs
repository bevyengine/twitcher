use std::{collections::HashSet, fs::File, io::BufReader, path::Path};

use serde::Serialize;
use tera::Tera;
use twitcher::{
    file_safe_metric_name,
    stats::{Stats, find_stats_files},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = std::fs::create_dir("data");

    let stats: Vec<Stats> = find_stats_files(Path::new("results"))
        .iter()
        .map(|path| {
            let file = File::open(path).unwrap();
            let reader = BufReader::new(file);
            serde_json::from_reader(reader).unwrap()
        })
        .collect();

    let compilation_keys: HashSet<_> = stats
        .iter()
        .flat_map(|stat| stat.metrics.keys())
        .filter(|k| (k.contains("compile-time") && k.ends_with("mean")) || k.ends_with("size"))
        .collect();
    compilation_keys.into_iter().for_each(|metric| {
        let values = stats
            .iter()
            // .filter(|stat| {
            //     (chrono::Utc::now()
            //         - chrono::DateTime::from_timestamp_millis(stat.commit_timestamp as i64)
            //             .unwrap())
            //         <= chrono::Duration::days(30)
            // })
            .flat_map(|stat| {
                stat.metrics.get(metric).map(|value| DataPoint {
                    timestamp: stat.commit_timestamp,
                    commit: stat.commit.clone(),
                    value: *value,
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_writer(
            std::fs::File::create(format!("data/{}.json", metric)).unwrap(),
            &values,
        )
        .unwrap();
    });

    let keys: HashSet<_> = stats
        .iter()
        .flat_map(|stat| stat.metrics.keys())
        .filter(|m| m.starts_with("crate-compile-time") && m.contains(".mean"))
        .map(|m| file_safe_metric_name(m))
        .collect();
    let mut metrics: Vec<_> = keys.iter().collect();
    metrics.sort();

    let tera = Tera::new("templates/*").unwrap();
    let mut context = tera::Context::new();

    context.insert("cratecompilationtimes", &metrics);

    let rendered = tera.render("compile-time.html", &context).unwrap();
    std::fs::write("./compile-time.html", &rendered).unwrap();

    Ok(())
}

#[derive(Serialize)]
struct DataPoint {
    timestamp: u128,
    commit: String,
    value: u64,
}
