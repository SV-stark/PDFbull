#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod commands;
pub mod engine;
pub mod form_scripting;
pub mod image_optimizer;
pub mod logging;
pub mod models;
pub mod ocr;
pub mod organize;
pub mod pdf_engine;
pub mod platform;
pub mod read_mode;
pub mod storage;
pub mod tiled_render;
pub mod ui_gpui;
