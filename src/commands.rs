use crate::models::{
    Annotation, DocumentId, DocumentMeta, FormField, OpenResult, PdfResult, RenderResult,
    SearchResultItem, TextItem,
};
use crate::pdf_engine::RenderOptions;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum PdfCommand {
    Open(String, DocumentId, oneshot::Sender<PdfResult<OpenResult>>),
    Render(
        DocumentId,
        usize,
        RenderOptions,
        oneshot::Sender<PdfResult<RenderResult>>,
    ),
    RenderThumbnail(
        DocumentId,
        usize,
        f32,
        i32,
        oneshot::Sender<PdfResult<RenderResult>>,
    ),
    Close(DocumentId),
    ExtractText(DocumentId, i32, oneshot::Sender<PdfResult<String>>),
    GetTextItems(DocumentId, usize, oneshot::Sender<PdfResult<Vec<TextItem>>>),
    LoadDocumentMeta(DocumentId, oneshot::Sender<PdfResult<DocumentMeta>>),
    Search(
        DocumentId,
        String,
        oneshot::Sender<PdfResult<Vec<SearchResultItem>>>,
    ),
    SaveAnnotations(
        DocumentId,
        Vec<Annotation>,
        oneshot::Sender<PdfResult<String>>,
    ),
    LoadAnnotations(
        DocumentId,
        String,
        oneshot::Sender<PdfResult<Vec<Annotation>>>,
    ),
    ExportImage(DocumentId, i32, f32, oneshot::Sender<PdfResult<Vec<u8>>>),
    ExportImages(
        DocumentId,
        Vec<i32>,
        f32,
        String,
        oneshot::Sender<PdfResult<Vec<String>>>,
    ),
    ExportPdf(
        DocumentId,
        String,
        Vec<Annotation>,
        oneshot::Sender<PdfResult<String>>,
    ),
    // New features
    Merge(Vec<String>, String, oneshot::Sender<PdfResult<String>>),
    Split(
        String,
        Vec<usize>,
        String,
        oneshot::Sender<PdfResult<Vec<String>>>,
    ),
    GetFormFields(String, oneshot::Sender<PdfResult<Vec<FormField>>>),
    FillForm(
        String,
        Vec<FormField>,
        String,
        oneshot::Sender<PdfResult<String>>,
    ),
    PrintPdf(String, Option<String>, oneshot::Sender<PdfResult<()>>),
    ListPrinters(oneshot::Sender<PdfResult<Vec<String>>>),
    AddWatermark(String, String, String, oneshot::Sender<PdfResult<String>>),
    Optimize(String, String, oneshot::Sender<PdfResult<String>>),
    ReorderPages(
        String,
        Vec<usize>,
        String,
        oneshot::Sender<PdfResult<String>>,
    ),
}
