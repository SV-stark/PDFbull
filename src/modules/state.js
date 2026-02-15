/**
 * ═══════════════════════════════════════════════════════════════════════════
 * PDFbull - Application State Module
 * ═══════════════════════════════════════════════════════════════════════════
 * 
 * Central state management for the PDFbull application.
 * All mutable application state is stored here for consistency.
 * 
 * @module state
 */

/**
 * @typedef {import('../types').DocumentState} DocumentState
 * @typedef {import('../types').Annotation} Annotation
 */

/**
 * Global application state object
 * @type {Object}
 */
export const state = {
    // ─────────────────────────────────────
    // PAGE RENDERING
    // ─────────────────────────────────────

    /** @type {number} Current page index (0-based) */
    currentPage: 0,

    /** @type {number} Total pages in current document */
    totalPages: 0,

    /** @type {number} Current zoom level (1.0 = 100%) */
    currentZoom: 1.0,

    /** @type {number} Render scale factor */
    renderScale: 1.0,

    /** @type {number} Page rotation in degrees (0, 90, 180, 270) */
    rotation: 0,

    /** @type {number|null} Zoom debounce timeout ID */
    zoomTimeout: null,

    /** @type {string|null} Current document file path */
    currentDoc: null,

    /** @type {string|null} Active visual filter ('grayscale', 'invert', null) */
    activeFilter: null,

    /** @type {Array<Object>} Search results */
    searchResults: [],

    /** @type {number} Current search result index */
    currentSearchIndex: -1,

    // ─────────────────────────────────────
    // CACHE
    // ─────────────────────────────────────

    /** @type {Map<number, ImageData>} Rendered page cache by page number */
    pageCache: new Map(),

    /** @type {number} Current cache size in bytes */
    currentCacheBytes: 0,

    /** @type {number} Maximum cache size (256 MB) */
    MAX_CACHE_BYTES: 256 * 1024 * 1024,

    // ─────────────────────────────────────
    // MULTI-DOCUMENT SUPPORT
    // ─────────────────────────────────────

    /** @type {Map<string, DocumentState>} Open document tabs */
    openDocuments: new Map(),

    /** @type {string|null} Currently active tab ID */
    activeTabId: null,

    /** @type {number} Counter for generating unique tab IDs */
    tabCounter: 0,

    /** @type {number} Current render request ID for cancellation */
    currentRenderRequest: 0,

    // ─────────────────────────────────────
    // ANNOTATIONS
    // ─────────────────────────────────────

    /** @type {Map<number, Annotation[]>} Annotations per page */
    annotations: new Map(),

    /** @type {string} Current drawing tool ('view', 'highlight', 'rectangle', etc.) */
    currentTool: 'view',

    /** @type {boolean} Whether user is currently drawing */
    isDrawing: false,

    /** @type {number} Drawing start X coordinate */
    startX: 0,

    /** @type {number} Drawing start Y coordinate */
    startY: 0,

    /** @type {Object|null} Current shape being drawn */
    currentShape: null,

    /** @type {string} Currently selected annotation color (hex) */
    selectedColor: '#ffeb3b',

    /** @type {string} Current annotation layer */
    currentLayer: 'default',

    /** @type {Set<string>} Set of visible layer names */
    visibleLayers: new Set(['default']),

    // Bookmarks
    bookmarks: new Set(),

    // ─────────────────────────────────────
    // VIEW MODES
    // ─────────────────────────────────────

    /** @type {string} Current view mode ('single', 'continuous', 'facing', 'book') */
    viewMode: 'single',

    /** @type {boolean} Is continuous scroll mode */
    continuousMode: false,

    /** @type {boolean} Is facing (dual page) mode */
    facingMode: false,

    /** @type {boolean} Is book view (right-to-left) */
    bookView: false,

    // ─────────────────────────────────────
    // OUTLINE / TOC
    // ─────────────────────────────────────

    /** @type {Array} PDF outline/table of contents */
    outline: [],

    // ─────────────────────────────────────
    // LINKS
    // ─────────────────────────────────────

    /** @type {Array} Links on current page */
    pageLinks: [],

    // ─────────────────────────────────────
    // PAN / DRAG SCROLL
    // ─────────────────────────────────────

    /** @type {boolean} Is currently panning (drag scroll) */
    isPanning: false,

    /** @type {number} Pan start X */
    panStartX: 0,

    /** @type {number} Pan start Y */
    panStartY: 0,

    /** @type {number} Scroll container start scrollLeft */
    scrollStartLeft: 0,

    /** @type {number} Scroll container start scrollTop */
    scrollStartTop: 0,

    // ─────────────────────────────────────
    // HISTORY (UNDO/REDO)
    // ─────────────────────────────────────

    /** @type {Array<Object>} History stack for undo operations */
    history: [],

    /** @type {number} Current position in history stack (-1 = empty) */
    historyIndex: -1,

    /** @type {number} Maximum history entries to keep */
    MAX_HISTORY_SIZE: 50,

    // ─────────────────────────────────────
    // BATCH PROCESSING
    // ─────────────────────────────────────

    /** @type {Set<number>} Selected pages for batch operations */
    selectedPages: new Set(),

    /** @type {boolean} Whether batch mode is active */
    batchMode: false,

    // ─────────────────────────────────────
    // VIRTUAL SCROLLER
    // ─────────────────────────────────────

    /** @type {Array<[number, number]>} Page dimensions [width, height] */
    pageDimensions: [],

    /** @type {IntersectionObserver|null} Observer for lazy page loading */
    pageObserver: null,

    /** @type {Set<number>} Currently visible page numbers */
    visiblePages: new Set(),
    
    // ─────────────────────────────────────
    // PERFORMANCE
    // ─────────────────────────────────────
    
    /** @type {number} Current render request ID for cancellation */
    currentRenderRequest: 0,
    
    /** @type {Map<number, number>} Render request IDs per page */
    pageRenderRequests: new Map(),
    
    /** @type {boolean} Whether there are unsaved changes */
    isDirty: false,
    
    /** @type {Map<string, Map<number, Object>>} Text layer cache per zoom */
    textLayerCache: new Map(),
    
    /** @type {Map<string, CanvasRenderingContext2D>} Cached canvas contexts */
    canvasContexts: new Map(),
    
    // ─────────────────────────────────────
    // DEBUG
    // ─────────────────────────────────────
    
    /** @type {boolean} Verbose debug logging mode */
    verboseDebug: false
};

/**
 * Reset application state to initial values
 * @returns {void}
 */
export function resetState() {
    state.currentPage = 0;
    state.totalPages = 0;
    state.annotations.clear();
    state.history = [];
    state.historyIndex = -1;
    state.pageCache.clear();
    state.currentCacheBytes = 0;
    state.visiblePages.clear();
    state.selectedPages.clear();
    state.batchMode = false;
    state.isDirty = false;
    state.pageRenderRequests.clear();
    state.textLayerCache.clear();
    state.canvasContexts.clear();
}

