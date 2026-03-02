use crate::models::{Annotation, DocumentId};
use crate::pdf_engine::{RenderFilter, RenderQuality};
use std::sync::Arc;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum PdfCommand {
    Open(
        String,
        oneshot::Sender<
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
        RenderQuality,
        oneshot::Sender<Result<(usize, u32, u32, Arc<Vec<u8>>), String>>,
    ),
    RenderThumbnail(
        DocumentId,
        i32,
        f32,
        oneshot::Sender<Result<(usize, u32, u32, Arc<Vec<u8>>), String>>,
    ),
    ExtractText(DocumentId, i32, oneshot::Sender<Result<String, String>>),
    ExportImage(
        DocumentId,
        i32,
        f32,
        String,
        oneshot::Sender<Result<(), String>>,
    ),
    ExportImages(
        DocumentId,
        Vec<i32>,
        f32,
        String,
        oneshot::Sender<Result<Vec<String>, String>>,
    ),
    ExportPdf(
        DocumentId,
        String,
        Vec<Annotation>,
        oneshot::Sender<Result<String, String>>,
    ),
    LoadAnnotations(
        DocumentId,
        String,
        oneshot::Sender<Result<Vec<Annotation>, String>>,
    ),
    Search(
        DocumentId,
        String,
        std::sync::mpsc::Sender<Result<Vec<(usize, String, f32, f32, f32, f32)>, String>>,
    ),
    Close(DocumentId),
}
