use crate::models::{Annotation, DocumentId, OpenResult, RenderResult, SearchResultItem, PdfResult, FormField};
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
    ExportImages(String, Vec<i32>, f32, String, oneshot::Sender<PdfResult<Vec<String>>>),
    ExportPdf(DocumentId, String, Vec<Annotation>, oneshot::Sender<PdfResult<String>>),
    // New features
    Merge(Vec<String>, String, oneshot::Sender<PdfResult<String>>),
    Split(String, Vec<usize>, String, oneshot::Sender<PdfResult<Vec<String>>>),
    GetFormFields(String, oneshot::Sender<PdfResult<Vec<FormField>>>),
    FillForm(String, Vec<FormField>, String, oneshot::Sender<PdfResult<String>>),
}
