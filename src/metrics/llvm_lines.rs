use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use regex::Regex;
use xshell::{Shell, cmd};

use crate::Metrics;

#[derive(Debug)]
pub struct LlvmLines;

impl Metrics for LlvmLines {
    fn prepare(&self) -> bool {
        true
    }

    fn artifacts(&self) -> HashMap<String, PathBuf> {
        std::fs::File::create("done").unwrap();
        HashMap::from([(format!("llvm-lines",), Path::new("done").to_path_buf())])
    }

    fn collect(&self) -> HashMap<String, u64> {
        let sh = Shell::new().unwrap();
        let out = cmd!(sh, "cargo llvm-lines --release --example breakout")
            .read()
            .unwrap();

        //    67 (0.1%, 82.8%)     9 (0.3%, 55.6%)  bevy_ecs::system::commands::Commands::queue
        let re = Regex::new(r"^ +([0-9]+) \([0-9.%, ]+\) +([0-9]+) \([0-9.%, ]+\) +(.*)$").unwrap();

        out.lines()
            .filter_map(|line| re.captures(line))
            .flat_map(|captured| {
                [
                    (
                        format!("llvm-lines.{}.lines", captured.get(3).unwrap().as_str()),
                        captured.get(1).unwrap().as_str().parse::<u64>().unwrap(),
                    ),
                    (
                        format!("llvm-lines.{}.copies", captured.get(3).unwrap().as_str()),
                        captured.get(2).unwrap().as_str().parse::<u64>().unwrap(),
                    ),
                ]
            })
            .collect()
    }
}
