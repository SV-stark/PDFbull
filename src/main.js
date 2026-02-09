// In a non-bundled setup, we use the global window.__TAURI__ object
const { open, save } = window.__TAURI__.dialog;
const { invoke } = window.__TAURI__.core;

// State Management
let currentPage = 0;
let totalPages = 0;
let currentZoom = 1.0;
let currentDoc = null;
let activeFilter = null;
let pageCache = new Map();
const MAX_CACHE_SIZE = 10;

// Multi-document support
let openDocuments = new Map();
let activeTabId = null;
let tabCounter = 0;
let currentRenderRequest = 0; // For handling race conditions

// Annotation system
let annotations = new Map();
let currentTool = 'view';
let isDrawing = false;
let startX = 0;
let startY = 0;
let currentShape = null;
let selectedColor = '#ffeb3b';
let currentLayer = 'default';
let visibleLayers = new Set(['default']);

// History for undo/redo
let history = [];
let historyIndex = -1;
const MAX_HISTORY_SIZE = 50;

// UI Elements
const canvas = document.getElementById('pdf-canvas');
const ctx = canvas.getContext('2d');
const pageIndicator = document.getElementById('page-indicator');
const zoomIndicator = document.getElementById('zoom-level');
const loadingSpinner = document.getElementById('loading-spinner');
const recentFilesDropdown = document.getElementById('recent-files-dropdown');

// Recent files management
const MAX_RECENT_FILES = 10;
let recentFiles = JSON.parse(localStorage.getItem('recentFiles') || '[]');

function addToRecentFiles(path) {
  recentFiles = recentFiles.filter(f => f.path !== path);
  recentFiles.unshift({
    path: path,
    name: path.split(/[/\\]/).pop(),
    timestamp: Date.now()
  });
  recentFiles = recentFiles.slice(0, MAX_RECENT_FILES);
  localStorage.setItem('recentFiles', JSON.stringify(recentFiles));
  updateRecentFilesDropdown();
}

function updateRecentFilesDropdown() {
  if (recentFiles.length === 0) {
    recentFilesDropdown.innerHTML = `
      <div class="recent-file-empty">
        <i class="ph ph-clock" style="font-size: 24px; margin-bottom: 8px;"></i>
        <div>No recent files</div>
      </div>
    `;
    return;
  }

  recentFilesDropdown.innerHTML = recentFiles.map(file => `
    <div class="recent-file-item" data-path="${file.path}">
      <i class="ph ph-file-pdf recent-file-icon"></i>
      <div class="recent-file-info">
        <div class="recent-file-name">${file.name}</div>
        <div class="recent-file-path">${file.path}</div>
      </div>
    </div>
  `).join('');

  recentFilesDropdown.querySelectorAll('.recent-file-item').forEach(item => {
    item.addEventListener('click', async () => {
      const path = item.getAttribute('data-path');
      await openNewTab(path);
      recentFilesDropdown.classList.remove('visible');
    });
  });
}

// Toast notifications
function showToast(message, type = 'info', duration = 3000) {
  const container = document.getElementById('toast-container');
  const toast = document.createElement('div');
  toast.className = `toast ${type}`;

  const icon = type === 'success' ? 'ph-check-circle' :
    type === 'error' ? 'ph-x-circle' : 'ph-info';

  toast.innerHTML = `
    <i class="ph ${icon}"></i>
    <span>${message}</span>
  `;

  container.appendChild(toast);

  setTimeout(() => {
    toast.classList.add('hide');
    setTimeout(() => toast.remove(), 300);
  }, duration);
}

// Status bar updates
function updateStatusBar() {
  const statusDoc = document.querySelector('#status-doc span');
  const statusPages = document.getElementById('status-pages');
  const statusDimensions = document.getElementById('status-dimensions');

  if (currentDoc) {
    const fileName = currentDoc.split(/[/\\]/).pop();
    statusDoc.textContent = fileName;
    statusPages.textContent = `${totalPages} pages`;
    statusDimensions.textContent = `${canvas.width}×${canvas.height}px`;
  } else {
    statusDoc.textContent = 'No document open';
    statusPages.textContent = '0 pages';
    statusDimensions.textContent = '-';
  }
}

function setStatusMessage(message) {
  const statusMessage = document.querySelector('#status-message span');
  statusMessage.textContent = message;
}

// Loading and skeleton states
function showLoading(text = 'Loading...') {
  loadingSpinner.querySelector('.loading-text').textContent = text;
  loadingSpinner.classList.remove('hidden');
}

