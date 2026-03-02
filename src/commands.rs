use crate::models::{Annotation, DocumentId, OpenResult, RenderResult, SearchResultItem};
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum PdfCommand {
    Open(String, oneshot::Sender<Result<OpenResult, String>>),
    Render(
        DocumentId,
        i32,
        crate::pdf_engine::RenderOptions,
        oneshot::Sender<Result<RenderResult, String>>,
    ),
    RenderThumbnail(
        DocumentId,
        i32,
        f32,
        oneshot::Sender<Result<RenderResult, String>>,
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
        std::sync::mpsc::Sender<Result<Vec<SearchResultItem>, String>>,
    ),
    Close(DocumentId),
}
