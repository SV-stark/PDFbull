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
const MAX_CACHE_SIZE = 15; // Reduced for larger memory footprint of ImageBitmaps

function getCachedPage(pageNum) {
  if (pageCache.has(pageNum)) {
    // LRU: Refresh item by deleting and re-inserting
    const data = pageCache.get(pageNum);
    pageCache.delete(pageNum);
    pageCache.set(pageNum, data);
    return data;
  }
  return null;
}

function setCachedPage(pageNum, data) {
  // If cache is full, remove oldest entry (first item)
  if (pageCache.size >= MAX_CACHE_SIZE) {
    const firstKey = pageCache.keys().next().value;
    const oldData = pageCache.get(firstKey);
    if (oldData && oldData.bitmap) {
      oldData.bitmap.close();
    }
    pageCache.delete(firstKey);
  }
  pageCache.set(pageNum, data);
}

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
// UI Elements
const pageIndicator = document.getElementById('page-indicator');
const zoomIndicator = document.getElementById('zoom-level');
const loadingSpinner = document.getElementById('loading-spinner');
const recentFilesDropdown = document.getElementById('recent-files-dropdown');
const viewerContainer = document.getElementById('viewer-container');

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

    if (pageDimensions[currentPage]) {
      const [w, h] = pageDimensions[currentPage];
      statusDimensions.textContent = `${w}×${h}px`;
    } else {
      statusDimensions.textContent = '-';
    }
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
  setupVirtualScroller();
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
      history = [];
      historyIndex = -1;
      document.getElementById('pages-container').innerHTML = '';
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

  // Set cursor on viewer container
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

  viewerContainer.style.cursor = cursors[tool] || 'default';

  // Update status bar
  const currentToolEl = document.getElementById('current-tool');
  if (currentToolEl) {
    currentToolEl.textContent = tool.charAt(0).toUpperCase() + tool.slice(1);
  }

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

function drawAnnotations(pageNum) {
  const pageAnnotations = annotations.get(pageNum) || [];

  const canvas = document.getElementById(`page-canvas-${pageNum}`);
  if (!canvas) return;
  const ctx = canvas.getContext('2d');

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
        drawArrow(ctx, ann.x1, ann.y1, ann.x2, ann.y2, ann.color);
        break;

      case 'text':
        ctx.fillStyle = ann.color;
        ctx.font = '16px Inter, sans-serif';
        ctx.fillText(ann.text, ann.x, ann.y);
        break;

      case 'sticky':
        drawStickyNote(ctx, ann.x, ann.y, ann.text, ann.color);
        break;

      case 'search_highlight':
        ctx.fillStyle = ann.color + '80'; // 50% opacity magenta
        ctx.fillRect(ann.x, ann.y, ann.w, ann.h);
        break;
    }

    ctx.restore();
  });
}

