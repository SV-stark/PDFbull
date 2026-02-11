import { state } from './state.js';
import { api } from './api.js';
import { ui } from './ui.js';

export const exportManager = {
    async exportToImage() {
        if (!state.currentDoc) {
            ui.showToast('No document open', 'error');
            return;
        }

        try {
            ui.showLoading('Exporting page as image...');
            // Default to current page, original quality (1.0)
            const responseBytes = await api.renderPage(state.currentPage, 1.0);

            const view = new DataView(responseBytes);
            const width = view.getInt32(0, false);
            const height = view.getInt32(4, false);
            const pixels = new Uint8ClampedArray(responseBytes, 8);

            const imageData = new ImageData(pixels, width, height);
            const canvas = document.createElement('canvas');
            canvas.width = width;
            canvas.height = height;
            const ctx = canvas.getContext('2d');
            const bitmap = await createImageBitmap(imageData);
            ctx.drawImage(bitmap, 0, 0);
            bitmap.close();

            canvas.toBlob(async (blob) => {
                const { save } = window.__TAURI__.dialog;
                const savePath = await save({
                    filters: [{ name: 'PNG Image', extensions: ['png'] }],
                    defaultPath: `page_${state.currentPage + 1}.png`
                });

                if (savePath) {
                    const arrayBuffer = await blob.arrayBuffer();
                    await api.saveFile(savePath, Array.from(new Uint8Array(arrayBuffer)));
                    ui.showToast('Page exported as PNG', 'success');
                }
                ui.hideLoading();
            }, 'image/png');

        } catch (e) {
            console.error('Image export failed:', e);
            ui.hideLoading();
            ui.showToast('Export failed: ' + e, 'error');
        }
    },

    async exportToText() {
        if (!state.currentDoc) {
            ui.showToast('No document open', 'error');
            return;
        }

        try {
            const { ask } = window.__TAURI__.dialog;
            const exportAll = await ask('Export entire document? Select "Yes" for Whole Document, "No" for Current Page Only', {
                title: 'Export Text',
                type: 'info'
            });

            ui.showLoading('Extracting text...');
            let allText = '';

            if (exportAll) {
                for (let i = 0; i < state.totalPages; i++) {
                    const pageText = await api.getPageText(i);
                    allText += `--- Page ${i + 1} ---\n${pageText}\n\n`;
                }
            } else {
                const pageText = await api.getPageText(state.currentPage);
                allText += `--- Page ${state.currentPage + 1} ---\n${pageText}\n\n`;
            }

            ui.hideLoading();

            const { save } = window.__TAURI__.dialog;
            const savePath = await save({
                filters: [{ name: 'Text File', extensions: ['txt'] }],
                defaultPath: `${state.currentDoc.split(/[/\\]/).pop().replace('.pdf', '.txt')}`
            });

            if (savePath) {
                const encoder = new TextEncoder();
                await api.saveFile(savePath, Array.from(encoder.encode(allText)));
                ui.showToast('Text extracted and saved', 'success');
            }

        } catch (e) {
            console.error('Text export failed:', e);
            ui.hideLoading();
            ui.showToast('Text export failed: ' + e, 'error');
        }
    },

    async exportToJSON() {
        if (!state.currentDoc) {
            ui.showToast('No document open', 'error');
            return;
        }

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
    },

    async compressPDF() {
        if (!state.currentDoc) {
            ui.showToast('No document open', 'error');
            return;
        }

        ui.showLoading('Compressing PDF...');
        try {
            await api.compressPdf(50); // Medium quality integer
            ui.showToast('Compression complete', 'success');
        } catch (e) {
            ui.showToast('Compression failed: ' + e, 'error');
        } finally {
            ui.hideLoading();
        }
    }
};
