use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use xshell::{Shell, cmd};

use crate::Metrics;

#[derive(Debug)]
pub struct WasmBinarySize {
    pub example_name: String,
}

impl WasmBinarySize {
    pub fn on(example_name: String) -> Self {
        Self {
            example_name: if example_name.is_empty() {
                "breakout".to_string()
            } else {
                example_name
            },
        }
    }
}

impl Metrics for WasmBinarySize {
    fn prepare(&self) {
        let example = &self.example_name;
        let sh = Shell::new().unwrap();
        cmd!(
            sh,
            "cargo run -p build-wasm-example -- --optimize-size {example}"
        )
        .run()
        .unwrap();
    }

    fn artifacts(&self) -> HashMap<String, PathBuf> {
        std::fs::File::create("done").unwrap();
        HashMap::from([(
            "wasm32-unknown-unknown-size.stats".to_string(),
            Path::new("done").to_path_buf(),
        )])
    }

    fn collect(&self) -> HashMap<String, u64> {
        let wasm_file = Path::new("examples/wasm/target/wasm_example_bg.wasm");
        let size = wasm_file.metadata().unwrap().len();
        let optimized_wasm_file = Path::new("examples/wasm/target/wasm_example_bg.wasm.optimized");
        let optimized_size = optimized_wasm_file.metadata().unwrap().len();
        HashMap::from([
            ("wasm32-unknown-unknown.size".to_string(), size),
            (
                "wasm32-unknown-unknown.optimized.size".to_string(),
                optimized_size,
            ),
        ])
    }
}
