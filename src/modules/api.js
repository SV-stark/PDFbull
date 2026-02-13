/**
 * ═══════════════════════════════════════════════════════════════════════════
 * PDFbull - Backend API Module
 * ═══════════════════════════════════════════════════════════════════════════
 * 
 * Wrapper module for Tauri IPC calls to the Rust backend.
 * Provides a clean interface for PDF operations.
 * 
 * @module api
 */

const { invoke } = window.__TAURI__.core;

/**
 * Backend API interface for PDF operations
 */
export const api = {
    /**
     * Open a PDF document
     * @param {string} path - Absolute file path to the PDF
     * @returns {Promise<number>} Number of pages in the document
     * @throws {Error} If document cannot be opened
     */
    openDocument: (path) => invoke('open_document', { path }),

    /**
     * Close a PDF document
     * @param {string} path - Document path to close
     * @returns {Promise<void>}
     */
    closeDocument: (path) => invoke('close_document', { path }),

    /**
     * Set active document
     * @param {string} path - Document path to activate
     * @returns {Promise<void>}
     */
    setActiveDocument: (path) => invoke('set_active_document', { path }),

    /**
     * Get the page count of the current document
     * @returns {Promise<number>}
     */
    getPageCount: () => invoke('get_page_count'),

    /**
     * Get dimensions of all pages
     * @returns {Promise<Array<[number, number]>>} Array of [width, height] tuples
     */
    getPageDimensions: () => invoke('get_page_dimensions'),

    /**
     * Render a specific page at the given scale
     * @param {number} pageNum - Page number (0-indexed)
     * @param {number} scale - Render scale factor
     * @returns {Promise<ArrayBuffer>} Raw image data as binary
     */
    renderPage: (pageNum, scale) => invoke('render_page', { pageNum, scale }),

    /**
     * Extract text content from a specific page
     * @param {number} pageNum - Page number (0-indexed)
     * @returns {Promise<string>} Text content of the page
     */
    getPageText: (pageNum) => invoke('get_page_text', { pageNum }),

    /**
     * Save data to a file
     * @param {string} path - File path
     * @param {Uint8Array|number[]} data - File data as byte array
     * @returns {Promise<void>}
     */
    saveFile: (path, data) => invoke('save_file', { path, data }),

    /**
     * Search for text on a specific page
     * @param {number} pageNum - Page number (0-indexed)
     * @param {string} query - Search query string
     * @returns {Promise<Array<[number, number, number, number]>>} Bounding boxes [x, y, w, h]
     */
    searchText: (pageNum, query) => invoke('search_text', { pageNum, query }),

    /**
     * Search for text in the entire document
     * @param {string} query - Search query string
     * @returns {Promise<Array<{page: number, x: number, y: number, w: number, h: number}>>} Search results
     */
    searchDocument: (query) => invoke('search_document', { query }),

    /**
     * Get text with coordinates for text selection layer
     * @param {number} pageNum - Page number (0-indexed)
     * @returns {Promise<Array<{text: string, x: number, y: number, w: number, h: number}>>}
     */
    getPageTextRects: (pageNum) => invoke('get_page_text_with_coords', { pageNum }),

    /**
     * Apply a visual filter to a page
     * @param {number} pageNum - Page number (0-indexed)
     * @param {string} filterType - Filter type ('grayscale', 'invert', etc.)
     * @param {number} intensity - Filter intensity (0.0 to 1.0)
     * @returns {Promise<void>}
     */
    applyFilter: (pageNum, filterType, intensity) => invoke('apply_filter', { pageNum, filterType, intensity }),

    /**
     * Apply scanner-style filter to document
     * @param {string} docPath - Document file path
     * @param {string} filterType - Filter type
     * @param {number} intensity - Filter intensity (0.0 to 1.0)
     * @returns {Promise<void>}
     */
    applyScannerFilter: (docPath, filterType, intensity) => invoke('apply_scanner_filter', { docPath, filterType, intensity }),

    /**
     * Get form fields from a specific page
     * @param {number} pageNum - Page number (0-indexed)
     * @returns {Promise<Array<Object>>} Array of form field objects
     */
    getFormFields: (pageNum) => invoke('get_form_fields', { pageNum }),

    /**
     * Compress the PDF file
     * @param {string} inputPath - Input PDF path
     * @param {string} outputPath - Output PDF path
     * @param {string} level - Compression level ('low', 'standard', 'high')
     * @returns {Promise<Object>} Compression result
     */
    compressPdf: (inputPath, outputPath, level) => invoke('compress_pdf', { inputPath, outputPath, level }),

    /**
     * Auto-crop whitespace from a page
     * @param {number} pageNum - Page number (0-indexed)
     * @returns {Promise<void>}
     */
    autoCrop: (pageNum) => invoke('auto_crop', { pageNum }),
};

