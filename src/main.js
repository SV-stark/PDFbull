import { state } from './modules/state.js';
import { api } from './modules/api.js';
import { ui } from './modules/ui.js';
import { renderer } from './modules/renderer.js';
import { events } from './modules/events.js';
import { settings, applySettings } from './modules/settings.js';
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

    // Update visual zoom state
    state.renderScale = state.currentZoom;
    renderer.setupVirtualScroller();
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

  saveWithAnnotations() {
    // Logic moved from main.js, needs implementation or move to tools/renderer?
    // It involves `api.saveFile` (via invoke save_annotations)
    // Let's implement it here as it matches "Save" action
    if (!state.currentDoc) return;

    // Flatten annotations from map to array
    let allAnnotations = [];
    state.annotations.forEach((pageAnns, pageNum) => {
      pageAnns.forEach(ann => {
        if (ann.type === 'search_highlight') return;

        allAnnotations.push({
          page: parseInt(pageNum),
          type: ann.type,
          x: ann.x,
          y: ann.y,
          w: ann.w,
          h: ann.h,
          color: ann.color,
          text: ann.text || null,
          x1: ann.x1 || null,
          y1: ann.y1 || null,
          x2: ann.x2 || null,
          y2: ann.y2 || null
        });
      });
    });

    if (allAnnotations.length === 0) {
      ui.showToast('No annotations to save');
      return;
    }

    // We need `save` dialog from Tauri.
    // `api.js` only wraps invoke.
    // We need `window.__TAURI__.dialog.save`
    // Or move this logic to `events.js`?
    // `events.js` dispatched `app:save`.
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

// Init Events
events.init();

// Apply Settings
applySettings();

// Load recents
const recentFiles = JSON.parse(localStorage.getItem('recentFiles') || '[]');
ui.updateRecentFilesDropdown(recentFiles, app.openNewTab);

ui.updateStatusBar();
ui.updateUndoRedoButtons();
ui.setStatusMessage('Ready - Drag and drop PDFs onto the window to open');

// Backend Test
api.openDocument('ping').then(() => { }).catch(() => { }); // Just generic invoke test if needed, or stick to dedicated ping
// In api.js we didn't add ping.
// window.__TAURI__.core.invoke('ping').then(...);

console.log('PDFbull modules loaded.');