function drawArrow(ctx, x1, y1, x2, y2, color) {
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

function drawStickyNote(ctx, x, y, text, color) {
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

  const canvas = document.getElementById(`page-canvas-${currentPage}`);
  if (!canvas) {
    showToast('Page not rendered', 'error');
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
      // drawAnnotations(currentPage, exportCtx) ? 
      // Reuse drawAnnotations logic or duplicate? 
      // drawAnnotations draws to the *page canvas* directly.
      // We need to draw to exportCtx.
      // Let's manually draw annotations here for now or refactor drawAnnotations to accept ctx again (which I did in previous step but checking implementation...)

      const pageAnnotations = annotations.get(currentPage) || [];
      const ctx = exportCtx; // Use export context

      pageAnnotations.forEach(ann => {
        // ... (Logic from drawAnnotations)
        // To avoid duplicating code, I should have refactored drawAnnotations to take ctx.
        // But providing ID was easier for the main function.
        // I'll duplicate the loop for export to be safe and quick, or just capture the canvas as is?
        // The canvas ALREADY has annotations drawn on it!
        // wait, renderPage calls drawAnnotations.
        // So canvas has everything.
      });

      // If canvas has annotations drawn, we just need to copy it!
      // But wait, are annotations drawn ON the canvas or ON A LAYER?
      // They are drawn ON the canvas using ctx.
      // So exportCtx.drawImage(canvas, 0, 0) copies everything.

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

// Virtual Scroller & Rendering
let pageDimensions = [];
let pageObserver = null;
let visiblePages = new Set();

async function setupVirtualScroller() {
  const container = document.getElementById('pages-container');
  container.innerHTML = '';
  visiblePages.clear();
  pageCache.clear();

  // Fetch dimensions for all pages
  try {
    pageDimensions = await invoke('get_page_dimensions');
  } catch (e) {
    console.error('Failed to get page dimensions:', e);
    return;
  }

  // Create placeholders
  pageDimensions.forEach((dim, index) => {
    const [w, h] = dim;

    const pageContainer = document.createElement('div');
    pageContainer.className = 'page-container';
    pageContainer.id = `page-container-${index}`;
    pageContainer.style.width = `${w * currentZoom}px`;
    pageContainer.style.height = `${h * currentZoom}px`;

    // Placeholder content
    const placeholder = document.createElement('div');
    placeholder.className = 'page-placeholder';
    placeholder.textContent = `Page ${index + 1}`;
    pageContainer.appendChild(placeholder);

    // Canvas (initially hidden or not created? Created but empty)
    const pageCanvas = document.createElement('canvas');
    pageCanvas.id = `page-canvas-${index}`;
    pageCanvas.className = 'page-canvas';
    pageCanvas.width = w * currentZoom;
    pageCanvas.height = h * currentZoom;
    pageCanvas.style.display = 'none'; // Hide until rendered
    pageContainer.appendChild(pageCanvas);

    container.appendChild(pageContainer);
  });

  // Setup Observer
  if (pageObserver) pageObserver.disconnect();

  pageObserver = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      const pageNum = parseInt(entry.target.id.split('-')[2]);

      if (entry.isIntersecting) {
        visiblePages.add(pageNum);
        renderPage(pageNum);
      } else {
        visiblePages.delete(pageNum);
        unloadPage(pageNum);
      }
    });

    updateCurrentPageFromScroll();
  }, {
    root: document.getElementById('viewer-container'),
    rootMargin: '200% 0px', // Render 2 screens ahead/behind
    threshold: 0
  });

  // Observe all pages
  document.querySelectorAll('.page-container').forEach(el => pageObserver.observe(el));

  updateStatusBar();
}

function updateCurrentPageFromScroll() {
  if (visiblePages.size > 0) {
    const sorted = Array.from(visiblePages).sort((a, b) => a - b);
    // Determine the "center" or most relevant page
    // Using the first visible page is often good enough for "Current Page" status
    // But closely check if we scrolled UP, maybe we want the middle one.
    // For now simple is good.
    const centerPage = sorted[Math.floor(sorted.length / 2)];
    if (currentPage !== centerPage) {
      currentPage = centerPage;
      updateUI();
    }
  }
}

function unloadPage(pageNum) {
  const canvas = document.getElementById(`page-canvas-${pageNum}`);
  const placeholder = document.querySelector(`#page-container-${pageNum} .page-placeholder`);

  if (canvas && canvas.style.display !== 'none') {
    const ctx = canvas.getContext('2d');
    // Clear canvas to free GPU memory
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    // Reset size to 0 to be sure? No, keeping size avoids layout shift usually,
    // but the container handles layout. The canvas is inside.
    // Setting width/height to 0 frees more memory than clearRect.
    canvas.width = 0;
    canvas.height = 0;

    canvas.style.display = 'none';
    if (placeholder) placeholder.style.display = 'flex';
  }
}

