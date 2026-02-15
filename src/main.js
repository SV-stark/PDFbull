import { state } from './modules/state.js';
import { api } from './modules/api.js';
import { ui } from './modules/ui.js';
import { renderer } from './modules/renderer.js';
import { events } from './modules/events.js';
import { settings, applySettings } from './modules/settings.js';
import { ocr } from './modules/ocr.js';
import { CONSTANTS } from './modules/constants.js';
import { CommandPalette } from './modules/commandPalette.js';
import { ContextMenu } from './modules/contextMenu.js';
import { debug } from './modules/debug.js';

// Controller Logic
const app = {
  async openNewTab(path) {
    // Check if document is already open
    for (const [tabId, doc] of state.openDocuments) {
      if (doc.path === path) {
        debug.log(`Document already open in tab ${tabId}, switching to it`);
        app.switchToTab(tabId);
        return;
      }
    }

    const tabId = `tab-${++state.tabCounter}`;

    try {
      ui.showLoading('Opening PDF...');
      debug.log(`Opening document: ${path}`);
      
      // Try fast open first for instant feedback
      let pageCount;
      try {
        const docInfo = await api.openDocumentFast(path);
        pageCount = docInfo.pageCount;
      } catch (e) {
        // Fallback to regular open
        pageCount = await api.openDocument(path);
      }
      
      debug.log(`Document opened with ${pageCount} pages`);

      state.openDocuments.set(tabId, {
        id: tabId,
        path: path,
        name: path.split(/[/\\]/).pop(),
        totalPages: pageCount,
        currentPage: 0,
        zoom: 1.0
      });

      app.addToRecentFiles(path);
      ui.createTabUI(tabId, state.openDocuments.get(tabId), app.switchToTab, app.closeTab);
      await app.switchToTab(tabId);
      
      // Render first page immediately for instant display
      await renderer.renderPage(0);
      
      ui.hideLoading();

      // Hide welcome screen
      const emptyState = document.getElementById('empty-state');
      if (emptyState) emptyState.style.display = 'none';
    } catch (e) {
      debug.error('Failed to open document:', e);
      ui.showToast('Error opening PDF: ' + e, 'error');
      ui.hideLoading();
    }
  },

  async switchToTab(tabId) {
    if (!state.openDocuments.has(tabId)) {
      debug.warn(`Tab ${tabId} not found`);
      return;
    }

    debug.log(`Switching to tab: ${tabId}`);

    if (state.activeTabId) {
      const currentDoc = state.openDocuments.get(state.activeTabId);
      if (currentDoc) {
        currentDoc.currentPage = state.currentPage;
        currentDoc.zoom = state.currentZoom;
      }
    }

    state.activeTabId = tabId;
    const doc = state.openDocuments.get(tabId);

    state.currentDoc = doc.path;
    state.totalPages = doc.totalPages;
    state.currentPage = doc.currentPage;
    state.currentZoom = doc.zoom;

    ui.updateActiveTab(tabId);

    await api.setActiveDocument(doc.path);
    debug.log(`Active document set to: ${doc.path}`);
    
    state.pageCache.clear();
    state.currentCacheBytes = 0;
    state.textLayerCache.clear();
    state.canvasContexts.clear();
    app.loadAnnotations();
    app.loadBookmarks();

    state.renderScale = state.currentZoom;
    await renderer.setupVirtualScroller();
    renderer.renderThumbnails();
    
    debug.log(`Tab switch complete`);
  },

  closeTab(tabId) {
    const tab = document.getElementById(tabId);
    if (tab) tab.remove();

    const doc = state.openDocuments.get(tabId);
    if (doc) {
      debug.log(`Closing document: ${doc.path}`);
      api.closeDocument(doc.path).catch(console.error);
    }

    state.openDocuments.delete(tabId);

    if (state.activeTabId === tabId) {
      const remainingTabs = Array.from(state.openDocuments.keys());
      if (remainingTabs.length > 0) {
        app.switchToTab(remainingTabs[0]);
      } else {
        state.currentDoc = null;
        state.totalPages = 0;
        state.currentPage = 0;
        state.annotations.clear();
        state.history = [];
        state.historyIndex = -1;
        state.isDirty = false;
        const pagesContainer = ui.elements.pagesContainer();
        if (pagesContainer) pagesContainer.innerHTML = '';
        ui.updateUI();
        ui.updateStatusBar();
      }
    }
  },

  addToRecentFiles(path) {
    let recentFiles = JSON.parse(localStorage.getItem('recentFiles') || '[]');
    recentFiles = recentFiles.filter(f => f.path !== path);
    recentFiles.unshift({
      path: path,
      name: path.split(/[/\\]/).pop(),
      timestamp: Date.now()
    });
    recentFiles = recentFiles.slice(0, CONSTANTS.MAX_RECENT_FILES);
    localStorage.setItem('recentFiles', JSON.stringify(recentFiles));
    ui.updateRecentFilesDropdown(recentFiles, app.openNewTab);
  },

  loadAnnotations() {
    if (!state.currentDoc) return;

    const savedAnnotations = JSON.parse(localStorage.getItem('pdfAnnotations') || '{}');
    const docAnnotations = savedAnnotations[state.currentDoc];

    if (docAnnotations) {
      state.annotations = new Map(docAnnotations.annotations);
      state.isDirty = false;
      ui.showToast('Annotations loaded');
    }
  },

  async handleSave() {
    if (!state.currentDoc) return;

    let allAnnotations = [];
    state.annotations.forEach((pageAnns, pageNum) => {
      pageAnns.forEach(ann => {
        if (ann.type === 'search_highlight') return;
        allAnnotations.push({
          page: parseInt(pageNum),
          type: ann.type,
          x: ann.x, y: ann.y, w: ann.w, h: ann.h,
          color: ann.color,
          text: ann.text || null,
          x1: ann.x1 || null, y1: ann.y1 || null, x2: ann.x2 || null, y2: ann.y2 || null
        });
      });
    });

    if (allAnnotations.length === 0) {
      ui.showToast('No annotations to save');
      return;
    }

    try {
      const { save } = window.__TAURI__.dialog;
      const savePath = await save({
        filters: [{ name: 'PDF with Annotations', extensions: ['pdf'] }],
        defaultPath: `${state.currentDoc.split(/[/\\]/).pop().replace('.pdf', '_annotated.pdf')}`
      });

      if (savePath) {
        ui.showLoading('Saving annotations...');
        await window.__TAURI__.core.invoke('save_annotations', {
          outputPath: savePath,
          annotations: allAnnotations
        });
        ui.hideLoading();
        ui.showToast('Annotations saved to new PDF', 'success');
      }
    } catch (e) {
      console.error('Save failed:', e);
      ui.hideLoading();
      ui.showToast('Save failed: ' + e, 'error');
    }
  },

  // Bookmarks
  toggleBookmark(pageNum) {
    if (state.bookmarks.has(pageNum)) {
      state.bookmarks.delete(pageNum);
      ui.showToast('Bookmark removed');
    } else {
      state.bookmarks.add(pageNum);
      ui.showToast('Bookmark added');
    }
    app.saveBookmarks();
    ui.updateBookmarkUI(pageNum);
  },

  markDirty() {
    state.isDirty = true;
  },

  saveBookmarks() {
    if (!state.currentDoc) return;
    const all = JSON.parse(localStorage.getItem('pdfBookmarks') || '{}');
    all[state.currentDoc] = Array.from(state.bookmarks);
    localStorage.setItem('pdfBookmarks', JSON.stringify(all));
  },

  loadBookmarks() {
    if (!state.currentDoc) return;
    const all = JSON.parse(localStorage.getItem('pdfBookmarks') || '{}');
    const docBookmarks = all[state.currentDoc];
    if (docBookmarks) {
      state.bookmarks = new Set(docBookmarks);
    } else {
      state.bookmarks.clear();
    }
    ui.updateBookmarkUI(state.currentPage);
  }
};

