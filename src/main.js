const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.opener || {}; // Use dialog open if available, or just input

let currentPage = 0;
let totalPages = 0;
let currentZoom = 1.0;
let currentDoc = null;

// UI Elements
const canvas = document.getElementById('pdf-canvas');
const ctx = canvas.getContext('2d');
const pageIndicator = document.getElementById('page-indicator');
const zoomIndicator = document.getElementById('zoom-level');

async function loadDocument() {
  try {
    // In actual app, use dialog.open
    // For now, prompt user or use a fixed path for testing if file dialog not set up
    // But tauri-plugin-dialog is standard.
    // Let's assume we can call a Rust command to open dialog if needed, or just specific path.
    // Actually, invoke('open_document', { path }) expects a path strings.

    // We'll use a simple prompt for now as we didn't add dialog plugin, 
    // or better, if the user drags and drops.
    // Let's implement Drag & Drop
    alert("Drag and drop a PDF file window to open it.");
  } catch (e) {
    console.error(e);
  }
}

async function renderPage(pageNum) {
  if (pageNum < 0 || pageNum >= totalPages) return;

  try {
    const base64Image = await invoke('render_page', {
      pageNum: pageNum,
      scale: currentZoom
    });

    const img = new Image();
    img.onload = () => {
      canvas.width = img.width;
      canvas.height = img.height;
      ctx.drawImage(img, 0, 0);
    };
    img.src = 'data:image/png;base64,' + base64Image;

    currentPage = pageNum;
    updateUI();
  } catch (e) {
    console.error('Failed to render page:', e);
  }
}

function updateUI() {
  pageIndicator.textContent = `${currentPage + 1} / ${totalPages}`;
  zoomIndicator.textContent = `${Math.round(currentZoom * 100)}%`;
}

// Event Listeners
document.getElementById('btn-prev').addEventListener('click', () => renderPage(currentPage - 1));
document.getElementById('btn-next').addEventListener('click', () => renderPage(currentPage + 1));

document.getElementById('btn-zoom-in').addEventListener('click', () => {
  currentZoom *= 1.25;
  renderPage(currentPage);
});

document.getElementById('btn-zoom-out').addEventListener('click', () => {
  currentZoom /= 1.25;
  renderPage(currentPage);
});

// Drag and Drop
window.addEventListener('drop', async (e) => {
  e.preventDefault();
  const file = e.dataTransfer.files[0];
  if (file && file.name.endsWith('.pdf')) {
    // We rely on Tauri's file drop event or just path if available.
    // Actually, standard web file API gives us a File object, but we need the path for Rust.
    // In Tauri, drag-drop often needs specific handling or permission.
    // BUT, we can use the `tauri` event for file-drop.

    // Simplest for now: invoke a command that uses a file dialog
    // OR: just rely on the user typing the path if we lack the dialog plugin setup.
    // Let's try to get path. `file.path` is available in Electron/Tauri often? 
    // No, security.

    // Okay, let's use the 'tauri://file-drop' event if possible.
    // Or just "Open PDF" button invoking a file picker.
  }
});

// Prevent default drag behaviors
window.addEventListener('dragover', (e) => e.preventDefault());

// Button Open
document.getElementById('btn-open').addEventListener('click', async () => {
  // Since we didn't add tauri-plugin-dialog, we might be limited.
  // We can add it easily or simply ask user for path.
  const path = prompt("Enter full path to PDF:");
  if (path) {
    try {
      const pages = await invoke('open_document', { path });
      totalPages = pages;
      currentPage = 0;
      await renderPage(0);
    } catch (e) {
      alert("Error: " + e);
    }
  }
});

// ... existing code ...

let mode = 'view'; // 'view', 'highlight'
let isDrawing = false;
let startX = 0;
let startY = 0;
let selectionRect = null;

// ... loadDocument ...

async function renderPage(pageNum) {
  if (pageNum < 0 || pageNum >= totalPages) return;

  try {
    // Invoke returns number array (Vec<u8>)
    const imageBytes = await invoke('render_page', {
      pageNum: pageNum,
      scale: currentZoom
    });

    const blob = new Blob([new Uint8Array(imageBytes)], { type: 'image/png' });
    const url = URL.createObjectURL(blob);

    const img = new Image();
    img.onload = () => {
      canvas.width = img.width;
      canvas.height = img.height;
      ctx.drawImage(img, 0, 0);
      URL.revokeObjectURL(url); // Clean up memory

      // Draw selection if any
      if (selectionRect) {
        ctx.fillStyle = 'rgba(255, 255, 0, 0.3)';
        ctx.fillRect(selectionRect.x, selectionRect.y, selectionRect.w, selectionRect.h);
      }
    };
    img.src = url;

    currentPage = pageNum;
    updateUI();
  } catch (e) {
    console.error('Failed to render page:', e);
  }
}

