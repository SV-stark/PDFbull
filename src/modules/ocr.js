/**
 * OCR Module - Handles Optical Character Recognition functionality
 * @module ocr
 */

import { state } from './state.js';
import { api } from './api.js';
import { ui } from './ui.js';
import { renderer } from './renderer.js';

const { listen } = window.__TAURI__.event || { listen: async () => () => { } };

/** @type {boolean} */
let isOcrRunning = false;

/** @type {Array<Object>|null} */
let ocrResults = null;

export const ocr = {
    /**
     * Initialize OCR module
     */
    async init() {
        this.bindEvents();
        await this.loadAvailableLanguages();
        this.setupProgressListener();
    },

    /**
     * Bind OCR-related events
     */
    bindEvents() {
        const btnOcr = document.getElementById('btn-ocr');
        const btnCancelOcr = document.getElementById('btn-cancel-ocr');

        btnOcr?.addEventListener('click', () => this.startOcr());
        btnCancelOcr?.addEventListener('click', () => this.cancelOcr());
    },

    /**
     * Setup listener for OCR progress events from backend
     */
    setupProgressListener() {
        listen('ocr-progress', (event) => {
            const { current, total, percentage } = event.payload;
            this.updateProgress(current, total, percentage);
        });
    },

    /**
     * Load available OCR languages from backend
     */
    async loadAvailableLanguages() {
        try {
            const languages = await window.__TAURI__.core.invoke('list_ocr_languages');
            const select = document.getElementById('ocr-language-select');
            if (select && languages.length > 0) {
                select.innerHTML = languages.map(lang =>
                    `<option value="${lang.code}">${lang.name}</option>`
                ).join('');
            }
        } catch (error) {
            console.warn('Failed to load OCR languages:', error);
        }
    },

    /**
     * Start OCR process on current document
     */
    async startOcr() {
        if (!state.currentDoc) {
            ui.showToast('Please open a PDF document first', 'error');
            return;
        }

        if (isOcrRunning) {
            ui.showToast('OCR is already running', 'warning');
            return;
        }

        const languageSelect = /** @type {HTMLSelectElement|null} */ (document.getElementById('ocr-language-select'));
        const language = languageSelect?.value || 'en';

        try {
            isOcrRunning = true;
            this.showProgressModal();

            // Get page images from renderer
            const pageCount = state.totalPages;
            const pages = [];

            for (let i = 0; i < pageCount; i++) {
                const pageData = await renderer.getPageImageData(i);
                if (pageData) {
                    pages.push(Array.from(pageData));
                }
            }

            // Call backend OCR
            const results = await window.__TAURI__.core.invoke('ocr_document', {
                pages,
                language
            });

            ocrResults = results;
            this.hideProgressModal();
            this.renderTextLayer(results);
            ui.showToast(`OCR completed: ${results.length} pages processed`, 'success');

            // Show save button
            this.showSaveOcrButton();

        } catch (error) {
            this.hideProgressModal();
            if (error.toString().includes('cancelled')) {
                ui.showToast('OCR cancelled', 'info');
            } else {
                ui.showToast('OCR failed: ' + error, 'error');
            }
        } finally {
            isOcrRunning = false;
        }
    },

    /**
     * Cancel ongoing OCR process
     */
    async cancelOcr() {
        try {
            await window.__TAURI__.core.invoke('cancel_ocr');
        } catch (error) {
            console.error('Failed to cancel OCR:', error);
        }
    },

    /**
     * Show progress modal
     */
    showProgressModal() {
        const modal = document.getElementById('ocr-progress-modal');
        modal?.classList.remove('hidden');
        this.updateProgress(0, state.totalPages, 0);
    },

    /**
     * Hide progress modal
     */
    hideProgressModal() {
        const modal = document.getElementById('ocr-progress-modal');
        modal?.classList.add('hidden');
    },

    /**
     * Update progress display
     * @param {number} current - Current page number
     * @param {number} total - Total page count
     * @param {number} percentage - Completion percentage
     */
    updateProgress(current, total, percentage) {
        const pageProgress = document.getElementById('ocr-page-progress');
        const percentageEl = document.getElementById('ocr-percentage');
        const progressBar = document.getElementById('ocr-progress-bar');

        if (pageProgress) pageProgress.textContent = `Page ${current}/${total}`;
        if (percentageEl) percentageEl.textContent = `${percentage}%`;
        if (progressBar) progressBar.style.width = `${percentage}%`;
    },

    /**
     * Render OCR text layer on canvas
     * @param {Array} results - OCR results from backend
     */
    renderTextLayer(results) {
        // Create text layer overlays for each page
        results.forEach((pageResult, index) => {
            const pageContainer = document.querySelector(`[data-page-index="${index}"]`);
            if (!pageContainer) return;

            // Remove existing text layer if any
            const existing = pageContainer.querySelector('.ocr-text-layer');
            if (existing) existing.remove();

            // Create text layer
            const textLayer = document.createElement('div');
            textLayer.className = 'ocr-text-layer';

            pageResult.blocks.forEach(block => {
                const textSpan = document.createElement('span');
                textSpan.className = 'ocr-text-block';
                textSpan.textContent = block.text;
                textSpan.style.cssText = `
                    position: absolute;
                    left: ${block.x}px;
                    top: ${block.y}px;
                    width: ${block.width}px;
                    height: ${block.height}px;
                    font-size: ${Math.max(10, block.height * 0.8)}px;
                    color: transparent;
                    user-select: text;
                    cursor: text;
                `;
                textLayer.appendChild(textSpan);
            });

            pageContainer.appendChild(textLayer);
        });
    },

    /**
     * Show save OCR to PDF button
     */
    showSaveOcrButton() {
        // For now, just show a toast with instruction
        ui.showToast('Use Export → Save OCR to PDF to embed text layer', 'info');
    },

    /**
     * Save OCR results to PDF
     */
    async saveOcrToPdf() {
        if (!ocrResults || !state.currentDoc) {
            ui.showToast('No OCR results to save', 'error');
            return;
        }

        try {
            ui.showLoading('Embedding OCR text layer...');

            const outputPath = state.currentDoc.replace('.pdf', '_ocr.pdf');
            await window.__TAURI__.core.invoke('save_ocr_to_pdf', {
                pdfPath: state.currentDoc,
                ocrData: ocrResults,
                outputPath
            });

            ui.hideLoading();
            ui.showToast(`OCR saved to: ${outputPath}`, 'success');
        } catch (error) {
            ui.hideLoading();
            ui.showToast('Failed to save OCR: ' + error, 'error');
        }
    },

    /**
     * Get current OCR results
     * @returns {Array|null}
     */
    getResults() {
        return ocrResults;
    },

    /**
     * Clear OCR results
     */
    clearResults() {
        ocrResults = null;
        // Remove all text layers
        document.querySelectorAll('.ocr-text-layer').forEach(el => el.remove());
    }
};
