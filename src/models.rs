use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PdfDocument {
    pub path: String,
    pub name: String,
    pub page_count: i32,
    pub current_page: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Annotation {
    pub page_num: i32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub x1: Option<f32>,
    pub y1: Option<f32>,
    pub x2: Option<f32>,
    pub y2: Option<f32>,
    pub color: String,
    pub tool_type: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageRender {
    pub page_num: i32,
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub page: i32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextRect {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