// Initialize
console.log('PDFbull initializing modules...');

// Allow events.js to trigger app methods via events
document.addEventListener('app:open-file', (e) => {
  if (e.detail) app.openNewTab(e.detail);
});

document.addEventListener('app:save', () => {
  app.handleSave();
});

document.addEventListener('app:toggle-bookmark', () => {
  if (state.currentDoc) app.toggleBookmark(state.currentPage);
});

document.addEventListener('app:export-json', () => {
  if (!state.currentDoc) return;

  const data = {
    document: state.currentDoc,
    exportedAt: new Date().toISOString(),
    annotations: Array.from(state.annotations.entries()).map(([page, anns]) => ({
      page,
      items: anns.filter(a => a.type !== 'search_highlight')
    }))
  };

  const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `${state.currentDoc.split(/[/\\]/).pop()}_annotations.json`;
  a.click();
  URL.revokeObjectURL(url);
  ui.showToast('Annotations exported as JSON');
});

document.addEventListener('app:scan-forms', async () => {
  if (!state.currentDoc) {
    ui.showToast('No document open', 'error');
    return;
  }
  try {
    ui.showLoading('Scanning for form fields...');
    const fields = await api.getFormFields(state.currentPage);
    ui.hideLoading();
    if (fields.length === 0) {
      ui.showToast('No form fields found on this page');
    } else {
      // Add form field annotations
      const pageAnns = state.annotations.get(state.currentPage) || [];
      const newAnns = fields.map(f => ({
        id: 'form-' + Math.random(),
        type: 'form_field',
        layer: 'default',
        x: f.x,
        y: f.y,
        w: f.w,
        h: f.h,
        color: 'rgba(0, 150, 255, 0.3)',
        text: f.name,
        fieldType: f.field_type
      }));
      state.annotations.set(state.currentPage, [...pageAnns, ...newAnns]);
      renderer.drawAnnotations(state.currentPage);
      ui.showToast(`Found ${fields.length} form fields`);
    }
  } catch (e) {
    ui.hideLoading();
    ui.showToast('Form scan failed: ' + e, 'error');
  }
});