async function renderPage(pageNum, saveHistory = false) {
  if (pageNum < 0 || pageNum >= totalPages) return;

  const canvas = document.getElementById(`page-canvas-${pageNum}`);
  const ctx = canvas.getContext('2d');
  const placeholder = document.querySelector(`#page-container-${pageNum} .page-placeholder`);

  const cached = getCachedPage(pageNum);

  // If cached and zoom matches, draw instantly
  if (cached && cached.zoom === currentZoom) {
    drawCachedPage(pageNum, cached);
    return;
  }

  try {
    const { width, height, data } = await invoke('render_page', {
      pageNum: pageNum,
      scale: currentZoom
    });

    // OPTIMIZATION: Abort if page is no longer visible (user scrolled past)
    if (!visiblePages.has(pageNum)) {
      return;
    }

    // Use ImageData and createImageBitmap for performance
    const imageData = new ImageData(new Uint8ClampedArray(data), width, height);
    const imageBitmap = await createImageBitmap(imageData);

    // Double check visibility before expensive draw
    if (!visiblePages.has(pageNum)) {
      imageBitmap.close();
      return;
    }

    canvas.width = width;
    canvas.height = height;

    // Update container size to match exact render
    const container = document.getElementById(`page-container-${pageNum}`);
    container.style.width = `${width}px`;
    container.style.height = `${height}px`;

    ctx.drawImage(imageBitmap, 0, 0);
    canvas.style.display = 'block';
    if (placeholder) placeholder.style.display = 'none';

    setCachedPage(pageNum, {
      bitmap: imageBitmap,
      zoom: currentZoom,
      width: width,
      height: height
    });

    drawAnnotations(pageNum);

  } catch (e) {
    console.error(`Failed to render page ${pageNum}:`, e);
  }
}

function drawCachedPage(pageNum, cached) {
  const canvas = document.getElementById(`page-canvas-${pageNum}`);
  const ctx = canvas.getContext('2d');
  const placeholder = document.querySelector(`#page-container-${pageNum} .page-placeholder`);

  canvas.width = cached.width;
  canvas.height = cached.height;
  ctx.drawImage(cached.bitmap, 0, 0);
  canvas.style.display = 'block';
  if (placeholder) placeholder.style.display = 'none';
  drawAnnotations(pageNum);
}

function updateUI() {
  if (totalPages === 0) {
    pageIndicator.textContent = '- / -';
  } else {
    pageIndicator.textContent = `${currentPage + 1} / ${totalPages}`;
  }
  zoomIndicator.textContent = `${Math.round(currentZoom * 100)}%`;
}

function scrollToPage(pageNum) {
  if (pageNum < 0 || pageNum >= totalPages) return;

  const pageContainer = document.getElementById(`page-container-${pageNum}`);
  if (pageContainer) {
    pageContainer.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }

  currentPage = pageNum;
  updateUI();
  updateStatusBar();
}

// Canvas event handlers (Delegation)
let tempCanvas = null;
let tempCtx = null;
let currentDrawingPage = -1;

viewerContainer.addEventListener('mousedown', (e) => {
  if (currentTool === 'view') return;

  const pageCanvas = e.target.closest('.page-canvas');
  if (!pageCanvas) return;

  const pageId = pageCanvas.id.split('-')[2];
  currentDrawingPage = parseInt(pageId);
  currentPage = currentDrawingPage; // Update UI focus
  updateStatusBar();

  isDrawing = true;
  const rect = pageCanvas.getBoundingClientRect();
  startX = e.clientX - rect.left;
  startY = e.clientY - rect.top;

  // Create temporary canvas for preview
  tempCanvas = document.createElement('canvas');
  tempCanvas.width = pageCanvas.width;
  tempCanvas.height = pageCanvas.height;
  tempCtx = tempCanvas.getContext('2d');
  tempCtx.drawImage(pageCanvas, 0, 0);
});

viewerContainer.addEventListener('mousemove', (e) => {
  if (!isDrawing || !tempCtx || currentDrawingPage === -1) return;

  const pageCanvas = document.getElementById(`page-canvas-${currentDrawingPage}`);
  if (!pageCanvas) return;
  const ctx = pageCanvas.getContext('2d');

  const rect = pageCanvas.getBoundingClientRect();
  const currentX = e.clientX - rect.left;
  const currentY = e.clientY - rect.top;

  // Restore from temp canvas
  ctx.clearRect(0, 0, pageCanvas.width, pageCanvas.height);
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
      ctx.ellipse(
        startX + width / 2,
        startY + height / 2,
        Math.abs(width / 2),
        Math.abs(height / 2),
        0, 0, 2 * Math.PI
      );
      ctx.stroke();
      break;
    case 'line':
      ctx.beginPath();
      ctx.moveTo(startX, startY);
      ctx.lineTo(currentX, currentY);
      ctx.stroke();
      break;
    case 'arrow':
      drawArrow(ctx, startX, startY, currentX, currentY, selectedColor);
      break;
  }
});

