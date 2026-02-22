use crate::models::{Annotation, DocumentId};
use crate::pdf_engine::RenderFilter;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum PdfCommand {
    Open(
        String,
        mpsc::Sender<
            Result<
                (
                    DocumentId,
                    usize,
                    Vec<f32>,
                    f32,
                    Vec<crate::pdf_engine::Bookmark>,
                ),
                String,
            >,
        >,
    ),
    Render(
        DocumentId,
        i32,
        f32,
        i32,
        RenderFilter,
        bool,
        mpsc::Sender<Result<(usize, u32, u32, Arc<Vec<u8>>), String>>,
    ),
    ExtractText(DocumentId, i32, mpsc::Sender<Result<String, String>>),
    ExportImage(
        DocumentId,
        i32,
        f32,
        String,
        mpsc::Sender<Result<(), String>>,
    ),
    ExportImages(
        DocumentId,
        Vec<i32>,
        f32,
        String,
        mpsc::Sender<Result<Vec<String>, String>>,
    ),
    ExportPdf(
        DocumentId,
        String,
        Vec<Annotation>,
        mpsc::Sender<Result<String, String>>,
    ),
    LoadAnnotations(
        DocumentId,
        String,
        mpsc::Sender<Result<Vec<Annotation>, String>>,
    ),
    Search(
        DocumentId,
        String,
        mpsc::Sender<Result<Vec<(usize, String, f32)>, String>>,
    ),
    Close(DocumentId),
}
