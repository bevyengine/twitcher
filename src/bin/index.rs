use std::{collections::HashSet, fs::File, io::BufReader, path::Path};

use tera::Tera;
use twitcher::{
    file_safe_metric_name,
    stats::{Stats, find_stats_files},
};

fn main() {
    let tera = Tera::new("templates/*").unwrap();
    // Prepare the context with some data
    let mut context = tera::Context::new();

    let stats: Vec<Stats> = find_stats_files(Path::new("results"))
        .iter()
        .map(|path| {
            let file = File::open(path).unwrap();
            let reader = BufReader::new(file);
            serde_json::from_reader(reader).unwrap()
        })
        .collect();

    let keys: HashSet<_> = stats
        .iter()
        .flat_map(|stat| stat.metrics.keys())
        .map(|m| file_safe_metric_name(m))
        .collect();
    let mut metrics: Vec<_> = keys.iter().collect();
    metrics.sort();
    context.insert("metrics", &metrics);

    // Render the template with the given context
    let rendered = tera.render("index.html", &context).unwrap();
    std::fs::write("./index.html", &rendered).unwrap();
}