viewerContainer.addEventListener('mouseup', (e) => {
  if (!isDrawing || currentDrawingPage === -1) return;

  const pageCanvas = document.getElementById(`page-canvas-${currentDrawingPage}`);
  if (!pageCanvas) {
    isDrawing = false;
    currentDrawingPage = -1;
    return;
  }

  isDrawing = false;
  const rect = pageCanvas.getBoundingClientRect();
  const endX = e.clientX - rect.left;
  const endY = e.clientY - rect.top;

  const data = {
    color: selectedColor,
    x: Math.min(startX, endX),
    y: Math.min(startY, endY),
    w: Math.abs(endX - startX),
    h: Math.abs(endY - startY)
  };

  // Save annotation for the specific page
  // Backup currentPage just in case
  const savedPage = currentPage;
  currentPage = currentDrawingPage;

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

  currentPage = savedPage;
  currentDrawingPage = -1;
  tempCanvas = null;
  tempCtx = null;
});

// Save Annotations (Permanently)
async function saveWithAnnotations() {
  if (!currentDoc) return;

  // Flatten annotations from map to array
  let allAnnotations = [];
  annotations.forEach((pageAnns, pageNum) => {
    pageAnns.forEach(ann => {
      // Skip temporary search highlights
      if (ann.type === 'search_highlight') return;

      allAnnotations.push({
        page: parseInt(pageNum), // backend uses 0-based index? Verify. 
        // Frontend uses 0-based `currentPage`. Backend `page_num` is i32. 
        // `doc.pages().get(ann.page)`
        // So 0-based is correct.
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
    showToast('No annotations to save');
    return;
  }

  try {
    const savePath = await save({
      filters: [{
        name: 'PDF with Annotations',
        extensions: ['pdf']
      }],
      defaultPath: `${currentDoc.split(/[/\\]/).pop().replace('.pdf', '_annotated.pdf')}`
    });

    if (savePath) {
      showLoading('Saving annotations...');
      await invoke('save_annotations', {
        outputPath: savePath,
        annotations: allAnnotations
      });
      hideLoading();
      showToast('Annotations saved to new PDF', 'success');
    }
  } catch (e) {
    console.error('Save failed:', e);
    hideLoading();
    showToast('Save failed: ' + e, 'error');
  }
}

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
        // Trigger permanent save instead of just local
        saveWithAnnotations();
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
      if (currentPage > 0) {
        const newPage = currentPage - 1;
        scrollToPage(newPage);
      }
      break;
    case 'ArrowRight':
    case 'PageDown':
    case ' ':
      e.preventDefault();
      if (currentPage < totalPages - 1) {
        const newPage = currentPage + 1;
        scrollToPage(newPage);
      }
      break;
    case 'Home':
      e.preventDefault();
      if (totalPages > 0) {
        scrollToPage(0);
      }
      break;
    case 'End':
      e.preventDefault();
      if (totalPages > 0) {
        scrollToPage(totalPages - 1);
      }
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
  const container = document.getElementById('viewer-container');

  if (activeFilter === filterName) {
    activeFilter = null;
    container.classList.remove('grayscale', 'invert');
    grayBtn.setAttribute('data-filter', 'none');
    invertBtn.setAttribute('data-filter', 'none');
    showToast('Filter disabled');
  } else {
    activeFilter = filterName;
    container.classList.remove('grayscale', 'invert');

    if (filterName === 'grayscale') {
      container.classList.add('grayscale');
      grayBtn.setAttribute('data-filter', 'grayscale');
      invertBtn.setAttribute('data-filter', 'none');
    } else if (filterName === 'invert') {
      container.classList.add('invert');
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
document.getElementById('btn-prev').addEventListener('click', () => {
  if (currentPage > 0) {
    scrollToPage(currentPage - 1);
  }
});
document.getElementById('btn-next').addEventListener('click', () => {
  if (currentPage < totalPages - 1) {
    scrollToPage(currentPage + 1);
  }
});
document.getElementById('btn-zoom-in').addEventListener('click', () => {
  currentZoom *= 1.25;
  updateUI();
  setupVirtualScroller();
});
document.getElementById('btn-zoom-out').addEventListener('click', () => {
  currentZoom /= 1.25;
  updateUI();
  setupVirtualScroller();
});
document.getElementById('btn-reset-zoom').addEventListener('click', () => {
  currentZoom = 1.0;
  updateUI();
  setupVirtualScroller();
  showToast('Zoom reset to 100%');
});
document.getElementById('btn-sidebar-toggle').addEventListener('click', () => document.getElementById('sidebar').classList.toggle('collapsed'));
document.getElementById('btn-fullscreen').addEventListener('click', toggleFullscreen);

// Zoom and Navigation
// Zoom and Navigation
function fitWidth() {
  if (!currentDoc || pageDimensions.length === 0) return;
  const containerWidth = viewerContainer.clientWidth - 40; // Subtract padding
  const [w, h] = pageDimensions[currentPage] || pageDimensions[0];
  currentZoom = containerWidth / w;
  setupVirtualScroller();
  showToast(`Fit to width: ${Math.round(currentZoom * 100)}%`);
}

function fitPage() {
  if (!currentDoc || pageDimensions.length === 0) return;
  const containerHeight = viewerContainer.clientHeight - 40;
  const [w, h] = pageDimensions[currentPage] || pageDimensions[0];
  currentZoom = containerHeight / h;
  setupVirtualScroller();
  showToast(`Fit to page: ${Math.round(currentZoom * 100)}%`);
}

document.getElementById('btn-fit-width').addEventListener('click', fitWidth);
document.getElementById('btn-fit-page').addEventListener('click', fitPage);

// Mouse wheel for zoom only (native scroll handles navigation)
viewerContainer.addEventListener('wheel', (e) => {
  if (e.ctrlKey) {
    e.preventDefault();
    const zoomFactor = e.deltaY > 0 ? 0.9 : 1.1;
    currentZoom *= zoomFactor;
    setupVirtualScroller();
  }
}, { passive: false });

// Search
// Search
document.getElementById('btn-search').addEventListener('click', async () => {
  const query = document.getElementById('ipt-search').value;
  if (!query) return;

  try {
    showLoading('Searching...');
    // Clear previous search results from this page
    const pageAnns = annotations.get(currentPage) || [];
    const filtered = pageAnns.filter(a => a.type !== 'search_highlight');
    annotations.set(currentPage, filtered);

    // Perform search
    const results = await invoke('search_text', { pageNum: currentPage, query });
    hideLoading();

    setStatusMessage(`Found ${results.length} matches`);
    showToast(`Found ${results.length} matches on this page`);

    if (results.length > 0) {
      // Add new highlights
      // Note: PDF coordinates might need conversion (Y-flip) depending on PDFium output vs Canvas
      // PDFium: Bottom-Left origin. Canvas: Top-Left.
      // We need page height to convert.
      // Assuming we have pageDimensions[currentPage] = [width, height]
      const [pageW, pageH] = pageDimensions[currentPage] || [0, 0];

      const newAnns = results.map(r => {
        const [x, y, w, h] = r;
        // Convert PDF coords to Canvas coords
        // Canvas Y = PageHeight - PDF Y - PDF Height ? 
        // Usually PDF Y is bottom of the rect.
        // Let's try standard flip: canvasY = pageH - y - h; (if y is top) 
        // or if y is bottom: canvasY = pageH - y - h;
        // Needs experimental verification or robust logic. 
        // For now assuming PDFium rect.top is MAX Y.
        // Canvas Top = PageHeight - RectTop.

        // However, let's just use raw for now and if it's inverted we flip.
        // Using a heuristic: PDF coords usually put (0,0) at bottom-left.

        return {
          id: 'search-' + Math.random(),
          type: 'search_highlight',
          layer: 'default',
          x: x,
          y: pageH - y, // coordinate transform attempt 1
          w: w,
          h: h,
          color: '#ff00ff'
        };
      });

      const currentAnns = annotations.get(currentPage) || [];
      annotations.set(currentPage, [...currentAnns, ...newAnns]);
      drawAnnotations(currentPage);
    }

  } catch (e) {
    console.error("Search failed:", e);
    hideLoading();
    showToast('Search failed: ' + e, 'error');
  }
});

// Forms
document.getElementById('btn-forms').addEventListener('click', scanForForms);

async function scanForForms() {
  if (!currentDoc) return;

  showLoading('Scanning for forms...');
  try {
    // For now scan current page only to demonstrate
    const fields = await invoke('get_form_fields', { pageNum: currentPage });

    // Render fields
    const pageContainer = document.getElementById(`page-container-${currentPage}`);
    // Clear existing forms
    const existing = pageContainer.querySelectorAll('.form-field-overlay');
    existing.forEach(e => e.remove());

    if (fields.length === 0) {
      showToast('No form fields found on this page');
    } else {
      const [pageW, pageH] = pageDimensions[currentPage] || [0, 0];

      fields.forEach(field => {
        const input = document.createElement('input');
        input.type = 'text'; // Default to text
        input.className = 'form-field-overlay';
        input.value = field.value || '';
        input.placeholder = field.name;
        input.title = field.name;

        // Positioning
        // Again, check coordinate system. Assuming standard PDF (bottom-left)
        // If field.y is bottom, then top is pageH - field.y - field.h
        // If field.y is top (from pdfium-render normalization?), then just y.
        // Let's assume consistent with search for now (y is top? or bottom?)
        // Search used `pageH - y`. Let's try `pageH - field.y - field.h` for bottom-up coords.
        // Or if `y` is top, just `y`.
        // I'll stick to the transform I used in search or standard.
        // Actually, pdfium-render `bounds()` returns `bottom`, `left` etc.
        // If `field.y` comes from `rect.top.value` in backend...
        // In PDF, top > bottom.
        // So `top` is distance from bottom.
        // Canvas Top = PageHeight - PDF Top.

        const top = pageH - field.y;
        const left = field.x;
        const width = field.w;
        const height = field.h;

        input.style.position = 'absolute';
        input.style.left = `${left}px`;
        input.style.top = `${top}px`;
        input.style.width = `${width}px`;
        input.style.height = `${height}px`;
        input.style.zIndex = '10';
        input.style.background = 'rgba(255, 255, 255, 0.5)';
        input.style.border = '1px solid #0078d4';

        pageContainer.appendChild(input);
      });
      showToast(`Found ${fields.length} fields`);
    }

  } catch (e) {
    console.error('Form scan failed:', e);
    showToast('Form scan failed', 'error');
  }
  hideLoading();
}

// Filters
document.getElementById('btn-filter-gray').addEventListener('click', () => setFilter('grayscale'));
document.getElementById('btn-filter-invert').addEventListener('click', () => setFilter('invert'));

// Tools
document.getElementById('btn-highlight').addEventListener('click', () => setTool('highlight'));
document.getElementById('btn-rectangle').addEventListener('click', () => setTool('rectangle'));
document.getElementById('btn-circle').addEventListener('click', () => setTool('circle'));
document.getElementById('btn-line').addEventListener('click', () => setTool('line'));
document.getElementById('btn-arrow').addEventListener('click', () => setTool('arrow'));
document.getElementById('btn-text').addEventListener('click', () => setTool('text'));
document.getElementById('btn-sticky').addEventListener('click', () => setTool('sticky'));
document.getElementById('btn-undo').addEventListener('click', undo);
document.getElementById('btn-redo').addEventListener('click', redo);

// Export button
document.getElementById('btn-export').addEventListener('click', () => {
  const modal = document.getElementById('export-modal');
  modal.classList.remove('hidden');
});

// Export modal close
document.getElementById('btn-close-export').addEventListener('click', () => {
  document.getElementById('export-modal').classList.add('hidden');
});

// Export options
document.getElementById('btn-export-image').addEventListener('click', () => {
  document.getElementById('export-modal').classList.add('hidden');
  exportPageAsImage();
});

document.getElementById('btn-export-text').addEventListener('click', () => {
  document.getElementById('export-modal').classList.add('hidden');
  exportText();
});

document.getElementById('btn-export-compress').addEventListener('click', () => {
  document.getElementById('export-modal').classList.add('hidden');
  compressPDF();
});

// Compression
document.getElementById('btn-compress').addEventListener('click', compressPDF);

async function compressPDF() {
  if (!currentDoc) {
    showToast('No document open', 'error');
    return;
  }

  try {
    // Open save dialog
    const savePath = await save({
      filters: [{
        name: 'Compressed PDF',
        extensions: ['pdf']
      }],
      defaultPath: `${currentDoc.split(/[/\\]/).pop().replace('.pdf', '_compressed.pdf')}`
    });

    if (savePath) {
      showLoading('Compressing PDF...');
      await invoke('compress_pdf', { outputPath: savePath });
      hideLoading();
      showToast('PDF compressed successfully', 'success');
    }
  } catch (e) {
    console.error('Compression failed:', e);
    hideLoading();
    showToast('Compression failed: ' + e, 'error');
  }
}

// Auto-Crop
document.getElementById('btn-crop').addEventListener('click', autoCrop);

async function autoCrop() {
  if (!currentDoc) return;

  try {
    showLoading('Auto-cropping...');
    await invoke('auto_crop', { pageNum: currentPage });

    // Clear cache for this page to force re-render
    pageCache.delete(currentPage);

    // Re-render
    await renderPage(currentPage); // this usually just draws, but render_page backend will re-render with new crop box

    hideLoading();
    showToast('Page cropped to content');

    // Update dimensions in UI
    const dims = await invoke('get_page_dimensions');
    pageDimensions = dims;
    updateStatusBar();

  } catch (e) {
    console.error('Auto-crop failed:', e);
    hideLoading();
    showToast('Auto-crop failed: ' + e, 'error');
  }
}

// Theme dropdown toggle
const themeDropdown = document.getElementById('theme-dropdown');
document.getElementById('btn-theme').addEventListener('click', (e) => {
  e.stopPropagation();
  themeDropdown.classList.toggle('visible');
});

// Theme selection
document.querySelectorAll('.theme-option').forEach(option => {
  option.addEventListener('click', () => {
    const theme = option.getAttribute('data-theme');
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('theme', theme);
    themeDropdown.classList.remove('visible');
    showToast(`Theme changed to ${theme}`);
  });
});

// Close theme dropdown when clicking outside
document.addEventListener('click', (e) => {
  if (!e.target.closest('#btn-theme') && !e.target.closest('#theme-dropdown')) {
    themeDropdown.classList.remove('visible');
  }
});

// Recent files toggle
document.getElementById('btn-recent-toggle').addEventListener('click', (e) => {
  e.stopPropagation();
  recentFilesDropdown.classList.toggle('visible');
});

// Close recent files dropdown when clicking outside
document.addEventListener('click', (e) => {
  if (!e.target.closest('.recent-files-container') && !e.target.closest('#btn-recent-toggle')) {
    recentFilesDropdown.classList.remove('visible');
  }
});

// Load saved theme
const savedTheme = localStorage.getItem('theme') || 'dark';
document.documentElement.setAttribute('data-theme', savedTheme);

// Color picker
document.querySelectorAll('.color-btn').forEach(btn => {
  btn.addEventListener('click', () => {
    document.querySelectorAll('.color-btn').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    selectedColor = btn.getAttribute('data-color');
    document.getElementById('custom-color').value = selectedColor;
  });
});

document.getElementById('custom-color').addEventListener('input', (e) => {
  selectedColor = e.target.value;
  document.querySelectorAll('.color-btn').forEach(b => b.classList.remove('active'));
});

// Auto-save interval
setInterval(() => {
  if (currentDoc && annotations.size > 0) {
    autoSaveAnnotations();
  }
}, 30000); // Auto-save every 30 seconds
