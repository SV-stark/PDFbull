import { state } from './modules/state.js';
import { api } from './modules/api.js';
import { ui } from './modules/ui.js';
import { renderer } from './modules/renderer.js';
import { events } from './modules/events.js';
import { settings, applySettings } from './modules/settings.js';
import { ocr } from './modules/ocr.js';
import { CONSTANTS } from './modules/constants.js';

// Controller Logic
const app = {
  async openNewTab(path) {
    const tabId = `tab-${++state.tabCounter}`;

    try {
      ui.showLoading('Opening PDF...');
      const pages = await api.openDocument(path);

      state.openDocuments.set(tabId, {
        id: tabId,
        path: path,
        name: path.split(/[/\\]/).pop(),
        totalPages: pages,
        currentPage: 0,
        zoom: 1.0
      });

      app.addToRecentFiles(path);
      ui.createTabUI(tabId, state.openDocuments.get(tabId), app.switchToTab, app.closeTab);
      app.switchToTab(tabId);
      ui.hideLoading();
    } catch (e) {
      console.error('Failed to open document:', e);
      ui.showToast('Error opening PDF: ' + e, 'error');
      ui.hideLoading();
    }
  },

  switchToTab(tabId) {
    if (!state.openDocuments.has(tabId)) return;

    // Save current state
    if (state.activeTabId) {
      const currentDoc = state.openDocuments.get(state.activeTabId);
      if (currentDoc) {
        currentDoc.currentPage = state.currentPage;
        currentDoc.zoom = state.currentZoom;
      }
    }

    // Switch to new tab
    state.activeTabId = tabId;
    const doc = state.openDocuments.get(tabId);

    state.currentDoc = doc.path;
    state.totalPages = doc.totalPages;
    state.currentPage = doc.currentPage;
    state.currentZoom = doc.zoom;

    // Update UI
    ui.updateActiveTab(tabId);

    state.pageCache.clear(); // Or manage per tab? Original code cleared it.
    // loadAnnotations(); // Need to migrate loadAnnotations
    app.loadAnnotations();
    app.loadBookmarks();

    // Update visual zoom state
    state.renderScale = state.currentZoom;
    state.renderScale = state.currentZoom;
    renderer.setupVirtualScroller();
    renderer.renderThumbnails();
  },

  closeTab(tabId) {
    // Remove tab UI
    const tab = document.getElementById(tabId);
    if (tab) tab.remove();

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
    // Update UI if current page is bookmarked
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

// Init Events
events.init();

// Init OCR module
ocr.init();

// Apply Settings
applySettings();

// Auto-Save Logic
let autoSaveTimer = null;
function startAutoSave() {
  if (autoSaveTimer) clearInterval(autoSaveTimer);
  const interval = settings.get('autoSaveInterval') * 1000;

  if (interval > 0) {
    autoSaveTimer = setInterval(() => {
      if (state.currentDoc && state.annotations.size > 0) {
        const saved = JSON.parse(localStorage.getItem('pdfAnnotations') || '{}');
        // Only save if we have annotations for the current doc
        const currentAnns = state.annotations;
        // logic to convert map to array for storage
        const annArray = [];
        currentAnns.forEach((anns, page) => {
          annArray.push({ page, annotations: anns });
        });

        // Actually we need to match the loadAnnotations format
        // loadAnnotations does: state.annotations = new Map(docAnnotations.annotations);
        // So docAnnotations.annotations should be an array of [key, value] pairs or compatible?
        // Wait, JSON.stringify(map) returns {}, maps don't stringify well.
        // We need to convert Map to Array of entries.

        const serializableAnns = Array.from(state.annotations.entries());

        saved[state.currentDoc] = { annotations: serializableAnns };
        localStorage.setItem('pdfAnnotations', JSON.stringify(saved));

        // Optional: toast only on first save or periodic? 
        // Plan said toast "Auto-saved".
        // Let's debounce the toast or it gets annoying.
        // ui.showToast('Auto-saved', 'info', 1500);
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

console.log('PDFbull modules loaded.');
