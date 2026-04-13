pub(crate) struct MangohudSample {
    pub frame_time: f32,
    pub cpu: f32,
    pub gpu: f32,
    pub vram: f32,
    pub ram: f32,
}

pub mod benchmarks;
pub mod binary_size;
pub mod compile_time;
pub mod crate_compile_time;
pub mod large_scenes;
pub mod llvm_lines;
pub mod stress_tests;
pub mod wasm_binary_size;
