use crate::pdf_engine::RenderFilter;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum PdfCommand {
    Open(
        String,
        mpsc::Sender<Result<(usize, Vec<f32>, f32, Vec<crate::pdf_engine::Bookmark>), String>>,
    ),
    Render(
        i32,
        f32,
        i32,
        RenderFilter,
        mpsc::Sender<Result<(usize, u32, u32, Arc<Vec<u8>>), String>>,
    ),
    RenderThumbnail(
        i32,
        mpsc::Sender<Result<(usize, u32, u32, Arc<Vec<u8>>), String>>,
    ),
    ExtractText(i32, mpsc::Sender<Result<String, String>>),
    ExportImage(i32, f32, String, mpsc::Sender<Result<(), String>>),
    Search(
        String,
        mpsc::Sender<Result<Vec<(usize, String, f32)>, String>>,
    ),
    Close,
}
