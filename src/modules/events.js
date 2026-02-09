import { state, resetState } from './state.js';
import { api } from './api.js';
import { ui } from './ui.js';
import { renderer } from './renderer.js';
import { tools } from './tools.js';
import { search } from './search.js';
import { scanner } from './scanner.js';
import { settings } from './settings.js';
import { CONSTANTS } from './constants.js';

const { listen } = window.__TAURI__.event;

export const events = {
    init() {
        this.bindGlobalEvents();
        this.bindToolbarEvents();
        this.bindSidebarEvents();
        this.bindTauriEvents();
        this.bindKeyboardEvents();
        this.bindCanvasEvents(); // Delegated from viewerContainer
    },

    bindGlobalEvents() {
        window.addEventListener('resize', () => {
            // Optional: Re-calculate zoom if fitting?
            // renderer.setupVirtualScroller();
        });
    },

    bindToolbarEvents() {
        // Open/Save
        document.getElementById('btn-open')?.addEventListener('click', async () => {
            try {
                const { open } = window.__TAURI__.dialog;
                const selected = await open({
                    multiple: false,
                    directory: false,
                    filters: [{ name: 'PDF Files', extensions: ['pdf'] }, { name: 'All Files', extensions: ['*'] }]
                });
                if (selected) {
                    // Call main app logic to open tab
                    // We need a circular dependency fix or pass app controller?
                    // Ideally events dispatches a custom event or calls a "controller" module.
                    // For now, let's assume `window.PDFApp.openNewTab(selected)` is available or we import `app`.
                    // Circular dependency: main imports events. events imports main? No.
                    // events.js should take callbacks or we should have a `controller.js`.
                    // Or we attach `openNewTab` to `api` or `state`? No.
                    // Let's rely on a global or exported function passed on init, OR
                    // Move `openNewTab` to `renderer.js` or `state.js` (logic) + `ui.js` (dom)?
                    // `openNewTab` logic is: call API open_document, update state, create UI.
                    // This fits in a "controller" or "app" logic.
                    // Let's assume we can trigger a CustomEvent 'request-open-tab'.

                    const event = new CustomEvent('app:open-file', { detail: selected });
                    document.dispatchEvent(event);
                }
            } catch (e) {
                console.error(e);
            }
        });

        document.getElementById('btn-save')?.addEventListener('click', () => {
            const event = new CustomEvent('app:save');
            document.dispatchEvent(event);
        });

        // Navigation
        document.getElementById('btn-prev')?.addEventListener('click', () => {
            if (state.currentPage > 0) {
                state.currentPage--;
                renderer.scrollToPage(state.currentPage); // We need scrollToPage in renderer
            }
        });
        document.getElementById('btn-next')?.addEventListener('click', () => {
            if (state.currentPage < state.totalPages - 1) {
                state.currentPage++;
                renderer.scrollToPage(state.currentPage);
            }
        });

        // Zoom
        document.getElementById('btn-zoom-in')?.addEventListener('click', () => events.updateZoom(state.currentZoom * 1.25));
        document.getElementById('btn-zoom-out')?.addEventListener('click', () => events.updateZoom(state.currentZoom / 1.25));
        document.getElementById('btn-reset-zoom')?.addEventListener('click', () => {
            events.updateZoom(1.0);
            ui.showToast('Zoom reset to 100%');
        });

        // Tools
        ['highlight', 'rectangle', 'circle', 'line', 'arrow', 'text', 'sticky'].forEach(tool => {
            document.getElementById(`btn-${tool}`)?.addEventListener('click', () => tools.setTool(tool));
        });

        // Search
        document.getElementById('btn-search-toggle')?.addEventListener('click', search.togglePanel);
        document.getElementById('btn-search-exec')?.addEventListener('click', search.execute);
        document.getElementById('ipt-search')?.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') search.execute();
        });

        // Forms
        document.getElementById('btn-forms')?.addEventListener('click', () => {
            // trigger form scan
            const event = new CustomEvent('app:scan-forms');
            document.dispatchEvent(event);
        });

        // Scanner
        document.getElementById('btn-scanner')?.addEventListener('click', scanner.open);
        document.getElementById('btn-close-scanner')?.addEventListener('click', scanner.close);
        document.getElementById('btn-scanner-apply')?.addEventListener('click', scanner.apply);

        // Settings
        document.getElementById('btn-settings')?.addEventListener('click', () => document.getElementById('settings-modal').classList.remove('hidden'));
        document.getElementById('btn-close-settings')?.addEventListener('click', () => document.getElementById('settings-modal').classList.add('hidden'));
    },

    bindSidebarEvents() {
        document.getElementById('btn-sidebar-toggle')?.addEventListener('click', () => {
            document.getElementById('sidebar').classList.toggle('collapsed');
        });
    },

    bindTauriEvents() {
        // Drag and Drop
        listen('tauri://drag-enter', () => ui.elements.viewerContainer().classList.add('drag-over'));
        listen('tauri://drag-leave', () => ui.elements.viewerContainer().classList.remove('drag-over'));
        listen('tauri://drag-drop', (event) => {
            ui.elements.viewerContainer().classList.remove('drag-over');
            if (event.payload.paths && event.payload.paths.length > 0) {
                const path = event.payload.paths[0]; // Just open first for now
                if (path.toLowerCase().endsWith('.pdf')) {
                    const e = new CustomEvent('app:open-file', { detail: path });
                    document.dispatchEvent(e);
                }
            }
        });

        listen('open-file', (event) => {
            const path = event.payload;
            if (path) {
                const e = new CustomEvent('app:open-file', { detail: path });
                document.dispatchEvent(e);
            }
        });
    },

    updateZoom(newZoom) {
        state.currentZoom = newZoom;
        // Visual update
        document.getElementById('pages-container').style.setProperty('--zoom-factor', state.currentZoom);
        ui.updateUI();

        // Debounce commit
        if (state.zoomTimeout) clearTimeout(state.zoomTimeout);
        state.zoomTimeout = setTimeout(() => {
            state.renderScale = state.currentZoom;
            renderer.setupVirtualScroller();
        }, 300);
    },

    bindKeyboardEvents() {
        document.addEventListener('keydown', (e) => {
            // Shortcuts (Ctrl+Z, etc) -> call tools.undo(), tools.redo()
            if (e.ctrlKey && e.key === 'z') { e.preventDefault(); tools.undo(); }
            if (e.ctrlKey && e.key === 'y') { e.preventDefault(); tools.redo(); }
            // ... other shortcuts
        });
    },

    bindCanvasEvents() {
        const viewer = ui.elements.viewerContainer();
        if (!viewer) return;

        viewer.addEventListener('mousedown', (e) => {
            if (state.currentTool === 'view') return;
            // Delegate to tools module which should handle drawing logic
            // For now, let's assume tools has handleMouseDown(e)
            // or we implement the logic here? 
            // Implementation in main.js was inline.
            // Let's create `tools.handleMouseDown(e)`
            tools.handleMouseDown && tools.handleMouseDown(e);
        });

        viewer.addEventListener('mousemove', (e) => {
            tools.handleMouseMove && tools.handleMouseMove(e);
        });

        viewer.addEventListener('mouseup', (e) => {
            tools.handleMouseUp && tools.handleMouseUp(e);
        });
    }
};