function hideLoading() {
  loadingSpinner.classList.add('hidden');
}

function showSkeleton() {
  const container = document.getElementById('viewer-container');
  container.classList.add('skeleton-loading');
}

function hideSkeleton() {
  const container = document.getElementById('viewer-container');
  container.classList.remove('skeleton-loading');
}

// LRU Cache
function getCachedPage(pageNum) {
  return pageCache.get(pageNum);
}

function setCachedPage(pageNum, imageData) {
  if (pageCache.size >= MAX_CACHE_SIZE) {
    const firstKey = pageCache.keys().next().value;
    pageCache.delete(firstKey);
  }
  pageCache.set(pageNum, imageData);
}

// History management for undo/redo
function saveState() {
  const state = {
    annotations: new Map(annotations),
    currentPage,
    currentZoom,
    timestamp: Date.now()
  };

  // Remove any states after current index
  history = history.slice(0, historyIndex + 1);

  // Add new state
  history.push(state);

  // Limit history size
  if (history.length > MAX_HISTORY_SIZE) {
    history.shift();
  } else {
    historyIndex++;
  }

  updateUndoRedoButtons();
  autoSaveAnnotations();
}

function undo() {
  if (historyIndex > 0) {
    historyIndex--;
    restoreState(history[historyIndex]);
    showToast('Undo successful');
  }
}

function redo() {
  if (historyIndex < history.length - 1) {
    historyIndex++;
    restoreState(history[historyIndex]);
    showToast('Redo successful');
  }
}

function restoreState(state) {
  annotations = new Map(state.annotations);
  currentPage = state.currentPage;
  currentZoom = state.currentZoom;
  renderPage(currentPage, false);
  updateUndoRedoButtons();
}

function updateUndoRedoButtons() {
  const undoBtn = document.getElementById('btn-undo');
  const redoBtn = document.getElementById('btn-redo');

  if (undoBtn) {
    undoBtn.disabled = historyIndex <= 0;
    undoBtn.style.opacity = historyIndex <= 0 ? '0.5' : '1';
  }
  if (redoBtn) {
    redoBtn.disabled = historyIndex >= history.length - 1;
    redoBtn.style.opacity = historyIndex >= history.length - 1 ? '0.5' : '1';
  }
}

// Auto-save annotations
function autoSaveAnnotations() {
  if (!currentDoc) return;

  const saveData = {
    document: currentDoc,
    annotations: Array.from(annotations.entries()),
    timestamp: Date.now()
  };

  const savedAnnotations = JSON.parse(localStorage.getItem('pdfAnnotations') || '{}');
  savedAnnotations[currentDoc] = saveData;
  localStorage.setItem('pdfAnnotations', JSON.stringify(savedAnnotations));
}

function loadAnnotations() {
  if (!currentDoc) return;

  const savedAnnotations = JSON.parse(localStorage.getItem('pdfAnnotations') || '{}');
  const docAnnotations = savedAnnotations[currentDoc];

  if (docAnnotations) {
    annotations = new Map(docAnnotations.annotations);
    showToast('Annotations loaded');
  }
}

// Tab management
async function openNewTab(path) {
  const tabId = `tab-${++tabCounter}`;

  try {
    showLoading('Opening PDF...');
    const pages = await invoke('open_document', { path });

    openDocuments.set(tabId, {
      id: tabId,
      path: path,
      name: path.split(/[/\\]/).pop(),
      totalPages: pages,
      currentPage: 0,
      zoom: 1.0
    });

    addToRecentFiles(path);
    createTabUI(tabId, openDocuments.get(tabId));
    switchToTab(tabId);
    hideLoading();
  } catch (e) {
    console.error('Failed to open document:', e);
    showToast('Error opening PDF: ' + e, 'error');
    hideLoading();
  }
}

function createTabUI(tabId, docInfo) {
  const tabsContainer = document.getElementById('tabs-container');
  const tab = document.createElement('div');
  tab.className = 'tab';
  tab.id = tabId;
  tab.innerHTML = `
    <i class="ph ph-file-pdf"></i>
    <span class="tab-title">${docInfo.name}</span>
    <button class="tab-close" data-tab="${tabId}">
      <i class="ph ph-x"></i>
    </button>
  `;

  tab.addEventListener('click', (e) => {
    if (!e.target.closest('.tab-close')) {
      switchToTab(tabId);
    }
  });

  tab.querySelector('.tab-close').addEventListener('click', () => {
    closeTab(tabId);
  });

  tabsContainer.appendChild(tab);
}

