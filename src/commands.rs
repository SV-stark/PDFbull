use crate::models::{Annotation, DocumentId, OpenResult, RenderResult, SearchResultItem, PdfResult};
use crate::pdf_engine::{RenderOptions};
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum PdfCommand {
    Open(String, DocumentId, oneshot::Sender<PdfResult<OpenResult>>),
    Render(String, usize, RenderOptions, oneshot::Sender<PdfResult<RenderResult>>),
    Close(DocumentId),
    ExtractText(String, i32, oneshot::Sender<PdfResult<String>>),
    Search(String, String, oneshot::Sender<PdfResult<Vec<SearchResultItem>>>),
    SaveAnnotations(String, Vec<Annotation>, oneshot::Sender<PdfResult<String>>),
    LoadAnnotations(DocumentId, String, oneshot::Sender<PdfResult<Vec<Annotation>>>),
    ExportImage(String, i32, f32, String, oneshot::Sender<PdfResult<()>>),
}
