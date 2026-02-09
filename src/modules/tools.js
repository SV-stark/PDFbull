import { state, resetState } from './state.js';
import { ui } from './ui.js';
import { renderer } from './renderer.js';
import { CONSTANTS } from './constants.js';

export const tools = {
    setTool(tool) {
        state.currentTool = tool;

        document.querySelectorAll('.tool-btn[data-tool]').forEach(btn => {
            btn.classList.remove('active');
        });

        const activeBtn = document.querySelector(`[data-tool="${tool}"]`);
        if (activeBtn) activeBtn.classList.add('active');

        const viewer = ui.elements.viewerContainer();
        if (viewer) {
            viewer.style.cursor = CONSTANTS.CURSORS[tool] || 'default';
        }

        const currentToolEl = ui.elements.currentTool();
        if (currentToolEl) {
            currentToolEl.textContent = tool.charAt(0).toUpperCase() + tool.slice(1);
        }

        if (tool !== 'view') {
            ui.showToast(`${tool.charAt(0).toUpperCase() + tool.slice(1)} tool selected`);
        }
    },

    addAnnotation(type, data) {
        const pageAnnotations = state.annotations.get(state.currentPage) || [];
        const annotation = {
            id: Date.now().toString(),
            type,
            layer: state.currentLayer,
            ...data
        };

        pageAnnotations.push(annotation);
        state.annotations.set(state.currentPage, pageAnnotations);

        tools.saveState();
        renderer.drawAnnotations(state.currentPage);
    },

    saveState() {
        const savedState = {
            annotations: new Map(state.annotations),
            currentPage: state.currentPage,
            currentZoom: state.currentZoom,
            timestamp: Date.now()
        };

        // Remove any states after current index
        state.history = state.history.slice(0, state.historyIndex + 1);

        // Add new state
        state.history.push(savedState);

        // Limit history size
        if (state.history.length > CONSTANTS.MAX_HISTORY_SIZE) {
            state.history.shift();
        } else {
            state.historyIndex++;
        }

        ui.updateUndoRedoButtons();
        // Auto save or sync logic here if needed
    },

    undo() {
        if (state.historyIndex > 0) {
            state.historyIndex--;
            tools.restoreState(state.history[state.historyIndex]);
            ui.showToast('Undo successful');
        }
    },

    redo() {
        if (state.historyIndex < state.history.length - 1) {
            state.historyIndex++;
            tools.restoreState(state.history[state.historyIndex]);
            ui.showToast('Redo successful');
        }
    },

    // Drawing State
    tempCanvas: null,
    tempCtx: null,
    isDrawing: false,
    startX: 0,
    startY: 0,
    currentDrawingPage: -1,

    handleMouseDown(e) {
        if (state.currentTool === 'view') return;

        const layer = e.target.closest('.annotation-layer');
        if (!layer) return;

        const pageId = layer.id.split('-')[2];
        tools.currentDrawingPage = parseInt(pageId);
        state.currentPage = tools.currentDrawingPage;
        ui.updateStatusBar();

        tools.isDrawing = true;
        const rect = layer.getBoundingClientRect();
        tools.startX = e.clientX - rect.left;
        tools.startY = e.clientY - rect.top;

        tools.tempCanvas = document.createElement('canvas');
        tools.tempCanvas.width = layer.width;
        tools.tempCanvas.height = layer.height;
        tools.tempCtx = tools.tempCanvas.getContext('2d');
        tools.tempCtx.drawImage(layer, 0, 0);
    },

    handleMouseMove(e) {
        if (!tools.isDrawing || !tools.tempCtx || tools.currentDrawingPage === -1) return;

        const pageCanvas = document.getElementById(`ann-canvas-${tools.currentDrawingPage}`);
        if (!pageCanvas) return;
        const ctx = pageCanvas.getContext('2d');

        const rect = pageCanvas.getBoundingClientRect();
        const currentX = e.clientX - rect.left;
        const currentY = e.clientY - rect.top;

        ctx.clearRect(0, 0, pageCanvas.width, pageCanvas.height);
        ctx.drawImage(tools.tempCanvas, 0, 0);

        ctx.strokeStyle = state.selectedColor;
        ctx.fillStyle = state.selectedColor + '4D';
        ctx.lineWidth = 2;

        const width = currentX - tools.startX;
        const height = currentY - tools.startY;

        const startX = tools.startX;
        const startY = tools.startY;

        switch (state.currentTool) {
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
                renderer.drawArrow(ctx, startX, startY, currentX, currentY, state.selectedColor);
                break;
        }
    },

    handleMouseUp(e) {
        if (!tools.isDrawing || tools.currentDrawingPage === -1) return;

        const pageCanvas = document.getElementById(`ann-canvas-${tools.currentDrawingPage}`);
        if (!pageCanvas) {
            tools.isDrawing = false;
            tools.currentDrawingPage = -1;
            return;
        }

        tools.isDrawing = false;
        const rect = pageCanvas.getBoundingClientRect();
        const endX = e.clientX - rect.left;
        const endY = e.clientY - rect.top;

        const startX = tools.startX;
        const startY = tools.startY;

        const data = {
            color: state.selectedColor,
            x: Math.min(startX, endX),
            y: Math.min(startY, endY),
            w: Math.abs(endX - startX),
            h: Math.abs(endY - startY)
        };

        // Save logic matches main.js
        const savedPage = state.currentPage;
        state.currentPage = tools.currentDrawingPage;

        switch (state.currentTool) {
            case 'highlight': tools.addAnnotation('highlight', data); break;
            case 'rectangle': tools.addAnnotation('rectangle', data); break;
            case 'circle': tools.addAnnotation('circle', data); break;
            case 'line': tools.addAnnotation('line', { ...data, x1: startX, y1: startY, x2: endX, y2: endY }); break;
            case 'arrow': tools.addAnnotation('arrow', { ...data, x1: startX, y1: startY, x2: endX, y2: endY }); break;
            case 'text':
                const text = prompt('Enter text:');
                if (text) tools.addAnnotation('text', { ...data, text, x: startX, y: startY + 16 });
                break;
            case 'sticky':
                const note = prompt('Enter note:');
                if (note) tools.addAnnotation('sticky', { ...data, text: note, x: startX, y: startY });
                break;
        }

        state.currentPage = savedPage;
        tools.currentDrawingPage = -1;
        tools.tempCanvas = null;
        tools.tempCtx = null;
    },

    restoreState(historyState) {
        state.annotations = new Map(historyState.annotations);
        state.currentPage = historyState.currentPage;
        state.currentZoom = historyState.currentZoom;
        renderer.renderPage(state.currentPage);
        ui.updateUndoRedoButtons();
    }
};
