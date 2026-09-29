use crate::models::{
    Annotation, DetectedTable, DocumentId, DocumentMeta, FormField, OpenResult, PdfResult,
    RenderResult, SearchResultItem, TextItem,
};
use crate::pdf_engine::RenderOptions;
use tokio::sync::oneshot;

/// Engine work queue. `Open` carries the document password, so `Debug` is
/// implemented by hand (see below) rather than derived — a derived `Debug`
/// would print the secret into any `tracing::debug!("{:?}", cmd)` we ever add.
pub enum PdfCommand {
    Open(
        String,
        Option<String>,
        DocumentId,
        oneshot::Sender<PdfResult<OpenResult>>,
    ),
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
    ExtractText(DocumentId, usize, oneshot::Sender<PdfResult<String>>),
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
    ExportImage(DocumentId, usize, f32, oneshot::Sender<PdfResult<Vec<u8>>>),
    ExportImages(
        DocumentId,
        Vec<usize>,
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
    AddHeaderFooter(
        String,
        String,
        String,
        String,
        oneshot::Sender<PdfResult<String>>,
    ),
    Optimize(String, String, oneshot::Sender<PdfResult<String>>),
    ReorderPages(
        String,
        Vec<usize>,
        String,
        oneshot::Sender<PdfResult<String>>,
    ),
    ToggleLayer(DocumentId, (u32, u16), bool),
    GetAttachmentBytes(DocumentId, (u32, u16), oneshot::Sender<PdfResult<Vec<u8>>>),
    DetectTables(
        DocumentId,
        usize,
        oneshot::Sender<PdfResult<Vec<DetectedTable>>>,
    ),
    EncryptPdf(
        String,
        String,
        String,
        String,
        String,
        oneshot::Sender<PdfResult<String>>,
    ),
    LinearizePdf(String, String, oneshot::Sender<PdfResult<String>>),
    ValidateConformance(
        String,
        String,
        oneshot::Sender<PdfResult<crate::models::ConformanceReport>>,
    ),
    VerifySignatureTrust(
        DocumentId,
        Vec<u8>,
        oneshot::Sender<PdfResult<Vec<crate::models::SigTrustResult>>>,
    ),
    ConvertPdf(
        DocumentId,
        String,
        String,
        oneshot::Sender<PdfResult<String>>,
    ),
    /// Feature 1: Create a blank PDF from scratch using `DocumentBuilder`.
    /// args: `output_path`, tx
    CreateBlankDocument(String, oneshot::Sender<PdfResult<String>>),
    /// Feature 2: Apply a cryptographic PKCS#12 digital signature to a document.
    /// args: `doc_id`, `cert_path` (.p12/.pfx), `output_path`, tx
    SignDocumentWithCert(
        DocumentId,
        String,
        String,
        oneshot::Sender<PdfResult<String>>,
    ),
    /// Feature 3: Overlay a rubber-stamp annotation onto a page.
    /// args: `doc_id`, `page_num` (0-based), `stamp_label`, `output_path`, tx
    ApplyStamp(
        DocumentId,
        usize,
        String,
        String,
        oneshot::Sender<PdfResult<String>>,
    ),
    /// Feature 6: OCR text recognition for scanned pages.
    /// args: `doc_id`, `page_num` (0-based), `script`, tx
    OcrPage(
        DocumentId,
        usize,
        crate::ocr::OcrScript,
        oneshot::Sender<PdfResult<crate::ocr::OcrPageResult>>,
    ),
}

impl std::fmt::Debug for PdfCommand {
    /// Renders the variant name only, and never the `Open` password.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Open(..) => "Open",
            Self::Render(..) => "Render",
            Self::RenderThumbnail(..) => "RenderThumbnail",
            Self::Close(_) => "Close",
            Self::ExtractText(..) => "ExtractText",
            Self::GetTextItems(..) => "GetTextItems",
            Self::LoadDocumentMeta(..) => "LoadDocumentMeta",
            Self::Search(..) => "Search",
            Self::SaveAnnotations(..) => "SaveAnnotations",
            Self::LoadAnnotations(..) => "LoadAnnotations",
            Self::ExportImage(..) => "ExportImage",
            Self::ExportImages(..) => "ExportImages",
            Self::ExportPdf(..) => "ExportPdf",
            Self::Merge(..) => "Merge",
            Self::Split(..) => "Split",
            Self::GetFormFields(..) => "GetFormFields",
            Self::FillForm(..) => "FillForm",
            Self::PrintPdf(..) => "PrintPdf",
            Self::ListPrinters(_) => "ListPrinters",
            Self::AddWatermark(..) => "AddWatermark",
            Self::AddHeaderFooter(..) => "AddHeaderFooter",
            Self::Optimize(..) => "Optimize",
            Self::ReorderPages(..) => "ReorderPages",
            Self::ToggleLayer(..) => "ToggleLayer",
            Self::GetAttachmentBytes(..) => "GetAttachmentBytes",
            Self::DetectTables(..) => "DetectTables",
            Self::EncryptPdf(..) => "EncryptPdf",
            Self::LinearizePdf(..) => "LinearizePdf",
            Self::ValidateConformance(..) => "ValidateConformance",
            Self::VerifySignatureTrust(..) => "VerifySignatureTrust",
            Self::ConvertPdf(..) => "ConvertPdf",
            Self::CreateBlankDocument(..) => "CreateBlankDocument",
            Self::SignDocumentWithCert(..) => "SignDocumentWithCert",
            Self::ApplyStamp(..) => "ApplyStamp",
            Self::OcrPage(..) => "OcrPage",
        };
        f.write_str(name)
    }
}