function switchToTab(tabId) {
  if (!openDocuments.has(tabId)) return;

  // Save current state
  if (activeTabId) {
    const currentDoc = openDocuments.get(activeTabId);
    if (currentDoc) {
      currentDoc.currentPage = currentPage;
      currentDoc.zoom = currentZoom;
    }
  }

  // Switch to new tab
  activeTabId = tabId;
  const doc = openDocuments.get(tabId);

  currentDoc = doc.path;
  totalPages = doc.totalPages;
  currentPage = doc.currentPage;
  currentZoom = doc.zoom;

  // Update UI
  document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
  document.getElementById(tabId).classList.add('active');

  pageCache.clear();
  loadAnnotations();
  renderPage(currentPage);
}

function closeTab(tabId) {
  const tab = document.getElementById(tabId);
  if (tab) tab.remove();

  openDocuments.delete(tabId);

  if (activeTabId === tabId) {
    const remainingTabs = Array.from(openDocuments.keys());
    if (remainingTabs.length > 0) {
      switchToTab(remainingTabs[0]);
    } else {
      currentDoc = null;
      totalPages = 0;
      currentPage = 0;
      annotations.clear();
      history = [];
      historyIndex = -1;
      canvas.width = 0;
      canvas.height = 0;
      updateUI();
      updateStatusBar();
    }
  }
}

// Annotation tools
function setTool(tool) {
  currentTool = tool;

  // Update UI
  document.querySelectorAll('.tool-btn[data-tool]').forEach(btn => {
    btn.classList.remove('active');
  });

  const activeBtn = document.querySelector(`[data-tool="${tool}"]`);
  if (activeBtn) activeBtn.classList.add('active');

  // Set cursor
  const cursors = {
    'view': 'default',
    'highlight': 'crosshair',
    'rectangle': 'crosshair',
    'circle': 'crosshair',
    'line': 'crosshair',
    'arrow': 'crosshair',
    'text': 'text',
    'sticky': 'pointer'
  };

  canvas.style.cursor = cursors[tool] || 'default';

  if (tool !== 'view') {
    showToast(`${tool.charAt(0).toUpperCase() + tool.slice(1)} tool selected`);
  }
}

function addAnnotation(type, data) {
  const pageAnnotations = annotations.get(currentPage) || [];
  const annotation = {
    id: Date.now().toString(),
    type,
    layer: currentLayer,
    ...data
  };

  pageAnnotations.push(annotation);
  annotations.set(currentPage, pageAnnotations);

  saveState();
  drawAnnotations();
}

function drawAnnotations() {
  const pageAnnotations = annotations.get(currentPage) || [];

  pageAnnotations.forEach(ann => {
    if (!visibleLayers.has(ann.layer)) return;

    ctx.save();

    switch (ann.type) {
      case 'highlight':
        ctx.fillStyle = ann.color + '4D'; // 30% opacity
        ctx.fillRect(ann.x, ann.y, ann.w, ann.h);
        break;

      case 'rectangle':
        ctx.strokeStyle = ann.color;
        ctx.lineWidth = 2;
        ctx.strokeRect(ann.x, ann.y, ann.w, ann.h);
        break;

      case 'circle':
        ctx.strokeStyle = ann.color;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.ellipse(
          ann.x + ann.w / 2,
          ann.y + ann.h / 2,
          ann.w / 2,
          ann.h / 2,
          0, 0, 2 * Math.PI
        );
        ctx.stroke();
        break;

      case 'line':
        ctx.strokeStyle = ann.color;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(ann.x1, ann.y1);
        ctx.lineTo(ann.x2, ann.y2);
        ctx.stroke();
        break;

      case 'arrow':
        drawArrow(ann.x1, ann.y1, ann.x2, ann.y2, ann.color);
        break;

      case 'text':
        ctx.fillStyle = ann.color;
        ctx.font = '16px Inter, sans-serif';
        ctx.fillText(ann.text, ann.x, ann.y);
        break;

      case 'sticky':
        drawStickyNote(ann.x, ann.y, ann.text, ann.color);
        break;
    }

    ctx.restore();
  });
}

