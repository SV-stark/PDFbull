#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod comic;
pub mod commands;
pub mod engine;
pub mod image_optimizer;
pub mod logging;
pub mod models;
pub mod ocr;
pub mod organize;
pub mod pdf_engine;
pub mod platform;
pub mod startup_report;
pub mod storage;
pub mod ui_gpui;
