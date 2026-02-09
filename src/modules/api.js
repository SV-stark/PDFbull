const { invoke } = window.__TAURI__.core;

export const api = {
    openDocument: (path) => invoke('open_document', { path }),
    getPageCount: () => invoke('get_page_count'),
    getPageDimensions: () => invoke('get_page_dimensions'),
    renderPage: (pageNum, scale) => invoke('render_page', { pageNum, scale }),
    getPageText: (pageNum) => invoke('get_page_text', { pageNum }),
    saveFile: (path, data) => invoke('save_file', { path, data }),
    applyFilter: (pageNum, filterType, intensity) => invoke('apply_filter', { pageNum, filterType, intensity }), // Assuming filter logic exists/might be needed
    applyScannerFilter: (docPath, filterType, intensity) => invoke('apply_scanner_filter', { docPath, filterType, intensity }),
    getFormFields: () => invoke('get_form_fields'),
    compressPdf: (quality) => invoke('compress_pdf', { quality }),
    autoCrop: (pageNum) => invoke('auto_crop', { pageNum }),

    // Custom wrappers around invoke if needed for complex error handling
};
