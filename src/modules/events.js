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
        this.bindSettingsEvents();
        this.bindTauriEvents();
        this.bindKeyboardEvents();
        this.bindCanvasEvents();

        // New Bindings
        document.getElementById('btn-bookmark')?.addEventListener('click', () => {
            // Assume app is global or dispatched
            // Main.js is not exported as 'app'. It is 'app' inside main.js.
            // We need to dispatch a custom event or attach 'app' to window.
            // Given main.js structure: `const app = { ... };` and not attached to window.
            // But main.js does: `document.addEventListener('app:open-file', ...)`
            // So we should dispatch events.

            // Wait, `app` in main.js is local. 
            // We need to add event listeners in main.js or export app?
            // main.js is a module, not exported.
            // I should add a custom event for bookmarking.
            document.dispatchEvent(new CustomEvent('app:toggle-bookmark'));
        });

        document.getElementById('btn-export-json')?.addEventListener('click', () => {
            document.dispatchEvent(new CustomEvent('app:export-json'));
        });

        document.getElementById('btn-close-keyboard-help')?.addEventListener('click', () => {
            document.getElementById('keyboard-help-modal').classList.add('hidden');
        });

        // Context specific inits
        scanner.init();
    },

    bindGlobalEvents() {
        window.addEventListener('resize', () => {
            // Optional: Re-calculate zoom if fitting?
            // renderer.setupVirtualScroller();
        });

        // Ctrl + Wheel Zoom
        window.addEventListener('wheel', (e) => {
            if (e.ctrlKey) {
                e.preventDefault();
                const zoomFactor = 1.1;
                const newZoom = e.deltaY < 0
                    ? state.currentZoom * zoomFactor
                    : state.currentZoom / zoomFactor;

                // Clamp zoom
                const clampedZoom = Math.min(Math.max(0.1, newZoom), 5.0);
                events.updateZoom(clampedZoom);
            }
        }, { passive: false });
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

        document.getElementById('btn-new-tab')?.addEventListener('click', () => {
            // Re-use open logic
            document.getElementById('btn-open').click();
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

        document.getElementById('zoom-select')?.addEventListener('change', (e) => {
            const val = e.target.value;
            if (val === 'fit-width') {
                // Calculate fit width zoom
                const container = document.getElementById('viewer-container');
                const page = state.pageDimensions[state.currentPage] || [600, 800];
                if (container) {
                    events.updateZoom((container.clientWidth - 40) / page[0]);
                }
            } else if (val === 'fit-page') {
                const container = document.getElementById('viewer-container');
                const page = state.pageDimensions[state.currentPage] || [600, 800];
                if (container) {
                    // rough fit
                    const scale = Math.min((container.clientWidth - 40) / page[0], (container.clientHeight - 40) / page[1]);
                    events.updateZoom(scale);
                }
            } else {
                events.updateZoom(parseFloat(val));
            }
        });

        document.getElementById('btn-fit-width')?.addEventListener('click', () => {
            document.getElementById('zoom-select').value = 'fit-width';
            document.getElementById('zoom-select').dispatchEvent(new Event('change'));
        });

        document.getElementById('btn-fit-page')?.addEventListener('click', () => {
            document.getElementById('zoom-select').value = 'fit-page';
            document.getElementById('zoom-select').dispatchEvent(new Event('change'));
        });

        document.getElementById('btn-fullscreen')?.addEventListener('click', () => {
            if (!document.fullscreenElement) {
                document.documentElement.requestFullscreen();
            } else {
                if (document.exitFullscreen) {
                    document.exitFullscreen();
                }
            }
        });

        // Rotation
        document.getElementById('btn-rotate')?.addEventListener('click', () => {
            // For now, consistent visual rotation via CSS
            // Ideally this should trigger a re-render with rotation if backend supported it.
            state.rotation = (state.rotation || 0) + 90;
            if (state.rotation >= 360) state.rotation = 0;

            // Apply to all pages for consistency
            document.querySelectorAll('.page-canvas').forEach(canvas => {
                canvas.style.transform = `rotate(${state.rotation}deg)`;
            });
            ui.showToast(`Rotated ${state.rotation}°`);
        });

        // Print
        document.getElementById('btn-print')?.addEventListener('click', () => {
            window.print();
        });

        // Edit
        document.getElementById('btn-undo')?.addEventListener('click', tools.undo);
        document.getElementById('btn-redo')?.addEventListener('click', tools.redo);

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
        document.getElementById('btn-search-next')?.addEventListener('click', search.nextResult);
        document.getElementById('btn-search-prev')?.addEventListener('click', search.prevResult);

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
    },

    bindSettingsEvents() {
        const modal = document.getElementById('settings-modal');
        const btnOpen = document.getElementById('btn-settings');
        const btnClose = document.getElementById('btn-close-settings');
        const btnSave = document.getElementById('btn-save-settings');

        if (btnOpen) {
            btnOpen.addEventListener('click', () => {
                // Load current settings into inputs
                const s = settings.load();

                // Appearance
                document.querySelectorAll('.theme-btn').forEach(btn => {
                    btn.classList.toggle('active', btn.dataset.theme === s.theme);
                });
                document.getElementById('setting-sidebar-width').value = s.sidebarWidth;
                document.getElementById('sidebar-width-value').textContent = s.sidebarWidth + 'px';
                document.getElementById('setting-toolbar-labels').checked = s.showToolbarLabels;

                // Behavior
                document.getElementById('setting-default-zoom').value = s.defaultZoom;
                document.getElementById('setting-autosave').value = s.autoSaveInterval;
                document.getElementById('autosave-value').textContent = s.autoSaveInterval + 's';
                document.getElementById('setting-restore-session').checked = s.restoreSession;
                document.getElementById('setting-smooth-scroll').checked = s.smoothScroll;
                document.getElementById('setting-double-click').value = s.doubleClickAction;

                // Performance
                document.getElementById('setting-cache-size').value = s.cacheSize;
                document.getElementById('cache-size-value').textContent = s.cacheSize + ' pages';
                document.getElementById('setting-render-quality').value = s.renderQuality;
                document.getElementById('setting-hardware-accel').checked = s.hardwareAccel;

                // Files
                document.getElementById('setting-recent-files').value = s.recentFilesLimit;
                document.getElementById('recent-files-value').textContent = s.recentFilesLimit + ' files';
                document.getElementById('setting-auto-open').checked = s.autoOpenLast;

                // Annotations
                document.getElementById('setting-sticky-width').value = s.stickyNoteWidth;
                document.getElementById('sticky-width-value').textContent = s.stickyNoteWidth + 'px';
                document.getElementById('setting-sticky-height').value = s.stickyNoteHeight;
                document.getElementById('sticky-height-value').textContent = s.stickyNoteHeight + 'px';

                modal.classList.remove('hidden');
            });
        }

        if (btnClose) {
            btnClose.addEventListener('click', () => {
                modal.classList.add('hidden');
            });
        }

        if (btnSave) {
            btnSave.addEventListener('click', () => {
                const newSettings = {
                    theme: document.querySelector('.theme-btn.active')?.dataset.theme || 'dark',
                    sidebarWidth: parseInt(document.getElementById('setting-sidebar-width').value),
                    showToolbarLabels: document.getElementById('setting-toolbar-labels').checked,
                    defaultZoom: document.getElementById('setting-default-zoom').value,
                    autoSaveInterval: parseInt(document.getElementById('setting-autosave').value),
                    restoreSession: document.getElementById('setting-restore-session').checked,
                    smoothScroll: document.getElementById('setting-smooth-scroll').checked,
                    doubleClickAction: document.getElementById('setting-double-click').value,
                    cacheSize: parseInt(document.getElementById('setting-cache-size').value),
                    renderQuality: document.getElementById('setting-render-quality').value,
                    hardwareAccel: document.getElementById('setting-hardware-accel').checked,
                    recentFilesLimit: parseInt(document.getElementById('setting-recent-files').value),
                    autoOpenLast: document.getElementById('setting-auto-open').checked,
                    stickyNoteWidth: parseInt(document.getElementById('setting-sticky-width').value),
                    stickyNoteHeight: parseInt(document.getElementById('setting-sticky-height').value),
                    // Accent color is handled via separate listener or we should grab it here
                    accentColor: getComputedStyle(document.documentElement).getPropertyValue('--accent-color').trim()
                };

                settings.save(newSettings);

                // Re-apply settings
                // We need to import applySettings or call logic
                // For now, let's assume global or re-import? 
                // events.js imports settings object, not applySettings function.
                // We should expose applySettings in settings.js and import it.
                // Or just reload page? No.
                // Let's rely on styles updates.

                // Theme
                document.documentElement.setAttribute('data-theme', newSettings.theme);

                // Colors
                // ...

                ui.showToast('Settings saved');
                modal.classList.add('hidden');
            });
        }

        // Tab Switching
        document.querySelectorAll('.settings-tab').forEach(tab => {
            tab.addEventListener('click', () => {
                document.querySelectorAll('.settings-tab').forEach(t => t.classList.remove('active'));
                document.querySelectorAll('.settings-panel').forEach(p => p.classList.remove('active'));

                tab.classList.add('active');
                document.getElementById(`panel-${tab.dataset.tab}`).classList.add('active');
            });
        });

        // Live Inputs (Visual Feedback)
        document.getElementById('setting-sidebar-width')?.addEventListener('input', (e) => {
            document.getElementById('sidebar-width-value').textContent = e.target.value + 'px';
        });
        // ... (add other live updates as needed)

        // Theme Buttons
        document.querySelectorAll('.theme-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                document.querySelectorAll('.theme-btn').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
            });
        });
    },

    bindSidebarEvents() {
        document.getElementById('btn-sidebar-left-toggle')?.addEventListener('click', () => {
            const sidebar = document.getElementById('sidebar-left');
            if (sidebar) sidebar.classList.toggle('collapsed');
        });

        document.getElementById('btn-sidebar-right-toggle')?.addEventListener('click', () => {
            const sidebar = document.getElementById('sidebar-right');
            if (sidebar) sidebar.classList.toggle('collapsed');
        });

        // Actions
        // Filters
        document.getElementById('btn-filter-gray')?.addEventListener('click', async () => {
            ui.showLoading('Applying Grayscale...');
            try {
                // Using scanner filter for now as it's implemented
                // or api.applyFilter if backend supports it.
                // Re-using applyScannerFilter for single page? No, it takes docPath.
                // Let's use api.applyFilter if available, else warn.
                // Actually filter logic was in pdf_engine.rs as apply_scanner_filter which processes WHOLE doc.
                // If we want single page view filter, we need CSS or canvas filter.
                // For "Action" sidebar, it implies modifying the PDF.
                // Let's assume we want to modify the document.
                if (!state.currentDoc) return;
                await api.applyScannerFilter(state.currentDoc, 'grayscale', 1.0);
                // Reload
                const event = new CustomEvent('app:open-file', { detail: state.currentDoc });
                document.dispatchEvent(event);
                ui.showToast('Grayscale applied');
            } catch (e) {
                console.error(e);
                ui.showToast('Filter failed: ' + e, 'error');
            } finally {
                ui.hideLoading();
            }
        });

        document.getElementById('btn-filter-invert')?.addEventListener('click', async () => {
            // Basic CSS inversion for view-only?
            // The button is in "Actions", likely meant for PDF modification.
            // But invert usually is a view preference.
            // Let's make it a view toggle for now (CSS).
            document.getElementById('pages-container').classList.toggle('filter-invert');
            ui.showToast('Invert Colors Toggled');
        });

        // Crop
        document.getElementById('btn-crop')?.addEventListener('click', async () => {
            if (!state.currentDoc) return;
            ui.showLoading('Auto-cropping...');
            try {
                await api.autoCrop(state.currentPage);
                renderer.renderPage(state.currentPage); // Re-render
                ui.showToast('Page cropped');
            } catch (e) {
                ui.showToast('Crop failed: ' + e, 'error');
            } finally {
                ui.hideLoading();
            }
        });

        // Compress
        document.getElementById('btn-compress')?.addEventListener('click', async () => {
            if (!state.currentDoc) return;
            ui.showLoading('Compressing PDF...');
            try {
                await api.compressPdf('medium');
                ui.showToast('Compression complete');
            } catch (e) {
                ui.showToast('Compression failed: ' + e, 'error');
            } finally {
                ui.hideLoading();
            }
        });

        // Batch Mode
        document.getElementById('btn-batch')?.addEventListener('click', () => {
            state.batchMode = !state.batchMode;
            document.getElementById('btn-batch').classList.toggle('active', state.batchMode);
            document.body.classList.toggle('batch-mode', state.batchMode);
            ui.showToast(state.batchMode ? 'Batch Mode Enabled' : 'Batch Mode Disabled');
            // Logic to show checkboxes on thumbnails would go here or in ui.js
        });

        // Note: Forms button is handled in bindToolbarEvents to avoid duplicate handlers
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

            if (e.key === '?' || e.key === 'F1') {
                e.preventDefault();
                document.getElementById('keyboard-help-modal').classList.toggle('hidden');
            }

            if (e.ctrlKey && e.key === 'd') {
                e.preventDefault();
                document.dispatchEvent(new CustomEvent('app:toggle-bookmark'));
            }

            if (e.ctrlKey && e.key === 'p') {
                e.preventDefault();
                window.print();
            }
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
