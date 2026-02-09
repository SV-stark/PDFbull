export const state = {
    // Page rendering
    currentPage: 0,
    totalPages: 0,
    currentZoom: 1.0,
    renderScale: 1.0,
    zoomTimeout: null,
    currentDoc: null,
    activeFilter: null,

    // Cache
    pageCache: new Map(),
    currentCacheBytes: 0,
    MAX_CACHE_BYTES: 256 * 1024 * 1024, // 256 MB Limit

    // Multi-document support
    openDocuments: new Map(),
    activeTabId: null,
    tabCounter: 0,
    currentRenderRequest: 0,

    // Annotations
    annotations: new Map(),
    currentTool: 'view',
    isDrawing: false,
    startX: 0,
    startY: 0,
    currentShape: null,
    selectedColor: '#ffeb3b',
    currentLayer: 'default',
    visibleLayers: new Set(['default']),

    // History
    history: [],
    historyIndex: -1,
    MAX_HISTORY_SIZE: 50,

    // Batch
    selectedPages: new Set(),
    batchMode: false,

    // Virtual Scroller
    pageDimensions: [],
    pageObserver: null,
    visiblePages: new Set()
};

// Also export individual getters/setters if strict reactivity is needed, 
// but for this refactor, replacing `variable = value` with `state.variable = value` is easiest.

export function resetState() {
    state.currentPage = 0;
    state.totalPages = 0;
    state.annotations.clear();
    state.history = [];
    state.historyIndex = -1;
    // ... clear others as needed
}
