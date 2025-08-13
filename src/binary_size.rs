use std::{collections::HashMap, path::Path};

use xshell::{Shell, cmd};

use crate::Metrics;

#[derive(Debug)]
pub struct BinarySize {
    pub example_name: String,
}

impl BinarySize {
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

impl Metrics for BinarySize {
    fn prepare(&self) {
        let example = &self.example_name;
        let sh = Shell::new().unwrap();
        cmd!(sh, "cargo build --release --example {example}")
            .run()
            .unwrap();
    }

    fn collect(&self) -> HashMap<String, u64> {
        let target_dir = Path::new("target/release/examples");
        let file_path = target_dir.join(&self.example_name);
        let size = file_path.metadata().unwrap().len();
        HashMap::from([(
            format!(
                "native-{}-{}.size",
                std::env::consts::FAMILY,
                std::env::consts::ARCH
            ),
            size,
        )])
    }
}