function drawArrow(x1, y1, x2, y2, color) {
  const headlen = 15;
  const angle = Math.atan2(y2 - y1, x2 - x1);

  ctx.strokeStyle = color;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(x1, y1);
  ctx.lineTo(x2, y2);
  ctx.stroke();

  ctx.beginPath();
  ctx.moveTo(x2, y2);
  ctx.lineTo(x2 - headlen * Math.cos(angle - Math.PI / 6), y2 - headlen * Math.sin(angle - Math.PI / 6));
  ctx.lineTo(x2 - headlen * Math.cos(angle + Math.PI / 6), y2 - headlen * Math.sin(angle + Math.PI / 6));
  ctx.closePath();
  ctx.fillStyle = color;
  ctx.fill();
}

function drawStickyNote(x, y, text, color) {
  const width = 150;
  const height = 100;

  ctx.fillStyle = color || '#ffeb3b';
  ctx.fillRect(x, y, width, height);

  ctx.strokeStyle = '#000';
  ctx.lineWidth = 1;
  ctx.strokeRect(x, y, width, height);

  ctx.fillStyle = '#000';
  ctx.font = '12px Inter, sans-serif';

  const words = text.split(' ');
  let line = '';
  let lineY = y + 20;

  words.forEach(word => {
    const testLine = line + word + ' ';
    const metrics = ctx.measureText(testLine);

    if (metrics.width > width - 10 && line !== '') {
      ctx.fillText(line, x + 5, lineY);
      line = word + ' ';
      lineY += 16;
    } else {
      line = testLine;
    }
  });

  ctx.fillText(line, x + 5, lineY);
}

// Export functionality
async function exportPageAsImage() {
  if (!currentDoc) {
    showToast('No document open', 'error');
    return;
  }

  try {
    const savePath = await save({
      filters: [{
        name: 'PNG Image',
        extensions: ['png']
      }],
      defaultPath: `page_${currentPage + 1}.png`
    });

    if (savePath) {
      showLoading('Exporting...');

      // Create temporary canvas with annotations
      const exportCanvas = document.createElement('canvas');
      exportCanvas.width = canvas.width;
      exportCanvas.height = canvas.height;
      const exportCtx = exportCanvas.getContext('2d');

      // Copy base image
      exportCtx.drawImage(canvas, 0, 0);

      // Add annotations
      const pageAnnotations = annotations.get(currentPage) || [];
      pageAnnotations.forEach(ann => {
        // Redraw annotations on export canvas
      });

      // Convert to blob and save
      exportCanvas.toBlob(async (blob) => {
        const reader = new FileReader();
        reader.onload = async () => {
          const arrayBuffer = reader.result;
          await invoke('save_file', { path: savePath, data: Array.from(new Uint8Array(arrayBuffer)) });
          hideLoading();
          showToast('Page exported successfully', 'success');
        };
        reader.readAsArrayBuffer(blob);
      }, 'image/png');
    }
  } catch (e) {
    console.error('Export failed:', e);
    hideLoading();
    showToast('Export failed', 'error');
  }
}

async function exportText() {
  if (!currentDoc) {
    showToast('No document open', 'error');
    return;
  }

  try {
    showLoading('Extracting text...');
    const text = await invoke('get_page_text', { pageNum: currentPage });

    const savePath = await save({
      filters: [{
        name: 'Text File',
        extensions: ['txt']
      }],
      defaultPath: `page_${currentPage + 1}.txt`
    });

    if (savePath) {
      await invoke('save_file', { path: savePath, data: Array.from(new TextEncoder().encode(text)) });
      showToast('Text exported successfully', 'success');
    }
    hideLoading();
  } catch (e) {
    console.error('Text export failed:', e);
    hideLoading();
    showToast('Text export failed', 'error');
  }
}

// Batch processing
let selectedPages = new Set();
let batchMode = false;

function toggleBatchMode() {
  batchMode = !batchMode;
  const btn = document.getElementById('btn-batch');

  if (batchMode) {
    btn.classList.add('active');
    showToast('Batch mode enabled. Select pages from sidebar.');
    renderBatchControls();
  } else {
    btn.classList.remove('active');
    selectedPages.clear();
    hideBatchControls();
  }
}

function renderBatchControls() {
  // Add batch operation buttons
  const sidebar = document.getElementById('sidebar');
  const existing = document.getElementById('batch-controls');
  if (existing) existing.remove();

  const controls = document.createElement('div');
  controls.id = 'batch-controls';
  controls.className = 'batch-controls';
  controls.innerHTML = `
    <div class="sidebar-header">Batch Operations (${selectedPages.size} selected)</div>
    <div class="tools-container">
      <button class="tool-btn" id="btn-batch-export">
        <i class="ph ph-export"></i>
        <span>Export Selected</span>
      </button>
      <button class="tool-btn" id="btn-batch-delete">
        <i class="ph ph-trash"></i>
        <span>Delete Selected</span>
      </button>
      <button class="tool-btn" id="btn-batch-clear">
        <i class="ph ph-x"></i>
        <span>Clear Selection</span>
      </button>
    </div>
  `;

  sidebar.insertBefore(controls, sidebar.firstChild);
}