// ... updateUI ...

// Search
document.getElementById('btn-search').addEventListener('click', async () => {
  const query = document.getElementById('ipt-search').value;
  if (!query) return;

  try {
    const results = await invoke('search_text', {
      pageNum: currentPage,
      query: query
    });

    // Draw results
    // Results are (x0, y0, x1, y1) in PDF coordinates. Need to scale to canvas.
    // PDF coords are usually 72 DPI. Canvas is scaled by zoom.
    // We'd need to know original page size or scale factor.
    // For MVP, let's just log them or assume simple scaling if possible.
    console.log('Search results:', results);
    alert(`Found ${results.length} matches on this page.`);

    // Simple visualization
    results.forEach(rect => {
      // rect is [x0, y0, x1, y1]
      // We need to transform these to canvas coords. 
      // This requires knowing the PDF page size vs rendered size.
      // For now, we'll skip precise drawing without that metadata.
    });

  } catch (e) {
    console.error("Search failed:", e);
  }
});

// Filters
document.getElementById('btn-filter-gray').addEventListener('click', () => {
  canvas.style.filter = 'grayscale(100%)'; // Fast CSS preview
});

document.getElementById('btn-filter-invert').addEventListener('click', () => {
  canvas.style.filter = 'invert(100%)';
});

// Auto Crop
document.getElementById('btn-crop').addEventListener('click', async () => {
  try {
    await invoke('auto_crop', { pageNum: currentPage });
    renderPage(currentPage); // Re-render to see changes
  } catch (e) {
    console.error("Auto crop failed:", e);
  }
});

// Highlight Tool
document.getElementById('btn-highlight').addEventListener('click', () => {
  mode = mode === 'view' ? 'highlight' : 'view';
  document.getElementById('btn-highlight').style.background = mode === 'highlight' ? 'var(--accent-color)' : 'transparent';
  canvas.style.cursor = mode === 'highlight' ? 'crosshair' : 'default';
});

// Canvas Interactions
canvas.addEventListener('mousedown', (e) => {
  if (mode === 'highlight') {
    isDrawing = true;
    const rect = canvas.getBoundingClientRect();
    startX = e.clientX - rect.left;
    startY = e.clientY - rect.top;
  }
});

canvas.addEventListener('mousemove', (e) => {
  if (mode === 'highlight' && isDrawing) {
    const rect = canvas.getBoundingClientRect();
    const currentX = e.clientX - rect.left;
    const currentY = e.clientY - rect.top;

    selectionRect = {
      x: Math.min(startX, currentX),
      y: Math.min(startY, currentY),
      w: Math.abs(currentX - startX),
      h: Math.abs(currentY - startY)
    };

    // Redraw to show selection
    // In a real app we'd use a separate layer to avoid re-rendering drawing.
    // For MVP, simple context rect on top (cleared by next render frame ideally)
    // actually we need to repaint the image first to clear old rect
    // renderPage(currentPage); // Too slow for mousemove

    // Just draw on top (artifacts will remain until release)
    // Or render image from cache?
  }
});

canvas.addEventListener('mouseup', async (e) => {
  if (mode === 'highlight' && isDrawing) {
    isDrawing = false;
    if (selectionRect) {
      // Convert selectionRect to PDF coordinates and create annotation
      // invoke('create_highlight', ...);
      console.log('Created highlight at', selectionRect);
      selectionRect = null;
      await renderPage(currentPage); // Clear rect
    }
  }
});

// Scan Forms
document.getElementById('btn-forms').addEventListener('click', async () => {
  try {
    const fields = await invoke('get_form_fields', { pageNum: currentPage });
    if (fields.length === 0) {
      alert("No form fields detected on this page.");
    } else {
      console.log("Form fields:", fields);
      alert(`Detected ${fields.length} fields. Check console for details.`);
    }
  } catch (e) {
    console.error("Form scan failed:", e);
  }
});

// Compress PDF
document.getElementById('btn-compress').addEventListener('click', async () => {
  const outputPath = prompt("Enter output path for compressed PDF:");
  if (outputPath) {
    try {
      await invoke('compress_pdf', { outputPath: outputPath });
      alert("Compression complete!");
    } catch (e) {
      alert("Compression failed: " + e);
    }
  }
});
