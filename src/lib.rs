#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(
    clippy::struct_excessive_bools,
    clippy::too_many_lines,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::elidable_lifetime_names,
    clippy::option_if_let_else,
    clippy::map_unwrap_or,
    clippy::match_wildcard_for_single_variants,
    clippy::unused_self,
    clippy::manual_string_new,
    clippy::ignored_unit_patterns,
    clippy::branches_sharing_code,
    clippy::implicit_clone,
    clippy::default_trait_access
)]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod app;
pub mod commands;
pub mod engine;
pub mod message;
pub mod models;
pub mod pdf_engine;
pub mod storage;
pub mod ui;
pub mod ui_document;
pub mod ui_keyboard_help;
pub mod ui_metadata;
pub mod ui_settings;
pub mod ui_welcome;
pub mod update;
pub mod platform;
