import { state } from './state.js';
import { api } from './api.js';
import { ui } from './ui.js';

export const scanner = {
    currentFilter: 'original',
    currentIntensity: 50,

    init() {
        // Bind Filter Buttons
        document.querySelectorAll('.filter-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                document.querySelectorAll('.filter-btn').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
                scanner.currentFilter = btn.dataset.filter;
                scanner.updateStyle();
            });
        });

        // Bind Intensity Slider
        const slider = document.getElementById('filter-intensity');
        if (slider) {
            slider.addEventListener('input', (e) => {
                scanner.currentIntensity = parseInt(e.target.value);
                scanner.updateStyle();
            });
        }
    },

    async open() {
        if (!state.currentDoc) {
            ui.showToast('No document open', 'error');
            return;
        }
        document.getElementById('scanner-modal').classList.remove('hidden');
        await scanner.renderPreview();
    },

    close() {
        document.getElementById('scanner-modal').classList.add('hidden');
    },

    async renderPreview() {
        const previewCanvas = document.getElementById('scanner-preview-canvas');
        if (!previewCanvas) return;

        const [w, h] = state.pageDimensions[state.currentPage];

        // Scale calculation
        const container = document.querySelector('.scanner-preview-container');
        if (!container) return;

        const containerW = container.clientWidth - 40;
        const containerH = container.clientHeight - 40;
        const scaleX = containerW / w;
        const scaleY = containerH / h;
        const scale = Math.min(scaleX, scaleY, 1.5);

        try {
            const result = await api.renderPage(state.currentPage, scale); // Returns bytes
            // api.renderPage returns ArrayBuffer

            const view = new DataView(result);
            const width = view.getInt32(0, false);
            const height = view.getInt32(4, false);
            const pixels = new Uint8ClampedArray(result, 8);

            const imageData = new ImageData(pixels, width, height);
            const ctx = previewCanvas.getContext('2d');

            previewCanvas.width = width;
            previewCanvas.height = height;

            const bitmap = await createImageBitmap(imageData);
            ctx.drawImage(bitmap, 0, 0);

            scanner.updateStyle();
        } catch (e) {
            console.error("Scanner preview failed", e);
        }
    },

    updateStyle() {
        const canvas = document.getElementById('scanner-preview-canvas');
        if (!canvas) return;

        const intensity = scanner.currentIntensity;
        let filterString = '';

        switch (scanner.currentFilter) {
            case 'original': filterString = 'none'; break;
            case 'grayscale': filterString = `grayscale(${intensity}%)`; break;
            case 'bw': filterString = `grayscale(100%) contrast(${100 + intensity}%)`; break;
            case 'lighten': filterString = `brightness(${100 + intensity / 2}%)`; break;
            case 'eco': filterString = `grayscale(100%) contrast(150%) brightness(120%)`; break;
            case 'noshadow': filterString = `contrast(120%) brightness(${100 + intensity / 3}%)`; break;
        }

        canvas.style.filter = filterString;
    },

    async apply() {
        if (!state.currentDoc) return;

        ui.showLoading('Applying filter...');
        try {
            await api.applyFilter(state.currentDoc, scanner.currentFilter, scanner.currentIntensity / 100.0);
            ui.showToast('Filter applied successfully', 'success');
            scanner.close();
            // Trigger reload?
            // We need to refresh the page.
        } catch (e) {
            console.error('Filter application failed:', e);
            ui.showToast('Failed to apply filter: ' + e, 'error');
        } finally {
            ui.hideLoading();
        }
    }
};