function hideBatchControls() {
  const controls = document.getElementById('batch-controls');
  if (controls) controls.remove();
}

// Rendering
async function renderPage(pageNum, saveHistory = false) {
  if (pageNum < 0 || pageNum >= totalPages) return;

  // Update UI immediately for responsiveness
  document.getElementById('page-indicator').textContent = `${pageNum + 1} / ${totalPages}`;

  const cached = getCachedPage(pageNum);
  if (cached && currentZoom === cached.zoom) {
    displayCachedPage(cached, pageNum);
    prefetchPages(pageNum);
    return;
  }

  try {
    setStatusMessage('Rendering...');
    // Do NOT show blocking loader/skeleton to prevent flicker

    const requestId = Date.now();
    currentRenderRequest = requestId;

    const imageBytes = await invoke('render_page', {
      pageNum: pageNum,
      scale: currentZoom
    });

    // Ignore if a newer render request started
    if (currentRenderRequest !== requestId) return;

    const blob = new Blob([new Uint8Array(imageBytes)], { type: 'image/png' });
    const url = URL.createObjectURL(blob);

    const img = new Image();
    img.onload = () => {
      // Check again before drawing
      if (currentRenderRequest !== requestId) return;

      canvas.width = img.width;
      canvas.height = img.height;
      ctx.drawImage(img, 0, 0);

      setCachedPage(pageNum, {
        imageData: url,
        zoom: currentZoom,
        width: img.width,
        height: img.height
      });

      drawAnnotations();

      updateUI();
      updateStatusBar();
      setStatusMessage('Ready');

      if (saveHistory) {
        saveState();
      }

      currentPage = pageNum;
      prefetchPages(pageNum);
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      setStatusMessage('Error');
      showToast('Failed to render page', 'error');
    };
    img.src = url;

    // Optimistic update of current page variable
    currentPage = pageNum;
  } catch (e) {
    console.error('Failed to render page:', e);
    setStatusMessage('Error');
  }
}

async function prefetchPages(centerPage) {
  const pagesToPrefetch = [centerPage + 1, centerPage - 1];

  for (const pageNum of pagesToPrefetch) {
    if (pageNum >= 0 && pageNum < totalPages) {
      if (!getCachedPage(pageNum)) {
        try {
          // Prefetch silently
          const imageBytes = await invoke('render_page', {
            pageNum: pageNum,
            scale: currentZoom
          });
          const blob = new Blob([new Uint8Array(imageBytes)], { type: 'image/png' });
          const url = URL.createObjectURL(blob);
          const img = new Image();
          img.onload = () => {
            setCachedPage(pageNum, {
              imageData: url,
              zoom: currentZoom,
              width: img.width,
              height: img.height
            });
          };
          img.src = url;
        } catch (e) {
          // Silent fail for prefetch
        }
      }
    }
  }
}

function displayCachedPage(cached, pageNum) {
  const img = new Image();
  img.onload = () => {
    canvas.width = img.width;
    canvas.height = img.height;
    ctx.drawImage(img, 0, 0);
    drawAnnotations();
    currentPage = pageNum;
    updateUI();
    updateStatusBar();
    hideSkeleton();
  };
  img.src = cached.imageData;
}

function updateUI() {
  pageIndicator.textContent = `${currentPage + 1} / ${totalPages}`;
  zoomIndicator.textContent = `${Math.round(currentZoom * 100)}%`;
}

// Canvas event handlers
let tempCanvas = null;
let tempCtx = null;

canvas.addEventListener('mousedown', (e) => {
  if (currentTool === 'view') return;

  isDrawing = true;
  const rect = canvas.getBoundingClientRect();
  startX = e.clientX - rect.left;
  startY = e.clientY - rect.top;

  // Create temporary canvas for preview
  tempCanvas = document.createElement('canvas');
  tempCanvas.width = canvas.width;
  tempCanvas.height = canvas.height;
  tempCtx = tempCanvas.getContext('2d');
  tempCtx.drawImage(canvas, 0, 0);
});

