/**
 * ═══════════════════════════════════════════════════════════════════════════
 * PDFbull - Tauri Bridge Module
 * ═══════════════════════════════════════════════════════════════════════════
 * 
 * Centralized wrapper for all Tauri IPC (Inter-Process Communication) calls.
 * This module abstracts the `window.__TAURI__` API to provide:
 * - Better type safety via JSDoc
 * - Centralized error handling
 * - Easier testing and mocking
 * - Single source of truth for Tauri interactions
 */

/**
 * @typedef {Object} Annotation
 * @property {number} page - Page number (0-indexed)
 * @property {'highlight'|'rectangle'|'circle'|'line'|'arrow'|'text'|'note'} type
 * @property {number} x - X coordinate
 * @property {number} y - Y coordinate
 * @property {number} [w] - Width (for shapes)
 * @property {number} [h] - Height (for shapes)
 * @property {string} color - Hex color string
 * @property {string} [text] - Text content (for text/note annotations)
 * @property {number} [x1] - Start X (for lines/arrows)
 * @property {number} [y1] - Start Y (for lines/arrows)
 * @property {number} [x2] - End X (for lines/arrows)
 * @property {number} [y2] - End Y (for lines/arrows)
 */

/**
 * @typedef {Object} SaveDialogOptions
 * @property {Array<{name: string, extensions: string[]}>} [filters] - File type filters
 * @property {string} [defaultPath] - Default file path/name
 * @property {string} [title] - Dialog title
 */

/**
 * @typedef {Object} OpenDialogOptions
 * @property {Array<{name: string, extensions: string[]}>} [filters] - File type filters
 * @property {string} [title] - Dialog title
 * @property {boolean} [multiple] - Allow multiple file selection
 */

/**
 * Tauri API bridge - centralized access to Tauri functionality
 */
export const tauri = {
    /**
     * Raw invoke function for custom commands
     * @param {string} command - Tauri command name
     * @param {Object} [args] - Command arguments
     * @returns {Promise<*>}
     */
    async invoke(command, args = {}) {
        return window.__TAURI__.core.invoke(command, args);
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // DOCUMENT OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /**
     * Open a PDF document
     * @param {string} path - Absolute file path to the PDF
     * @returns {Promise<number>} - Number of pages in the document
     * @throws {Error} If document cannot be opened
     */
    async openDocument(path) {
        return window.__TAURI__.core.invoke('open_document', { path });
    },

    /**
     * Get the page count of the currently loaded document
     * @returns {Promise<number>}
     */
    async getPageCount() {
        return window.__TAURI__.core.invoke('get_page_count');
    },

    /**
     * Render a specific page at the given scale
     * @param {number} pageNum - Page number (0-indexed)
     * @param {number} scale - Render scale factor
     * @returns {Promise<ArrayBuffer>} - Raw image data
     */
    async renderPage(pageNum, scale) {
        return window.__TAURI__.core.invoke('render_page', { pageNum, scale });
    },

    /**
     * Get dimensions of all pages
     * @returns {Promise<Array<[number, number]>>} - Array of [width, height] tuples
     */
    async getPageDimensions() {
        return window.__TAURI__.core.invoke('get_page_dimensions');
    },

    /**
     * Get text content from a specific page
     * @param {number} pageNum - Page number (0-indexed)
     * @returns {Promise<string>}
     */
    async getPageText(pageNum) {
        return window.__TAURI__.core.invoke('get_page_text', { pageNum });
    },

    /**
     * Search for text on a specific page
     * @param {number} pageNum - Page number (0-indexed)
     * @param {string} query - Search query
     * @returns {Promise<Array<[number, number, number, number]>>} - Bounding boxes [x, y, w, h]
     */
    async searchText(pageNum, query) {
        return window.__TAURI__.core.invoke('search_text', { pageNum, query });
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // FILE OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /**
     * Save file to disk
     * @param {string} path - File path
     * @param {Uint8Array} data - File data
     * @returns {Promise<void>}
     */
    async saveFile(path, data) {
        return window.__TAURI__.core.invoke('save_file', {
            path,
            data: Array.from(data)
        });
    },

    /**
     * Save annotations to a new PDF
     * @param {string} outputPath - Output file path
     * @param {Annotation[]} annotations - Annotations to embed
     * @returns {Promise<void>}
     */
    async saveAnnotations(outputPath, annotations) {
        return window.__TAURI__.core.invoke('save_annotations', {
            outputPath,
            annotations
        });
    },

    /**
     * Compress a PDF file
     * @returns {Promise<void>}
     */
    async compressPdf() {
        return window.__TAURI__.core.invoke('compress_pdf');
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // FILTERS & PROCESSING
    // ═══════════════════════════════════════════════════════════════════════════

    /**
     * Apply a visual filter to the document
     * @param {string} filterType - Filter type ('grayscale', 'invert', etc.)
     * @param {number} [intensity=1.0] - Filter intensity (0.0 to 1.0)
     * @returns {Promise<void>}
     */
    async applyFilter(filterType, intensity = 1.0) {
        return window.__TAURI__.core.invoke('apply_filter', { filterType, intensity });
    },

    /**
     * Auto-crop whitespace from pages
     * @returns {Promise<void>}
     */
    async autoCrop() {
        return window.__TAURI__.core.invoke('auto_crop');
    },

    /**
     * Get form fields from the document
     * @returns {Promise<Array>}
     */
    async getFormFields() {
        return window.__TAURI__.core.invoke('get_form_fields');
    },

    /**
     * Apply scanner filter to document pages
     * @param {string} docPath - Document path
     * @param {string} filterType - Filter type
     * @param {number} intensity - Filter intensity
     * @returns {Promise<void>}
     */
    async applyScannerFilter(docPath, filterType, intensity) {
        return window.__TAURI__.core.invoke('apply_scanner_filter', {
            docPath,
            filterType,
            intensity
        });
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // DIALOGS
    // ═══════════════════════════════════════════════════════════════════════════

    /**
     * Show a save file dialog
     * @param {SaveDialogOptions} options - Dialog options
     * @returns {Promise<string|null>} - Selected file path or null if cancelled
     */
    async showSaveDialog(options = {}) {
        return window.__TAURI__.dialog.save(options);
    },

    /**
     * Show an open file dialog
     * @param {OpenDialogOptions} options - Dialog options
     * @returns {Promise<string|string[]|null>} - Selected path(s) or null if cancelled
     */
    async showOpenDialog(options = {}) {
        return window.__TAURI__.dialog.open(options);
    },

    // ═══════════════════════════════════════════════════════════════════════════
    // UTILITIES
    // ═══════════════════════════════════════════════════════════════════════════

    /**
     * Test connection to backend
     * @returns {Promise<string>}
     */
    async ping() {
        return window.__TAURI__.core.invoke('ping');
    },

    /**
     * Test PDFium library
     * @returns {Promise<string>}
     */
    async testPdfium() {
        return window.__TAURI__.core.invoke('test_pdfium');
    }
};

export default tauri;
