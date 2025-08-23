use std::{collections::HashSet, fs::File, io::BufReader, path::Path};

use twitcher::stats::{Stats, find_stats_files};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("drop table if exists metrics;");
    println!("create table metrics (timestamp timestamp, commit text, name text, value bigint);");

    let stats: Vec<Stats> = find_stats_files(Path::new("results"))
        .iter()
        .map(|path| {
            let file = File::open(path).unwrap();
            let reader = BufReader::new(file);
            serde_json::from_reader(reader).unwrap()
        })
        .collect();

    let keys: HashSet<_> = stats.iter().flat_map(|stat| stat.metrics.keys()).collect();
    let values = keys
        .into_iter()
        .flat_map(|metric| {
            stats
                .iter()
                .flat_map(|stat| {
                    stat.metrics.get(metric).map(|value| {
                        (
                            chrono::DateTime::from_timestamp_millis(stat.commit_timestamp as i64)
                                .unwrap(),
                            stat.commit.clone(),
                            *value,
                        )
                    })
                })
                .map(|(ts, commit, value)| {
                    format!("('{}', '{}', '{}', {})", ts, commit, metric.clone(), value)
                })
        })
        .collect::<Vec<_>>();
    values
        .chunks(1000)
        .for_each(|chunk| println!("insert into metrics values {};", chunk.join(",")));

    Ok(())
}