canvas.addEventListener('mousemove', (e) => {
  if (!isDrawing || !tempCtx) return;

  const rect = canvas.getBoundingClientRect();
  const currentX = e.clientX - rect.left;
  const currentY = e.clientY - rect.top;

  // Restore from temp canvas
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.drawImage(tempCanvas, 0, 0);

  // Draw preview based on tool
  ctx.strokeStyle = selectedColor;
  ctx.fillStyle = selectedColor + '4D';
  ctx.lineWidth = 2;

  const width = currentX - startX;
  const height = currentY - startY;

  switch (currentTool) {
    case 'highlight':
      ctx.fillRect(startX, startY, width, height);
      break;
    case 'rectangle':
      ctx.strokeRect(startX, startY, width, height);
      break;
    case 'circle':
      ctx.beginPath();
      ctx.ellipse(startX + width / 2, startY + height / 2, Math.abs(width / 2), Math.abs(height / 2), 0, 0, 2 * Math.PI);
      ctx.stroke();
      break;
    case 'line':
      ctx.beginPath();
      ctx.moveTo(startX, startY);
      ctx.lineTo(currentX, currentY);
      ctx.stroke();
      break;
    case 'arrow':
      drawArrow(startX, startY, currentX, currentY, selectedColor);
      break;
  }
});

canvas.addEventListener('mouseup', (e) => {
  if (!isDrawing) return;

  isDrawing = false;
  const rect = canvas.getBoundingClientRect();
  const endX = e.clientX - rect.left;
  const endY = e.clientY - rect.top;

  const data = {
    color: selectedColor,
    x: Math.min(startX, endX),
    y: Math.min(startY, endY),
    w: Math.abs(endX - startX),
    h: Math.abs(endY - startY)
  };

  switch (currentTool) {
    case 'highlight':
      addAnnotation('highlight', data);
      break;
    case 'rectangle':
      addAnnotation('rectangle', data);
      break;
    case 'circle':
      addAnnotation('circle', data);
      break;
    case 'line':
      addAnnotation('line', { ...data, x1: startX, y1: startY, x2: endX, y2: endY });
      break;
    case 'arrow':
      addAnnotation('arrow', { ...data, x1: startX, y1: startY, x2: endX, y2: endY });
      break;
    case 'text':
      const text = prompt('Enter text:');
      if (text) {
        addAnnotation('text', { ...data, text, x: startX, y: startY + 16 });
      }
      break;
    case 'sticky':
      const note = prompt('Enter note:');
      if (note) {
        addAnnotation('sticky', { ...data, text: note, x: startX, y: startY });
      }
      break;
  }

  tempCanvas = null;
  tempCtx = null;
});

// Keyboard shortcuts
document.addEventListener('keydown', (e) => {
  if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
    if (e.key === 'Escape') e.target.blur();
    return;
  }

  if (e.ctrlKey) {
    switch (e.key) {
      case 'z':
        e.preventDefault();
        undo();
        return;
      case 'y':
        e.preventDefault();
        redo();
        return;
      case 'o':
        e.preventDefault();
        openDocumentWithDialog();
        return;
      case 'f':
        e.preventDefault();
        document.getElementById('ipt-search').focus();
        return;
      case 'b':
        e.preventDefault();
        document.getElementById('sidebar').classList.toggle('collapsed');
        return;
      case '+':
      case '=':
        e.preventDefault();
        currentZoom *= 1.25;
        renderPage(currentPage);
        return;
      case '-':
        e.preventDefault();
        currentZoom /= 1.25;
        renderPage(currentPage);
        return;
      case '0':
        e.preventDefault();
        currentZoom = 1.0;
        renderPage(currentPage);
        return;
      case 's':
        e.preventDefault();
        autoSaveAnnotations();
        showToast('Annotations saved');
        return;
      case 'e':
        e.preventDefault();
        exportPageAsImage();
        return;
    }
  }

  switch (e.key) {
    case 'ArrowLeft':
    case 'PageUp':
      e.preventDefault();
      renderPage(currentPage - 1);
      break;
    case 'ArrowRight':
    case 'PageDown':
    case ' ':
      e.preventDefault();
      renderPage(currentPage + 1);
      break;
    case 'Home':
      e.preventDefault();
      renderPage(0);
      break;
    case 'End':
      e.preventDefault();
      renderPage(totalPages - 1);
      break;
    case 'h':
    case 'H':
      setTool('highlight');
      break;
    case 'r':
    case 'R':
      setTool('rectangle');
      break;
    case 'c':
    case 'C':
      setTool('circle');
      break;
    case 'l':
    case 'L':
      setTool('line');
      break;
    case 'a':
    case 'A':
      setTool('arrow');
      break;
    case 't':
    case 'T':
      setTool('text');
      break;
    case 'n':
    case 'N':
      setTool('sticky');
      break;
    case 'Escape':
      setTool('view');
      break;
    case 'F11':
      e.preventDefault();
      toggleFullscreen();
      break;
  }
});

