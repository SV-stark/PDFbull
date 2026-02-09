import { state } from './state.js';
import { api } from './api.js';
import { ui } from './ui.js';
import { CONSTANTS } from './constants.js';

export const renderer = {
    // Cache Management
    getCachedPage(pageNum) {
        if (state.pageCache.has(pageNum)) {
            // LRU: Refresh item by deleting and re-inserting
            const data = state.pageCache.get(pageNum);
            state.pageCache.delete(pageNum);
            state.pageCache.set(pageNum, data);
            return data;
        }
        return null;
    },

    setCachedPage(pageNum, data) {
        // Calculate approximate size: width * height * 4 bytes (RGBA)
        const pageSize = data.width * data.height * 4;

        // Evict until we have space
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

        state.pageCache.set(pageNum, data);
        state.currentCacheBytes += pageSize;
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
            let pageContainer = document.getElementById(`page-container-${index}`);

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

                container.appendChild(pageContainer);
            }

            const pdfCanvas = document.getElementById(`page-canvas-${index}`);
            const annCanvas = document.getElementById(`ann-canvas-${index}`);

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
        const pdfCanvas = document.getElementById(`page-canvas-${pageNum}`);
        const annCanvas = document.getElementById(`ann-canvas-${pageNum}`);
        const placeholder = document.querySelector(`#page-container-${pageNum} .page-placeholder`);

        if (pdfCanvas && pdfCanvas.style.display !== 'none') {
            const ctx = pdfCanvas.getContext('2d');
            ctx.clearRect(0, 0, pdfCanvas.width, pdfCanvas.height);
            pdfCanvas.width = 0;
            pdfCanvas.height = 0;
            pdfCanvas.style.display = 'none';
        }

        if (annCanvas) {
            const ctx = annCanvas.getContext('2d');
            ctx.clearRect(0, 0, annCanvas.width, annCanvas.height);
            annCanvas.width = 0;
            annCanvas.height = 0;
        }

        if (placeholder) placeholder.style.display = 'flex';
    },

    async renderPage(pageNum) {
        if (pageNum < 0 || pageNum >= state.totalPages) return;

        const pdfCanvas = document.getElementById(`page-canvas-${pageNum}`);
        const annCanvas = document.getElementById(`ann-canvas-${pageNum}`);
        const pdfCtx = pdfCanvas.getContext('2d');
        const placeholder = document.querySelector(`#page-container-${pageNum} .page-placeholder`);

        const cached = renderer.getCachedPage(pageNum);

        // If cached and zoom matches, draw instantly
        if (cached && cached.zoom === state.currentZoom) {
            renderer.drawCachedPage(pageNum, cached);
            return;
        }

        try {
            const responseBytes = await api.renderPage(pageNum, state.renderScale);

            const view = new DataView(responseBytes);
            const width = view.getInt32(0, false); // Big Endian
            const height = view.getInt32(4, false); // Big Endian

            // Pixels start at offset 8
            const pixels = new Uint8ClampedArray(responseBytes, 8);

            if (!state.visiblePages.has(pageNum)) return;

            const imageData = new ImageData(pixels, width, height);
            const imageBitmap = await createImageBitmap(imageData);

            if (!state.visiblePages.has(pageNum)) {
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
                zoom: state.renderScale,
                width: width,
                height: height
            });

            renderer.drawAnnotations(pageNum);

        } catch (e) {
            console.error(`Failed to render page ${pageNum}:`, e);
        }
    },

    drawCachedPage(pageNum, cached) {
        const pdfCanvas = document.getElementById(`page-canvas-${pageNum}`);
        const annCanvas = document.getElementById(`ann-canvas-${pageNum}`);
        const pdfCtx = pdfCanvas.getContext('2d');
        const placeholder = document.querySelector(`#page-container-${pageNum} .page-placeholder`);

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
        const canvas = document.getElementById(`ann-canvas-${pageNum}`);
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
};
