#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod app;
pub mod commands;
pub mod engine;
pub mod logging;
pub mod message;
pub mod models;
pub mod ocr;
pub mod pdf_engine;
pub mod platform;
pub mod storage;
pub mod ui;
pub mod ui_cmyk;
pub mod ui_conformance;
pub mod ui_document;
pub mod ui_gpui;
pub mod ui_keyboard_help;
pub mod ui_log_console;
pub mod ui_metadata;
pub mod ui_settings;
pub mod ui_welcome;
pub mod update;