// Init Events
events.init();

// Init Debug Module
debug.init();

// Init OCR module
ocr.init();

// Init UX Modules
window.commandPalette = new CommandPalette();
window.contextMenu = new ContextMenu();

// Apply Settings
applySettings();

// Auto-Save Logic
let autoSaveTimer = null;
function startAutoSave() {
  if (autoSaveTimer) clearInterval(autoSaveTimer);
  const interval = settings.get('autoSaveInterval') * 1000;

  if (interval > 0) {
    autoSaveTimer = setInterval(() => {
      if (state.currentDoc && state.annotations.size > 0 && state.isDirty) {
        const saved = JSON.parse(localStorage.getItem('pdfAnnotations') || '{}');
        const currentAnns = state.annotations;
        const annArray = [];
        currentAnns.forEach((anns, page) => {
          annArray.push({ page, annotations: anns });
        });

        const serializableAnns = Array.from(state.annotations.entries());

        saved[state.currentDoc] = { annotations: serializableAnns };
        localStorage.setItem('pdfAnnotations', JSON.stringify(saved));
        
        state.isDirty = false;
      }
    }, interval);
  }
}

// Restart auto-save when settings change (we can hook into settings save or just rely on reload)
// For now, start it once.
startAutoSave();

// Load recents
const recentFiles = JSON.parse(localStorage.getItem('recentFiles') || '[]');
ui.updateRecentFilesDropdown(recentFiles, app.openNewTab);

ui.updateStatusBar();
ui.updateUndoRedoButtons();
ui.setStatusMessage('Ready - Drag and drop PDFs onto the window to open');

// Show empty state initially
const emptyState = document.getElementById('empty-state');
if (emptyState && state.openDocuments.size === 0) {
  emptyState.style.display = 'flex';
}

console.log('PDFbull modules loaded.');
