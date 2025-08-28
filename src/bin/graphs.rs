use std::{
    collections::HashSet,
    fs::{self, File},
    io::BufReader,
    path::Path,
};

use plotters::prelude::*;
use twitcher::{
    file_safe_metric_name,
    stats::{Stats, find_stats_files},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("graphs").unwrap();
    let stats: Vec<Stats> = find_stats_files(Path::new("results"))
        .iter()
        .map(|path| {
            let file = File::open(path).unwrap();
            let reader = BufReader::new(file);
            serde_json::from_reader(reader).unwrap()
        })
        .collect();

    let keys: HashSet<_> = stats.iter().flat_map(|stat| stat.metrics.keys()).collect();
    for metric in keys {
        println!("Metric: {metric}");
        let mut data = stats
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
            .collect::<Vec<_>>();

        data.sort_by_key(|d| d.0);
        let min = data.iter().min_by_key(|d| d.2).unwrap().2;
        let max = data.iter().max_by_key(|d| d.2).unwrap().2;

        let out = format!("graphs/{}.svg", file_safe_metric_name(metric));

        let root = SVGBackend::new(&out, (1536, 512)).into_drawing_area();
        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .x_label_area_size(50)
            .y_label_area_size(100)
            .caption(metric, ("sans-serif", 50.0).into_font())
            .build_cartesian_2d(data.first().unwrap().0..data.last().unwrap().0, min..max)?;

        chart.configure_mesh().light_line_style(WHITE).draw()?;

        chart
            .draw_series(LineSeries::new(data.iter().map(|x| (x.0, x.2)), BLUE))
            .unwrap();

        root.present().expect("Unable to write result to file");
        println!("Result has been saved to {out}");
    }

    Ok(())
}
