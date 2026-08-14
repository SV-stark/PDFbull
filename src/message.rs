use crate::engine::EngineState;
use crate::models::{
    AppSettings, DocumentId, DocumentMeta, OpenResult, PdfResult, RecentFile, RenderResult,
    SearchResultItem, TextItem,
};
use crate::pdf_engine::RenderFilter;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Message {
    ResetZoom,
    OpenSettings,
    CloseSettings,
    SaveSettings(AppSettings),
    ToggleSidebar,
    ToggleFormsSidebar,
    ToggleFullscreen,
    ToggleKeyboardHelp,
    RotateClockwise,
    RotateCounterClockwise,
    AddBookmark,
    RemoveBookmark(usize),
    JumpToBookmark(usize),
    SetAnnotationMode(Option<crate::models::PendingAnnotationKind>),
    AnnotationDragStart {
        page: usize,
        x: f32,
        y: f32,
    },
    AnnotationDragUpdate {
        x: f32,
        y: f32,
    },
    AnnotationDragEnd,
    DeleteAnnotation(usize),
    Undo,
    Redo,
    SetFilter(RenderFilter),
    ToggleAutoCrop,
    DocumentOpenedWithPath((PathBuf, OpenResult)),
    OpenDocument,
    OpenFile(PathBuf),
    OpenRecentFile(RecentFile),
    ClearRecentFiles,
    CloseTab(usize),
    SwitchTab(usize),
    TabReordered(Vec<usize>),
    NextPage,
    PrevPage,
    ZoomOut,
    ZoomIn,
    SetZoom(f32),
    JumpToPage(usize),
    PageInputChanged(String),
    PageInputSubmitted,
    Search(String),
    PerformSearch(String),
    SearchResult(DocumentId, PdfResult<Vec<SearchResultItem>>),
    NextSearchResult,
    PrevSearchResult,
    ClearSearch,
    DocumentOpened(DocumentId, PdfResult<OpenResult>),
    PageRendered(DocumentId, usize, f32, PdfResult<RenderResult>),
    ThumbnailRendered(DocumentId, usize, f32, PdfResult<RenderResult>),
    TextItemsLoaded(DocumentId, usize, PdfResult<Vec<TextItem>>),
    DocumentMetaLoaded(DocumentId, PdfResult<DocumentMeta>),
    RequestRender(usize),
    ViewportChanged(f32, f32),
    SidebarViewportChanged(f32),
    ExtractText,
    ExtractTextToClipboard,
    TextExtracted(PdfResult<String>),
    CopyToClipboard(String),
    CopyImageToClipboard,
    SaveAnnotations,
    AnnotationsSaved(PdfResult<String>),
    AnnotationsLoaded(DocumentId, Vec<crate::models::Annotation>),
    MergeDocuments(Vec<PathBuf>),
    DocumentsMerged(PdfResult<String>),
    SplitPDF(Vec<usize>),
    PDFSplit(PdfResult<Vec<String>>),
    ToggleMetadata,
    LoadFormFields,
    FormFieldsLoaded(PdfResult<Vec<crate::models::FormField>>),
    FormFieldChanged(String, crate::models::FormFieldVariant),
    FillForm(Vec<crate::models::FormField>),
    FormFilled(PdfResult<String>),
    ExportImage,
    ImageExported(PdfResult<String>),
    ExportImages,
    Print,
    ListPrinters,
    PrintersListed(PdfResult<Vec<String>>),
    PrintWithPrinter(String),
    PrintDone(PdfResult<()>),
    AddWatermark(String),
    WatermarkDone(PdfResult<String>),
    HeaderFooterDone(PdfResult<String>),
    PermissionsDone(PdfResult<String>),
    OptimizePDF,
    PDFOptimized(PdfResult<String>),
    EngineInitialized(EngineState),
    Error(String),
    ClearStatus,
    IcedEvent(iced::Event),
    LinkClicked(crate::models::Hyperlink),
    ForceQuit,
    DocumentModifiedExternally(PathBuf),
    SetSidebarMode(crate::models::SidebarMode),
    SetReadingMode(crate::models::ReadingMode),
    SetAnnotationColor(String),
    SetAnnotationThickness(f32),
    SetAnnotationTextSize(f32),
    ReloadDocument(PathBuf),
    ToggleWatermarkPrompt(bool),
    WatermarkInputChanged(String),
    SubmitWatermark,
    ToggleHeaderFooterPrompt(bool),
    HeaderInputChanged(String),
    FooterInputChanged(String),
    SubmitHeaderFooter,
    TogglePermissionsPrompt(bool),
    TogglePrintPermission,
    ToggleCopyPermission,
    ToggleEditPermission,
    SubmitPermissions,
    OpenContainingFolder,
    ToggleSignatureCreator(bool),
    SignatureDragStart {
        x: f32,
        y: f32,
    },
    SignatureDragUpdate {
        x: f32,
        y: f32,
    },
    SignatureDragEnd,
    ClearSignature,
    SaveSignature,
    TogglePageOrganizer(bool),
    OrganizerDeletePage(usize),
    OrganizerRotatePage(usize, i32),
    OrganizerMovePage(usize, isize),
    AnnotationTextChanged(String),
    SaveOrganizedPDF,
    OrganizedPDFSaved(crate::models::PdfResult<String>),
    EditAnnotationText(usize, String),
    PasswordInputChanged(String),
    SubmitPassword,
    CancelPasswordPrompt,
    ToggleMarkupBar,
    ToggleSignaturesDetail(bool),
    SaveAttachment(usize),
    AttachmentSaved(crate::models::PdfResult<String>),
    ToggleLayer(usize, bool),
    LayerToggled,
    ToggleTableMode,
    TablesDetected(
        crate::models::DocumentId,
        usize,
        crate::models::PdfResult<Vec<crate::models::DetectedTable>>,
    ),
    SetRibbonTab(crate::models::RibbonTab),

    // ── Feature 1: New Blank Document ────────────────────────────────────────
    /// User triggered "File → New Document"
    CreateBlankDocument,
    /// Engine returned the path of the newly created blank PDF
    BlankDocumentCreated(crate::models::PdfResult<String>),

    // ── Feature 2: Digital Certificate Signing ───────────────────────────────
    /// Open file picker to select a .p12/.pfx certificate
    PickCertificate,
    /// Certificate path was selected by the file picker
    CertPathSelected(PathBuf),
    /// Start the signing operation with the stored cert path
    SignWithCertificate,
    /// Engine returned the path of the signed PDF
    CertSigningDone(crate::models::PdfResult<String>),
    /// Toggle the certificate-signing panel open/closed
    ToggleCertSigner(bool),

    // ── Feature 3: Stamp Tool ────────────────────────────────────────────────
    /// Toggle the stamps dropdown in the Annotate ribbon
    ShowStampMenu(bool),
    /// Apply a named stamp to the current page
    ApplyStamp(String),
    /// Engine returned the path of the stamped PDF
    StampApplied(crate::models::PdfResult<String>),

    // ── Feature 5: CMYK ↔ RGB Color Inspector ────────────────────────────────
    /// Toggle the CMYK color inspector panel
    ToggleCmykInspector(bool),
    /// One CMYK channel changed: (`channel_index` 0-3, `new_value` 0.0..=1.0)
    CmykValueChanged(usize, f64),

    // ── Feature 6: OCR Text Recognition ──────────────────────────────────────
    /// Select target script language for OCR (Latin or Devanagari)
    SelectOcrScript(crate::ocr::OcrScript),
    /// Trigger OCR analysis on the currently active document page
    TriggerOcrCurrentPage(crate::ocr::OcrScript),
    /// Engine returned OCR extraction result for page
    OcrPageCompleted(
        crate::models::DocumentId,
        usize,
        crate::models::PdfResult<crate::ocr::OcrPageResult>,
    ),
    /// Toggle visibility of OCR bounding box overlay on current page
    ToggleOcrResultsOverlay(bool),

    // ── Document Export & Conversion ─────────────────────────────────────────
    /// Export active PDF to Markdown (.md)
    ExportDocumentMarkdown,
    /// Export active PDF to HTML5 (.html)
    ExportDocumentHtml,
    /// Export active PDF to Plain Text (.txt)
    ExportDocumentTxt,
    /// Document conversion/export finished with output path or error
    DocumentExported(crate::models::PdfResult<String>),

    // ── Command Palette ──────────────────────────────────────────────────────
    /// Open/close the quick command palette modal
    ToggleCommandPalette,
    /// Query text changed in the command palette
    CommandPaletteQueryChanged(String),
    /// Execute a command action selected from the palette
    ExecutePaletteAction(crate::models::CommandAction),
    /// Select next item in palette
    PaletteSelectNext,
    /// Select previous item in palette
    PaletteSelectPrev,
    /// Submit currently selected palette item
    PaletteSubmit,
}