// Fullscreen toggle
function toggleFullscreen() {
  if (!document.fullscreenElement) {
    document.documentElement.requestFullscreen().then(() => {
      showToast('Fullscreen mode enabled');
    }).catch(e => {
      showToast('Fullscreen not supported', 'error');
    });
  } else {
    document.exitFullscreen();
    showToast('Fullscreen mode disabled');
  }
}

// Filter management
function setFilter(filterName) {
  const grayBtn = document.getElementById('btn-filter-gray');
  const invertBtn = document.getElementById('btn-filter-invert');

  if (activeFilter === filterName) {
    activeFilter = null;
    canvas.style.filter = 'none';
    grayBtn.setAttribute('data-filter', 'none');
    invertBtn.setAttribute('data-filter', 'none');
    showToast('Filter disabled');
  } else {
    activeFilter = filterName;
    canvas.style.filter = filterName === 'grayscale' ? 'grayscale(100%)' : 'invert(100%)';

    if (filterName === 'grayscale') {
      grayBtn.setAttribute('data-filter', 'grayscale');
      invertBtn.setAttribute('data-filter', 'none');
    } else if (filterName === 'invert') {
      grayBtn.setAttribute('data-filter', 'none');
      invertBtn.setAttribute('data-filter', 'invert');
    }
    showToast(`${filterName === 'grayscale' ? 'Greyscale' : 'Invert'} filter enabled`);
  }
}

// Layer management
function toggleLayer(layerName) {
  if (visibleLayers.has(layerName)) {
    visibleLayers.delete(layerName);
  } else {
    visibleLayers.add(layerName);
  }
  renderPage(currentPage);
  showToast(`Layer "${layerName}" ${visibleLayers.has(layerName) ? 'shown' : 'hidden'}`);
}

function createLayer(layerName) {
  visibleLayers.add(layerName);
  currentLayer = layerName;
  showToast(`Layer "${layerName}" created`);
}

// Drag and Drop support
const viewerContainer = document.getElementById('viewer-container');

document.addEventListener('dragover', (e) => {
  e.preventDefault();
  viewerContainer.classList.add('drag-over');
});

document.addEventListener('dragleave', (e) => {
  if (e.target === document.body || e.target === viewerContainer) {
    viewerContainer.classList.remove('drag-over');
  }
});

document.addEventListener('drop', async (e) => {
  e.preventDefault();
  viewerContainer.classList.remove('drag-over');

  const files = e.dataTransfer.files;
  if (files.length === 0) return;

  const file = files[0];

  if (file.name.toLowerCase().endsWith('.pdf')) {
    try {
      const arrayBuffer = await file.arrayBuffer();
      const uint8Array = new Uint8Array(arrayBuffer);

      showToast(`Loading ${file.name}...`);

      const docId = await invoke('load_document_from_bytes', {
        fileName: file.name,
        data: Array.from(uint8Array)
      });

      await openNewTabFromDrop(docId, file.name);
      showToast(`Loaded ${file.name}`, 'success');
    } catch (err) {
      console.error('Failed to load dropped file:', err);
      showToast('Failed to load PDF file', 'error');
    }
  } else {
    showToast('Only PDF files are supported', 'error');
  }
});

async function openNewTabFromDrop(docId, fileName) {
  tabCounter++;
  const tabId = `tab-${tabCounter}`;

  openDocuments.set(tabId, {
    id: docId,
    path: fileName,
    name: fileName,
    page: 0,
    totalPages: 0,
    zoom: 1.0,
    cache: new Map()
  });

  try {
    showLoading();
    const pageCount = await invoke('get_page_count', { docId });
    openDocuments.get(tabId).totalPages = pageCount;
    hideLoading();

    switchToTab(tabId);
    updateTabBar();
    addToRecentFiles(fileName);
  } catch (e) {
    console.error('Failed to get page count:', e);
    hideLoading();
    showToast('Error loading PDF', 'error');
  }
}

// Initialize
console.log('PDFbull initializing...');
updateRecentFilesDropdown();
updateStatusBar();
updateUndoRedoButtons();
setStatusMessage('Ready - Drag and drop PDFs or press Ctrl+O to open');

