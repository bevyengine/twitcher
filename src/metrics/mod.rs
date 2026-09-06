use std::io::BufRead;

pub(crate) struct MangohudSample {
    pub frame_time: f32,
    pub cpu: f32,
    pub gpu: f32,
    pub vram: f32,
    pub ram: f32,
}

pub(crate) fn parse_mangohud_csv(path: &std::path::Path) -> Vec<MangohudSample> {
    let file = std::fs::File::open(path).unwrap();
    let mut reader = std::io::BufReader::new(file);
    let mut discard = String::new();
    let _ = reader.read_line(&mut discard);
    let _ = reader.read_line(&mut discard);

    let mut rdr = csv::ReaderBuilder::new().from_reader(reader);
    let headers = match rdr.headers() {
        Ok(h) => h.clone(),
        Err(e) => {
            eprintln!("mangohud csv: unreadable header row: {e}");
            return Vec::new();
        }
    };
    let column = |name: &str| -> Option<usize> {
        let found = headers.iter().position(|h| h.trim() == name);
        if found.is_none() {
            eprintln!("mangohud csv: no `{name}` column; header is {headers:?}");
        }
        found
    };

    let (frame_time, cpu, gpu, vram, ram) = (
        column("frametime"),
        column("cpu_load"),
        column("gpu_load"),
        column("gpu_vram_used"),
        column("ram_used"),
    );

    let field = |record: &csv::StringRecord, index: Option<usize>| -> f32 {
        index
            .and_then(|i| record.get(i))
            .and_then(|v| v.trim().parse::<f32>().ok())
            .unwrap_or_default()
    };

    rdr.records()
        .flatten()
        .map(|record| MangohudSample {
            frame_time: field(&record, frame_time),
            cpu: field(&record, cpu),
            gpu: field(&record, gpu),
            vram: field(&record, vram),
            ram: field(&record, ram),
        })
        .collect()
}

pub mod benchmarks;
pub mod binary_size;
pub mod compile_time;
pub mod crate_compile_time;
pub mod large_scenes;
pub mod llvm_lines;
pub mod stress_tests;
pub mod wasm_binary_size;
