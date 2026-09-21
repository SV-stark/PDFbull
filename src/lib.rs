#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod commands;
pub mod engine;
pub mod logging;
pub mod models;
pub mod ocr;
pub mod pdf_engine;
pub mod platform;
pub mod storage;
pub mod ui_gpui;