// Backend ping test
invoke('ping').then(res => console.log('Backend response:', res)).catch(err => console.error('Backend ping failed:', err));
invoke('test_pdfium').then(res => console.log('PDFium check:', res)).catch(err => console.error('PDFium check failed:', err));

// Initialize first document if opened directly
async function openDocumentWithDialog() {
  console.log('Open document dialog triggered');
  try {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{
        name: 'PDF Files',
        extensions: ['pdf']
      }, {
        name: 'All Files',
        extensions: ['*']
      }]
    });
    console.log('Dialog selection:', selected);
    if (selected) {
      await openNewTab(selected);
    } else {
      console.log('Dialog cancelled');
    }
  } catch (e) {
    console.error('Failed to open dialog:', e);
    showToast('Error opening file dialog: ' + e, 'error');
  }
}

// Event listeners for UI controls
document.getElementById('btn-open').addEventListener('click', openDocumentWithDialog);
document.getElementById('btn-prev').addEventListener('click', () => renderPage(currentPage - 1));
document.getElementById('btn-next').addEventListener('click', () => renderPage(currentPage + 1));
document.getElementById('btn-zoom-in').addEventListener('click', () => { currentZoom *= 1.25; renderPage(currentPage); });
document.getElementById('btn-zoom-out').addEventListener('click', () => { currentZoom /= 1.25; renderPage(currentPage); });
document.getElementById('btn-reset-zoom').addEventListener('click', () => { currentZoom = 1.0; renderPage(currentPage); showToast('Zoom reset to 100%'); });
document.getElementById('btn-sidebar-toggle').addEventListener('click', () => document.getElementById('sidebar').classList.toggle('collapsed'));
document.getElementById('btn-fullscreen').addEventListener('click', toggleFullscreen);

// Zoom and Navigation
function fitWidth() {
  if (!currentDoc) return;
  const containerWidth = viewerContainer.clientWidth - 40; // Subtract padding
  const canvasWidth = canvas.width / currentZoom;
  currentZoom = containerWidth / canvasWidth;
  renderPage(currentPage);
  showToast(`Fit to width: ${Math.round(currentZoom * 100)}%`);
}

function fitPage() {
  if (!currentDoc) return;
  const containerHeight = viewerContainer.clientHeight - 40;
  const canvasHeight = canvas.height / currentZoom;
  currentZoom = containerHeight / canvasHeight;
  renderPage(currentPage);
  showToast(`Fit to page: ${Math.round(currentZoom * 100)}%`);
}

document.getElementById('btn-fit-width').addEventListener('click', fitWidth);
document.getElementById('btn-fit-page').addEventListener('click', fitPage);

// Mouse wheel for zoom and navigation
viewerContainer.addEventListener('wheel', (e) => {
  if (e.ctrlKey) {
    e.preventDefault();
    const zoomFactor = e.deltaY > 0 ? 0.9 : 1.1;
    currentZoom *= zoomFactor;
    renderPage(currentPage);
  } else {
    // Only change page if scrolling fast or at bounds
    if (Math.abs(e.deltaY) > 50) {
      const direction = e.deltaY > 0 ? 1 : -1;
      const next = currentPage + direction;
      if (next >= 0 && next < totalPages && next !== currentPage) {
        renderPage(next);
      }
    }
  }
}, { passive: false });

// Search
document.getElementById('btn-search').addEventListener('click', async () => {
  const query = document.getElementById('ipt-search').value;
  if (!query) return;

  try {
    showLoading('Searching...');
    const results = await invoke('search_text', { pageNum: currentPage, query });
    hideLoading();
    setStatusMessage(`Found ${results.length} matches`);
    showToast(`Found ${results.length} matches on this page`);
  } catch (e) {
    console.error("Search failed:", e);
    hideLoading();
    showToast('Search failed', 'error');
  }
});

// Filters
document.getElementById('btn-filter-gray').addEventListener('click', () => setFilter('grayscale'));
document.getElementById('btn-filter-invert').addEventListener('click', () => setFilter('invert'));

// Tools
document.getElementById('btn-highlight').addEventListener('click', () => setTool('highlight'));
document.getElementById('btn-undo').addEventListener('click', undo);
document.getElementById('btn-redo').addEventListener('click', redo);

// Auto-save interval
setInterval(() => {
  if (currentDoc && annotations.size > 0) {
    autoSaveAnnotations();
  }
}, 30000); // Auto-save every 30 seconds
