import { state } from './state.js';
import { api } from './api.js';
import { ui } from './ui.js';
import { settings } from './settings.js';
import { CONSTANTS } from './constants.js';
import { debug } from './debug.js';

export const renderer = {
    // Cache Management - Improved LRU with proper ordering
    getCachedPage(pageNum) {
        const cacheKey = `${pageNum}_${state.currentZoom}`;
        if (state.pageCache.has(cacheKey)) {
            const data = state.pageCache.get(cacheKey);
            return data;
        }
        return null;
    },

    setCachedPage(pageNum, data) {
        const cacheKey = `${pageNum}_${state.currentZoom}`;
        const pageSize = data.width * data.height * 4;

        while (state.currentCacheBytes + pageSize > state.MAX_CACHE_BYTES && state.pageCache.size > 0) {
            const firstKey = state.pageCache.keys().next().value;
            const oldData = state.pageCache.get(firstKey);
            if (oldData) {
                if (oldData.bitmap) {
                    oldData.bitmap.close();
                }
                const oldSize = oldData.width * oldData.height * 4;
                state.currentCacheBytes -= oldSize;
            }
            state.pageCache.delete(firstKey);
        }

        state.pageCache.set(cacheKey, data);
        state.currentCacheBytes += pageSize;
    },

    // Get or create cached canvas context
    getCanvasContext(pageNum, canvasId) {
        const key = `${pageNum}_${canvasId}`;
        if (state.canvasContexts.has(key)) {
            return state.canvasContexts.get(key);
        }
        const canvas = document.getElementById(`${canvasId}-${pageNum}`);
        if (!canvas) return null;
        const ctx = canvas.getContext('2d');
        state.canvasContexts.set(key, ctx);
        return ctx;
    },

    // Clear cached contexts when zoom changes
    clearCanvasContextCache() {
        state.canvasContexts.clear();
    },

    bindScrollEvents() {
        const viewer = document.getElementById('viewer-container');
        const progressBar = document.getElementById('reading-progress-bar');
        if (!viewer || !progressBar) return;

        viewer.addEventListener('scroll', () => {
            const scrollTop = viewer.scrollTop;
            const scrollHeight = viewer.scrollHeight - viewer.clientHeight;
            if (scrollHeight > 0) {
                const progress = (scrollTop / scrollHeight) * 100;
                progressBar.style.width = `${progress}%`;
            }
        });
    },

    // Setup Virtual Scroller
    async setupVirtualScroller() {
        const container = ui.elements.pagesContainer();
        const viewer = ui.elements.viewerContainer();
        if (!container || !viewer) return;

        const scrollRatio = viewer.scrollTop / viewer.scrollHeight;
        const previousHeight = viewer.scrollHeight;
        const existingPages = container.children.length > 0;

        state.visiblePages.clear();

        try {
            state.pageDimensions = await api.getPageDimensions();
        } catch (e) {
            console.error('Failed to get page dimensions:', e);
            return;
        }

        state.pageDimensions.forEach((dim, index) => {
            const [w, h] = dim;
            let pageContainer = /** @type {HTMLDivElement} */ (document.getElementById(`page-container-${index}`));

            if (!pageContainer) {
                pageContainer = document.createElement('div');
                pageContainer.className = 'page-container';
                pageContainer.id = `page-container-${index}`;

                // Placeholder content
                const placeholder = document.createElement('div');
                placeholder.className = 'page-placeholder';
                placeholder.textContent = `Page ${index + 1}`;
                pageContainer.appendChild(placeholder);

                // Canvas Layering: PDF Layer (Bottom)
                const pdfCanvas = document.createElement('canvas');
                pdfCanvas.id = `page-canvas-${index}`;
                pdfCanvas.className = 'page-canvas pdf-layer';
                pdfCanvas.width = w * state.currentZoom;
                pdfCanvas.height = h * state.currentZoom;
                pdfCanvas.style.display = 'none'; // Hidden until rendered
                pageContainer.appendChild(pdfCanvas);

                // Canvas Layering: Annotation Layer (Top)
                const annCanvas = document.createElement('canvas');
                annCanvas.id = `ann-canvas-${index}`;
                annCanvas.className = 'page-canvas annotation-layer';
                annCanvas.width = w * state.currentZoom;
                annCanvas.height = h * state.currentZoom;
                pageContainer.appendChild(annCanvas);

                // Text Selection Layer (Topmost)
                const textLayer = document.createElement('div');
                textLayer.id = `text-layer-${index}`;
                textLayer.className = 'text-layer';
                pageContainer.appendChild(textLayer);

                container.appendChild(pageContainer);
            }

            const pdfCanvas = /** @type {HTMLCanvasElement} */ (document.getElementById(`page-canvas-${index}`));
            const annCanvas = /** @type {HTMLCanvasElement} */ (document.getElementById(`ann-canvas-${index}`));

            pageContainer.style.setProperty('--page-width', `${w}px`);
            pageContainer.style.setProperty('--page-height', `${h}px`);

            if (pdfCanvas && pdfCanvas.width !== w * state.renderScale) {
                pdfCanvas.width = w * state.renderScale;
                pdfCanvas.height = h * state.renderScale;
                pdfCanvas.style.display = 'none';

                if (annCanvas) {
                    annCanvas.width = w * state.renderScale;
                    annCanvas.height = h * state.renderScale;
                }

                renderer.drawAnnotations(index);
            }
        });

        document.getElementById('pages-container').style.setProperty('--zoom-factor', state.currentZoom);

        // Restore scroll position
        if (existingPages && previousHeight > 0) {
            viewer.scrollTop = scrollRatio * viewer.scrollHeight;
        }

        // Setup Observer
        if (state.pageObserver) state.pageObserver.disconnect();

        state.pageObserver = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                const pageNum = parseInt(entry.target.id.split('-')[2]);

                if (entry.isIntersecting) {
                    state.visiblePages.add(pageNum);
                    renderer.renderPage(pageNum);
                } else {
                    state.visiblePages.delete(pageNum);
                    renderer.unloadPage(pageNum);
                }
            });

            renderer.updateCurrentPageFromScroll();
        }, {
            root: viewer,
            rootMargin: '200% 0px',
            threshold: 0
        });

        document.querySelectorAll('.page-container').forEach(el => state.pageObserver.observe(el));

        ui.updateStatusBar();
    },

    scrollToPage(pageNum) {
        const pageContainer = document.getElementById(`page-container-${pageNum}`);
        if (pageContainer) {
            pageContainer.scrollIntoView({ behavior: 'smooth', block: 'start' });
            state.currentPage = pageNum;
            ui.updateUI();
            ui.updateStatusBar();
        }
    },

    updateCurrentPageFromScroll() {
        if (state.visiblePages.size > 0) {
            const sorted = Array.from(state.visiblePages).sort((a, b) => a - b);
            const centerPage = sorted[Math.floor(sorted.length / 2)];
            if (state.currentPage !== centerPage) {
                state.currentPage = centerPage;
                ui.updateUI(); // Assumes ui.updateUI exists but it wasn't strictly in my snippet above. Need to add it or fix. 
                // Wait, I put updateUI in main.js snippet? No, I put updateStatusBar in ui.js
                // I need updateUI in ui.js. I'll add it in next step or now?
                // `updateUI` updates page indicator and zoom indicator.
                // I'll assume ui module has it or I need to add it.
                // Checking ui.js... no `updateUI` there yet, only `updateStatusBar`.
                // I should add `updateMainUI` to ui.js or similar.
                // For now, I'll call `ui.updateStatusBar` + explicit text updates.
                // Actually I should just add it to `ui.js` in a follow up or assume I missed it.
                // Let's call `ui.updatePageIndicators()`?
                ui.updateStatusBar();
                if (ui.elements.pageIndicator()) {
                    if (state.totalPages === 0) {
                        ui.elements.pageIndicator().textContent = '- / -';
                    } else {
                        ui.elements.pageIndicator().textContent = `${state.currentPage + 1} / ${state.totalPages}`;
                    }
                }
                if (ui.elements.zoomIndicator()) {
                    ui.elements.zoomIndicator().textContent = `${Math.round(state.currentZoom * 100)}%`;
                }
            }
        }
    },

    unloadPage(pageNum) {
        const pdfCanvas = /** @type {HTMLCanvasElement} */ (document.getElementById(`page-canvas-${pageNum}`));
        const annCanvas = /** @type {HTMLCanvasElement} */ (document.getElementById(`ann-canvas-${pageNum}`));
        const placeholder = /** @type {HTMLElement} */ (document.querySelector(`#page-container-${pageNum} .page-placeholder`));

        if (pdfCanvas && pdfCanvas.width > 0 && pdfCanvas.height > 0) {
            const ctx = pdfCanvas.getContext('2d');
            if (ctx) {
                ctx.clearRect(0, 0, pdfCanvas.width, pdfCanvas.height);
            }
            pdfCanvas.width = 0;
            pdfCanvas.height = 0;
            pdfCanvas.style.display = 'none';
        }

        if (annCanvas && annCanvas.width > 0 && annCanvas.height > 0) {
            const ctx = annCanvas.getContext('2d');
            if (ctx) {
                ctx.clearRect(0, 0, annCanvas.width, annCanvas.height);
            }
            annCanvas.width = 0;
            annCanvas.height = 0;
        }

        if (placeholder) placeholder.style.display = 'flex';

        // Clear text layer
        const textLayer = document.getElementById(`text-layer-${pageNum}`);
        if (textLayer) textLayer.innerHTML = '';
        
        // Remove from cache
        const cacheKey = `${pageNum}_${state.currentZoom}`;
        const cached = state.pageCache.get(cacheKey);
        if (cached && cached.bitmap) {
            cached.bitmap.close();
            state.currentCacheBytes -= cached.width * cached.height * 4;
            state.pageCache.delete(cacheKey);
        }
        
        debug.log(`Unloaded page ${pageNum}, cache size: ${state.currentCacheBytes} bytes`);
    },

    async renderPage(pageNum) {
        if (pageNum < 0 || pageNum >= state.totalPages) return;

        const requestId = ++state.currentRenderRequest;
        state.pageRenderRequests.set(pageNum, requestId);

        const pdfCanvas = /** @type {HTMLCanvasElement} */ (document.getElementById(`page-canvas-${pageNum}`));
        const annCanvas = /** @type {HTMLCanvasElement} */ (document.getElementById(`ann-canvas-${pageNum}`));
        const pdfCtx = pdfCanvas.getContext('2d');
        const placeholder = /** @type {HTMLElement} */ (document.querySelector(`#page-container-${pageNum} .page-placeholder`));

        const currentZoom = state.currentZoom;
        const cached = renderer.getCachedPage(pageNum);

        if (cached && cached.zoom === currentZoom) {
            debug.log(`Cache hit for page ${pageNum} at zoom ${currentZoom}`);
            renderer.drawCachedPage(pageNum, cached);
            return;
        }

        debug.log(`Rendering page ${pageNum} at zoom ${currentZoom} (cache miss)`);

        try {
            const responseBytes = await api.renderPage(pageNum, currentZoom);

            if (state.pageRenderRequests.get(pageNum) !== requestId) {
                debug.log(`Render cancelled for page ${pageNum}`);
                return;
            }

            const view = new DataView(responseBytes);
            const width = view.getInt32(0, false);
            const height = view.getInt32(4, false);

            const pixels = new Uint8ClampedArray(responseBytes, 8);

            if (!state.visiblePages.has(pageNum)) return;
            if (state.pageRenderRequests.get(pageNum) !== requestId) return;

            const imageData = new ImageData(pixels, width, height);
            const imageBitmap = await createImageBitmap(imageData);

            if (!state.visiblePages.has(pageNum)) {
                imageBitmap.close();
                return;
            }
            if (state.pageRenderRequests.get(pageNum) !== requestId) {
                imageBitmap.close();
                return;
            }

            pdfCanvas.width = width;
            pdfCanvas.height = height;

            if (annCanvas) {
                annCanvas.width = width;
                annCanvas.height = height;
            }

            pdfCtx.drawImage(imageBitmap, 0, 0);
            pdfCanvas.style.display = 'block';
            if (placeholder) placeholder.style.display = 'none';

            renderer.setCachedPage(pageNum, {
                bitmap: imageBitmap,
                zoom: currentZoom,
                width: width,
                height: height
            });

            renderer.drawAnnotations(pageNum);

            renderer.renderTextLayer(pageNum, width, height);

        } catch (e) {
            if (state.pageRenderRequests.get(pageNum) === requestId) {
                debug.error(`Failed to render page ${pageNum}:`, e);
            }
        } finally {
            state.pageRenderRequests.delete(pageNum);
        }
    },

    drawCachedPage(pageNum, cached) {
        const pdfCanvas = /** @type {HTMLCanvasElement} */ (document.getElementById(`page-canvas-${pageNum}`));
        const annCanvas = /** @type {HTMLCanvasElement} */ (document.getElementById(`ann-canvas-${pageNum}`));
        const pdfCtx = pdfCanvas.getContext('2d');
        const placeholder = /** @type {HTMLElement} */ (document.querySelector(`#page-container-${pageNum} .page-placeholder`));

        pdfCanvas.width = cached.width;
        pdfCanvas.height = cached.height;

        if (annCanvas) {
            annCanvas.width = cached.width;
            annCanvas.height = cached.height;
        }

        pdfCtx.drawImage(cached.bitmap, 0, 0);
        pdfCanvas.style.display = 'block';
        if (placeholder) placeholder.style.display = 'none';
        renderer.drawAnnotations(pageNum);
    },

    drawAnnotations(pageNum) {
        const pageAnnotations = state.annotations.get(pageNum) || [];
        const canvas = /** @type {HTMLCanvasElement} */ (document.getElementById(`ann-canvas-${pageNum}`));
        if (!canvas) return;
        const ctx = canvas.getContext('2d');

        ctx.clearRect(0, 0, canvas.width, canvas.height);

        pageAnnotations.forEach(ann => {
            if (!state.visibleLayers.has(ann.layer)) return;

            ctx.save();

            switch (ann.type) {
                case 'highlight':
                    ctx.fillStyle = ann.color + '4D';
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
                    renderer.drawArrow(ctx, ann.x1, ann.y1, ann.x2, ann.y2, ann.color);
                    break;

                case 'text':
                    ctx.fillStyle = ann.color;
                    ctx.font = '16px Inter, sans-serif';
                    ctx.fillText(ann.text, ann.x, ann.y);
                    break;

                case 'sticky':
                    renderer.drawStickyNote(ctx, ann.x, ann.y, ann.text, ann.color);
                    break;

                case 'search_highlight':
                    ctx.fillStyle = ann.color + '80';
                    ctx.fillRect(ann.x, ann.y, ann.w, ann.h);
                    break;
            }

            ctx.restore();
        });
    },

    /**
     * Render transparent text layer for text selection
     * @param {number} pageNum - Page number
     * @param {number} canvasWidth - Rendered canvas width
     * @param {number} canvasHeight - Rendered canvas height
     */
    async renderTextLayer(pageNum, canvasWidth, canvasHeight) {
        const textLayer = /** @type {HTMLElement} */ (document.getElementById(`text-layer-${pageNum}`));
        if (!textLayer) return;

        const zoomKey = String(state.currentZoom);
        
        let pageTextCache = state.textLayerCache.get(pageNum);
        if (!pageTextCache) {
            pageTextCache = new Map();
            state.textLayerCache.set(pageNum, pageTextCache);
        }

        if (pageTextCache.has(zoomKey)) {
            if (textLayer.dataset.rendered === zoomKey) return;
            textLayer.innerHTML = '';
            textLayer.innerHTML = pageTextCache.get(zoomKey);
            textLayer.dataset.rendered = zoomKey;
            return;
        }

        if (textLayer.dataset.rendered === zoomKey) return;

        try {
            const textRects = await api.getPageTextRects(pageNum);

            if (!state.visiblePages.has(pageNum)) return;

            const [origWidth, origHeight] = state.pageDimensions[pageNum] || [0, 0];
            if (origWidth === 0) return;

            const scaleX = canvasWidth / origWidth;
            const scaleY = canvasHeight / origHeight;

            textLayer.innerHTML = '';

            textRects.forEach(rect => {
                if (!rect.text.trim() && rect.text !== ' ') return;

                const span = document.createElement('span');
                span.textContent = rect.text;

                const cssX = rect.x * scaleX;
                const cssY = (origHeight - rect.y) * scaleY;
                const cssW = rect.w * scaleX;
                const cssH = rect.h * scaleY;

                span.style.left = `${cssX}px`;
                span.style.top = `${cssY - cssH}px`;
                span.style.width = `${cssW}px`;
                span.style.height = `${cssH}px`;
                span.style.fontSize = `${cssH * 0.9}px`;

                textLayer.appendChild(span);
            });

            pageTextCache.set(zoomKey, textLayer.innerHTML);
            textLayer.dataset.rendered = zoomKey;

        } catch (e) {
            console.error(`Failed to render text layer for page ${pageNum}:`, e);
        }
    },

    drawArrow(ctx, x1, y1, x2, y2, color) {
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
    },

    drawStickyNote(ctx, x, y, text, color) {
        const s = settings.load();
        const width = s.stickyNoteWidth || 150;
        const height = s.stickyNoteHeight || 100;

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
    },

    // Thumbnails
    async renderThumbnails() {
        const container = document.getElementById('thumbnail-list');
        if (!container) return;
        container.innerHTML = '';

        const pages = [];
        for (let i = 0; i < state.totalPages; i++) {
            const thumb = document.createElement('div');
            thumb.className = 'thumbnail skeleton';
            thumb.dataset.page = String(i);
            thumb.innerHTML = `<span style="pointer-events:none;">${i + 1}</span>`;

            if (i === state.currentPage) thumb.classList.add('active');

            thumb.onclick = () => {
                state.currentPage = i;
                renderer.scrollToPage(i);
                document.querySelectorAll('.thumbnail').forEach(t => t.classList.remove('active'));
                thumb.classList.add('active');
            };
            container.appendChild(thumb);
            pages.push({ index: i, element: thumb });
        }

        const Observer = new IntersectionObserver((entries) => {
            const visibleEntries = entries.filter(e => e.isIntersecting && !e.target.dataset.rendered);
            
            if (visibleEntries.length === 0) return;

            visibleEntries.forEach(entry => {
                entry.target.dataset.rendered = 'true';
            });

            const visiblePages = visibleEntries.map(e => parseInt(e.target.dataset.page));

            Promise.all(visiblePages.map(async (pageNum) => {
                try {
                    const responseBytes = await api.renderPage(pageNum, 0.15);
                    const view = new DataView(responseBytes);

                    const width = view.getInt32(0, false);
                    const height = view.getInt32(4, false);
                    const pixels = new Uint8ClampedArray(responseBytes, 8);

                    const imageData = new ImageData(pixels, width, height);
                    const imageBitmap = await createImageBitmap(imageData);

                    return { pageNum, width, height, imageBitmap };
                } catch (e) {
                    console.error('Thumb render failed', e);
                    return null;
                }
            })).then(results => {
                results.forEach(result => {
                    if (!result) return;
                    const { pageNum, width, height, imageBitmap } = result;
                    
                    const target = container.querySelector(`[data-page="${pageNum}"]`);
                    if (!target) return;

                    const canvas = document.createElement('canvas');
                    canvas.width = width;
                    canvas.height = height;
                    canvas.style.width = '100%';
                    canvas.style.height = 'auto';

                    const ctx = canvas.getContext('2d');
                    ctx.drawImage(imageBitmap, 0, 0);

                    target.innerHTML = '';
                    target.classList.remove('skeleton');
                    target.style.background = 'transparent';
                    target.appendChild(canvas);

                    const num = document.createElement('div');
                    num.textContent = String(pageNum + 1);
                    num.className = 'thumb-num';
                    num.style.position = 'absolute';
                    num.style.bottom = '2px';
                    num.style.right = '2px';
                    num.style.background = 'rgba(0,0,0,0.5)';
                    num.style.color = 'white';
                    num.style.padding = '2px 4px';
                    num.style.borderRadius = '4px';
                    num.style.fontSize = '10px';
                    target.appendChild(num);
                    target.style.position = 'relative';

                    imageBitmap.close();
                });
            });
        }, { root: container, rootMargin: '100px' });

        container.querySelectorAll('.thumbnail').forEach(el => Observer.observe(el));
    }
};
