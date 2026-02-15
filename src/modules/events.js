import { state, resetState } from './state.js';
import { api } from './api.js';
import { ui } from './ui.js';
import { renderer } from './renderer.js';
import { tools } from './tools.js';
// import { startAutoSave } from './autosave.js'; // Removed as file does not exist
import { search } from './search.js';
import { scanner } from './scanner.js';
import { settings } from './settings.js';
import { exportManager } from './export.js';
import { CONSTANTS } from './constants.js';
import { debug } from './debug.js';

const { listen } = window.__TAURI__?.event || { listen: () => () => { } };

export const events = {
    init() {
        this.bindGlobalEvents();
        this.bindToolbarEvents();
        this.bindSidebarEvents();
        this.bindSettingsEvents();
        this.bindTauriEvents();
        this.bindKeyboardEvents();
        this.bindCanvasEvents();
        this.bindColorPickerEvents();
        this.bindPageInputEvents();

        // Bookmark button
        document.getElementById('btn-bookmark')?.addEventListener('click', () => {
            document.dispatchEvent(new CustomEvent('app:toggle-bookmark'));
        });

        // Export JSON in export modal - Handled in bindToolbarEvents now
        /*
        document.getElementById('btn-export-json')?.addEventListener('click', () => {
            document.dispatchEvent(new CustomEvent('app:export-json'));
            document.getElementById('export-modal')?.classList.add('hidden');
        });
        */

        // Keyboard help close
        document.getElementById('btn-close-keyboard-help')?.addEventListener('click', () => {
            document.getElementById('keyboard-help-modal').classList.add('hidden');
        });

        // Context specific inits
        scanner.init();
    },

    bindGlobalEvents() {
        window.addEventListener('resize', () => {
            // Optional: Re-calculate zoom if fitting
        });

        // Ctrl + Wheel Zoom
        window.addEventListener('wheel', (e) => {
            if (e.ctrlKey) {
                e.preventDefault();
                const zoomFactor = 1.1;
                const newZoom = e.deltaY < 0
                    ? state.currentZoom * zoomFactor
                    : state.currentZoom / zoomFactor;

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
                    const event = new CustomEvent('app:open-file', { detail: selected });
                    document.dispatchEvent(event);
                }
            } catch (e) {
                console.error(e);
            }
        });

        document.getElementById('btn-new-tab')?.addEventListener('click', () => {
            document.getElementById('btn-open').click();
        });

        document.getElementById('btn-save')?.addEventListener('click', () => {
            document.dispatchEvent(new CustomEvent('app:save'));
        });

        // Navigation
        document.getElementById('btn-prev')?.addEventListener('click', () => {
            if (state.currentPage > 0) {
                state.currentPage--;
                renderer.scrollToPage(state.currentPage);
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

        document.getElementById('zoom-select')?.addEventListener('change', (e) => {
            const val = (/** @type {HTMLSelectElement} */ (e.target)).value;
            if (val === 'fit-width') {
                const container = document.getElementById('viewer-container');
                const page = state.pageDimensions[state.currentPage] || [600, 800];
                if (container) {
                    events.updateZoom((container.clientWidth - 40) / page[0]);
                }
            } else if (val === 'fit-page') {
                const container = document.getElementById('viewer-container');
                const page = state.pageDimensions[state.currentPage] || [600, 800];
                if (container) {
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
            state.rotation = (state.rotation || 0) + 90;
            if (state.rotation >= 360) state.rotation = 0;

            document.querySelectorAll('.page-canvas').forEach(canvas => {
                canvas.style.transform = `rotate(${state.rotation}deg)`;
            });
            ui.showToast(`Rotated ${state.rotation}°`);
        });

        // Print
        document.getElementById('btn-print')?.addEventListener('click', async () => {
            if (!state.currentDoc) {
                ui.showToast('No document open', 'error');
                return;
            }
            try {
                await api.printPdf();
                ui.showToast('Print dialog opened');
            } catch (e) {
                console.error('Print failed:', e);
                ui.showToast('Print failed: ' + e, 'error');
            }
        });

        // View Mode Buttons
        document.getElementById('btn-view-single')?.addEventListener('click', () => events.setViewMode('single'));
        document.getElementById('btn-view-continuous')?.addEventListener('click', () => events.setViewMode('continuous'));
        document.getElementById('btn-view-facing')?.addEventListener('click', () => events.setViewMode('facing'));
        document.getElementById('btn-view-book')?.addEventListener('click', () => events.setViewMode('book'));

        // Sidebar Navigation Tabs
        document.getElementById('btn-nav-thumbnails')?.addEventListener('click', () => {
            document.getElementById('btn-nav-thumbnails').classList.add('active');
            document.getElementById('btn-nav-outline').classList.remove('active');
            document.getElementById('thumbnails-panel').classList.remove('hidden');
            document.getElementById('outline-panel').classList.add('hidden');
        });

        document.getElementById('btn-nav-outline')?.addEventListener('click', async () => {
            document.getElementById('btn-nav-outline').classList.add('active');
            document.getElementById('btn-nav-thumbnails').classList.remove('active');
            document.getElementById('thumbnails-panel').classList.add('hidden');
            document.getElementById('outline-panel').classList.remove('hidden');
            
            // Load outline if not loaded
            if (state.outline.length === 0 && state.currentDoc) {
                try {
                    const outline = await api.getOutline();
                    state.outline = outline;
                    ui.renderOutline(outline);
                } catch (e) {
                    console.error('Failed to load outline:', e);
                }
            }
        });

        // Undo/Redo
        document.getElementById('btn-undo')?.addEventListener('click', tools.undo);
        document.getElementById('btn-redo')?.addEventListener('click', tools.redo);

        // Drawing Tools
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
        document.getElementById('btn-search-close')?.addEventListener('click', () => {
            search.isSearchOpen = false;
            document.getElementById('search-panel')?.classList.add('hidden');
        });

        // Forms
        document.getElementById('btn-forms')?.addEventListener('click', () => {
            document.dispatchEvent(new CustomEvent('app:scan-forms'));
        });

        // Scanner
        document.getElementById('btn-scanner')?.addEventListener('click', scanner.open);
        document.getElementById('btn-close-scanner')?.addEventListener('click', scanner.close);
        document.getElementById('btn-scanner-apply')?.addEventListener('click', scanner.apply);

        // Export Modal
        document.getElementById('btn-export')?.addEventListener('click', () => {
            if (!state.currentDoc) {
                ui.showToast('No document open', 'error');
                return;
            }
            document.getElementById('export-modal')?.classList.remove('hidden');
        });
        document.getElementById('btn-close-export')?.addEventListener('click', () => {
            document.getElementById('export-modal')?.classList.add('hidden');
        });

        // Export Actions
        document.getElementById('btn-export-image')?.addEventListener('click', async () => {
            await exportManager.exportToImage();
            document.getElementById('export-modal')?.classList.add('hidden');
        });

        document.getElementById('btn-export-text')?.addEventListener('click', async () => {
            await exportManager.exportToText();
            document.getElementById('export-modal')?.classList.add('hidden');
        });

        document.getElementById('btn-export-json')?.addEventListener('click', () => {
            exportManager.exportToJSON();
            document.getElementById('export-modal')?.classList.add('hidden');
        });


    },

    bindSettingsEvents() {
        const modal = document.getElementById('settings-modal');
        const btnOpen = document.getElementById('btn-settings');
        const btnClose = document.getElementById('btn-close-settings');
        const btnSave = document.getElementById('btn-save-settings');

        if (btnOpen) {
            btnOpen.addEventListener('click', () => {
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
                    accentColor: getComputedStyle(document.documentElement).getPropertyValue('--accent-color').trim()
                };

                settings.save(newSettings);

                // Apply theme
                document.documentElement.setAttribute('data-theme', newSettings.theme);

                // Apply accent color
                document.documentElement.style.setProperty('--accent-color', newSettings.accentColor);
                document.documentElement.style.setProperty('--accent-hover', settings.adjustColor(newSettings.accentColor, -20));

                // Apply sidebar width
                const sidebarRight = document.getElementById('sidebar-right');
                if (sidebarRight) sidebarRight.style.width = newSettings.sidebarWidth + 'px';

                // Apply toolbar labels
                const toolbar = document.querySelector('.toolbar');
                if (toolbar) {
                    toolbar.classList.toggle('hide-labels', !newSettings.showToolbarLabels);
                }

                ui.showToast('Settings saved', 'success');
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

        // Live Input Feedback
        document.getElementById('setting-sidebar-width')?.addEventListener('input', (e) => {
            document.getElementById('sidebar-width-value').textContent = e.target.value + 'px';
        });
        document.getElementById('setting-autosave')?.addEventListener('input', (e) => {
            document.getElementById('autosave-value').textContent = e.target.value + 's';
        });
        document.getElementById('setting-cache-size')?.addEventListener('input', (e) => {
            document.getElementById('cache-size-value').textContent = e.target.value + ' pages';
        });
        document.getElementById('setting-recent-files')?.addEventListener('input', (e) => {
            document.getElementById('recent-files-value').textContent = e.target.value + ' files';
        });
        document.getElementById('setting-sticky-width')?.addEventListener('input', (e) => {
            document.getElementById('sticky-width-value').textContent = e.target.value + 'px';
        });
        document.getElementById('setting-sticky-height')?.addEventListener('input', (e) => {
            document.getElementById('sticky-height-value').textContent = e.target.value + 'px';
        });

        // Theme Buttons
        document.querySelectorAll('.theme-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                document.querySelectorAll('.theme-btn').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
            });
        });

        // Accent Color Picker
        document.getElementById('accent-color-picker')?.addEventListener('input', (e) => {
            const color = e.target.value;
            document.documentElement.style.setProperty('--accent-color', color);
            document.documentElement.style.setProperty('--accent-hover', settings.adjustColor(color, -20));
        });

        // Accent color preset buttons in settings
        document.querySelectorAll('#panel-appearance .color-btn[data-color]').forEach(btn => {
            btn.addEventListener('click', () => {
                const color = btn.dataset.color;
                document.documentElement.style.setProperty('--accent-color', color);
                document.documentElement.style.setProperty('--accent-hover', settings.adjustColor(color, -20));
                const picker = document.getElementById('accent-color-picker');
                if (picker) picker.value = color;
            });
        });

        // Browse Path button
        document.getElementById('btn-browse-path')?.addEventListener('click', async () => {
            try {
                const { open } = window.__TAURI__.dialog;
                const selected = await open({ directory: true, multiple: false });
                if (selected) {
                    document.getElementById('setting-default-path').value = selected;
                }
            } catch (e) {
                console.error('Browse failed:', e);
            }
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

        // Greyscale filter (CSS-based since backend is a stub)
        document.getElementById('btn-filter-gray')?.addEventListener('click', () => {
            document.getElementById('pages-container').classList.toggle('filter-grayscale');
            const active = document.getElementById('pages-container').classList.contains('filter-grayscale');
            document.getElementById('btn-filter-gray')?.classList.toggle('active', active);
            ui.showToast(active ? 'Greyscale filter applied' : 'Greyscale filter removed');
        });

        // Invert filter (CSS-based)
        document.getElementById('btn-filter-invert')?.addEventListener('click', () => {
            document.getElementById('pages-container').classList.toggle('filter-invert');
            const active = document.getElementById('pages-container').classList.contains('filter-invert');
            document.getElementById('btn-filter-invert')?.classList.toggle('active', active);
            ui.showToast(active ? 'Invert filter applied' : 'Invert filter removed');
        });

        // Crop
        document.getElementById('btn-crop')?.addEventListener('click', async () => {
            if (!state.currentDoc) return;
            ui.showLoading('Auto-cropping...');
            try {
                debug.log(`Auto-cropping page ${state.currentPage}`);
                await api.autoCrop(state.currentPage);
                
                // Refresh page dimensions after crop
                state.pageDimensions = await api.getPageDimensions();
                await renderer.setupVirtualScroller();
                await renderer.renderPage(state.currentPage);
                
                debug.log(`Crop complete, dimensions refreshed`);
                ui.showToast('Page cropped');
            } catch (e) {
                debug.error('Crop failed:', e);
                ui.showToast('Crop failed: ' + e, 'error');
            } finally {
                ui.hideLoading();
            }
        });

        // Compress
        document.getElementById('btn-compress')?.addEventListener('click', () => {
            // Open Compress Modal
            document.getElementById('compress-modal')?.classList.remove('hidden');
        });

        // Close Compress Modal
        document.getElementById('btn-close-compress')?.addEventListener('click', () => {
            document.getElementById('compress-modal')?.classList.add('hidden');
        });

        // Compress Level Selection & Execution
        document.querySelectorAll('#compress-modal .export-card').forEach(card => {
            card.addEventListener('click', async () => {
                const level = card.dataset.level;

                // Close modal
                document.getElementById('compress-modal')?.classList.add('hidden');

                if (!state.currentDoc) {
                    ui.showToast('No document open', 'error');
                    return;
                }

                try {
                    // Pick save location
                    const { save } = window.__TAURI__.dialog;
                    const savePath = await save({
                        filters: [{ name: 'Compressed PDF', extensions: ['pdf'] }],
                        defaultPath: state.currentDoc.replace('.pdf', '_compressed.pdf')
                    });

                    if (savePath) {
                        ui.showLoading(`Compressing (${level})...`);
                        const result = await api.compressPdf(state.currentDoc, savePath, level);

                        ui.hideLoading();
                        ui.showToast(`Compressed! Saved ${result.savings_percent}% (${formatBytes(result.original_size)} -> ${formatBytes(result.compressed_size)})`, 'success');
                    }
                } catch (e) {
                    ui.hideLoading();
                    ui.showToast('Compression failed: ' + e, 'error');
                }
            });
        });

        function formatBytes(bytes, decimals = 2) {
            if (!+bytes) return '0 Bytes';
            const k = 1024;
            const dm = decimals < 0 ? 0 : decimals;
            const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
            const i = Math.floor(Math.log(bytes) / Math.log(k));
            return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
        }

        // Batch Mode
        document.getElementById('btn-batch')?.addEventListener('click', () => {
            state.batchMode = !state.batchMode;
            document.getElementById('btn-batch').classList.toggle('active', state.batchMode);
            document.body.classList.toggle('batch-mode', state.batchMode);
            ui.showToast(state.batchMode ? 'Batch Mode Enabled' : 'Batch Mode Disabled');
        });

        // Add Layer
        document.getElementById('btn-add-layer')?.addEventListener('click', () => {
            const name = `Layer ${state.visibleLayers.size + 1}`;
            state.visibleLayers.add(name);

            const container = document.getElementById('layers-container');
            if (container) {
                const label = document.createElement('label');
                label.className = 'layer-item';
                label.innerHTML = `<input type="checkbox" checked data-layer="${name}"><span>${name}</span>`;
                label.querySelector('input').addEventListener('change', (e) => {
                    if (e.target.checked) {
                        state.visibleLayers.add(name);
                    } else {
                        state.visibleLayers.delete(name);
                    }
                    // Re-render visible pages to update annotation visibility
                    state.visiblePages.forEach(pageNum => renderer.drawAnnotations(pageNum));
                });
                container.appendChild(label);
            }
            ui.showToast(`Layer "${name}" added`);
        });

        // Layer checkbox changes on existing default layer
        document.querySelectorAll('#layers-container input[data-layer]').forEach(checkbox => {
            checkbox.addEventListener('change', (e) => {
                const layerName = e.target.dataset.layer;
                if (e.target.checked) {
                    state.visibleLayers.add(layerName);
                } else {
                    state.visibleLayers.delete(layerName);
                }
                state.visiblePages.forEach(pageNum => renderer.drawAnnotations(pageNum));
            });
        });
    },

    bindTauriEvents() {
        // Drag and Drop
        listen('tauri://drag-enter', () => ui.elements.viewerContainer()?.classList.add('drag-over'));
        listen('tauri://drag-leave', () => ui.elements.viewerContainer()?.classList.remove('drag-over'));
        listen('tauri://drag-drop', (event) => {
            ui.elements.viewerContainer()?.classList.remove('drag-over');
            if (event.payload.paths && event.payload.paths.length > 0) {
                const path = event.payload.paths[0];
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
        const oldZoom = state.currentZoom;
        state.currentZoom = newZoom;
        document.getElementById('pages-container').style.setProperty('--zoom-factor', state.currentZoom);

        const zoomLevel = document.getElementById('zoom-level');
        if (zoomLevel) zoomLevel.textContent = `${Math.round(state.currentZoom * 100)}%`;

        ui.updateUI();

        if (state.zoomTimeout) clearTimeout(state.zoomTimeout);
        
        state.zoomTimeout = setTimeout(async () => {
            debug.log(`Zoom debounce complete: ${oldZoom} -> ${newZoom}`);
            
            state.renderScale = state.currentZoom;
            state.pageCache.clear();
            state.currentCacheBytes = 0;
            state.textLayerCache.clear();
            renderer.clearCanvasContextCache();
            await renderer.setupVirtualScroller();
            
            debug.log(`Virtual scroller re-setup complete`);
        }, 150);
    },

    async setViewMode(mode) {
        // Update button states
        document.getElementById('btn-view-single')?.classList.remove('active');
        document.getElementById('btn-view-continuous')?.classList.remove('active');
        document.getElementById('btn-view-facing')?.classList.remove('active');
        document.getElementById('btn-view-book')?.classList.remove('active');
        
        state.viewMode = mode;
        state.continuousMode = false;
        state.facingMode = false;
        state.bookView = false;
        
        const pagesContainer = document.getElementById('pages-container');
        
        switch (mode) {
            case 'single':
                document.getElementById('btn-view-single')?.classList.add('active');
                break;
            case 'continuous':
                document.getElementById('btn-view-continuous')?.classList.add('active');
                state.continuousMode = true;
                break;
            case 'facing':
                document.getElementById('btn-view-facing')?.classList.add('active');
                state.facingMode = true;
                pagesContainer?.classList.add('facing-mode');
                break;
            case 'book':
                document.getElementById('btn-view-book')?.classList.add('active');
                state.facingMode = true;
                state.bookView = true;
                pagesContainer?.classList.add('book-view');
                break;
        }
        
        if (mode !== 'facing' && mode !== 'book') {
            pagesContainer?.classList.remove('facing-mode', 'book-view');
        }
        
        ui.showToast(`View: ${mode.charAt(0).toUpperCase() + mode.slice(1)}`);
        
        // Re-render pages
        state.pageCache.clear();
        state.currentCacheBytes = 0;
        await renderer.setupVirtualScroller();
    },

    bindKeyboardEvents() {
        document.addEventListener('keydown', (e) => {
            // Don't capture shortcuts when typing in inputs
            const tag = e.target.tagName;
            if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') {
                // Allow Escape in inputs
                if (e.key === 'Escape') {
                    e.target.blur();
                }
                return;
            }

            // Ctrl shortcuts
            if (e.ctrlKey && !e.shiftKey) {
                switch (e.key.toLowerCase()) {
                    case 'o':
                        e.preventDefault();
                        document.getElementById('btn-open')?.click();
                        break;
                    case 's':
                        e.preventDefault();
                        document.dispatchEvent(new CustomEvent('app:save'));
                        break;
                    case 'f':
                        e.preventDefault();
                        search.togglePanel();
                        break;
                    case 'e':
                        e.preventDefault();
                        document.getElementById('btn-export')?.click();
                        break;
                    case 'z':
                        e.preventDefault();
                        tools.undo();
                        break;
                    case 'y':
                        e.preventDefault();
                        tools.redo();
                        break;
                    case 'k':
                        e.preventDefault();
                        window.commandPalette?.open();
                        break;
                    case 'd':
                        e.preventDefault();
                        document.dispatchEvent(new CustomEvent('app:toggle-bookmark'));
                        break;
                    case 'p':
                        e.preventDefault();
                        window.print();
                        break;
                    case 'b':
                        e.preventDefault();
                        document.getElementById('btn-sidebar-left-toggle')?.click();
                        break;
                    case ',':
                        e.preventDefault();
                        document.getElementById('btn-settings')?.click();
                        break;
                    case '=':
                    case '+':
                        e.preventDefault();
                        events.updateZoom(state.currentZoom * 1.25);
                        break;
                    case '-':
                        e.preventDefault();
                        events.updateZoom(state.currentZoom / 1.25);
                        break;
                }
            }

            // Ctrl+Shift shortcuts
            if (e.ctrlKey && e.shiftKey) {
                switch (e.key.toLowerCase()) {
                    case 'b':
                        e.preventDefault();
                        document.getElementById('btn-sidebar-right-toggle')?.click();
                        break;
                    case '2':
                        e.preventDefault();
                        events.setViewMode('facing');
                        break;
                    case '1':
                        e.preventDefault();
                        events.setViewMode('single');
                        break;
                    case '3':
                        e.preventDefault();
                        events.setViewMode('continuous');
                        break;
                }
            }

            // Alt shortcuts for view modes
            if (e.altKey && !e.ctrlKey) {
                switch (e.key.toLowerCase()) {
                    case '1':
                        e.preventDefault();
                        events.setViewMode('single');
                        break;
                    case '2':
                        e.preventDefault();
                        events.setViewMode('continuous');
                        break;
                    case '3':
                        e.preventDefault();
                        events.setViewMode('facing');
                        break;
                    case '4':
                        e.preventDefault();
                        events.setViewMode('book');
                        break;
                }
            }

            // Single key shortcuts (no modifiers)
            if (!e.ctrlKey && !e.altKey && !e.metaKey) {
                switch (e.key) {
                    case 'ArrowLeft':
                        if (state.currentPage > 0) {
                            state.currentPage--;
                            renderer.scrollToPage(state.currentPage);
                        }
                        break;
                    case 'ArrowRight':
                        if (state.currentPage < state.totalPages - 1) {
                            state.currentPage++;
                            renderer.scrollToPage(state.currentPage);
                        }
                        break;
                    case 'Home':
                        e.preventDefault();
                        state.currentPage = 0;
                        renderer.scrollToPage(0);
                        break;
                    case 'End':
                        e.preventDefault();
                        state.currentPage = state.totalPages - 1;
                        renderer.scrollToPage(state.totalPages - 1);
                        break;
                    case 'PageUp':
                        e.preventDefault();
                        if (state.currentPage > 0) {
                            state.currentPage--;
                            renderer.scrollToPage(state.currentPage);
                        }
                        break;
                    case 'PageDown':
                        e.preventDefault();
                        if (state.currentPage < state.totalPages - 1) {
                            state.currentPage++;
                            renderer.scrollToPage(state.currentPage);
                        }
                        break;
                    case ' ':
                        e.preventDefault();
                        if (state.currentPage < state.totalPages - 1) {
                            state.currentPage++;
                            renderer.scrollToPage(state.currentPage);
                        }
                        break;
                    case '?':
                    case 'F1':
                        e.preventDefault();
                        document.getElementById('keyboard-help-modal').classList.toggle('hidden');
                        break;
                    case 'F11':
                        e.preventDefault();
                        document.getElementById('btn-fullscreen')?.click();
                        break;
                    case 'w':
                        e.preventDefault();
                        document.getElementById('zoom-select').value = 'fit-width';
                        document.getElementById('zoom-select').dispatchEvent(new Event('change'));
                        break;
                    case 'W':
                        e.preventDefault();
                        document.getElementById('zoom-select').value = 'fit-width';
                        document.getElementById('zoom-select').dispatchEvent(new Event('change'));
                        break;
                    case 'z':
                        e.preventDefault();
                        document.getElementById('zoom-select').value = 'fit-page';
                        document.getElementById('zoom-select').dispatchEvent(new Event('change'));
                        break;
                    case 'Z':
                        e.preventDefault();
                        document.getElementById('zoom-select').value = 'fit-page';
                        document.getElementById('zoom-select').dispatchEvent(new Event('change'));
                        break;
                    case 'Escape':
                        // Close any open modal
                        document.querySelectorAll('.modal:not(.hidden)').forEach(m => m.classList.add('hidden'));
                        // Close search panel
                        if (search.isSearchOpen) {
                            search.isSearchOpen = false;
                            document.getElementById('search-panel')?.classList.add('hidden');
                        }
                        // Reset tool to view
                        if (state.currentTool !== 'view') {
                            tools.setTool('view');
                        }
                        break;
                    // Tool hotkeys
                    case 'h': tools.setTool('highlight'); break;
                    case 'r': tools.setTool('rectangle'); break;
                    case 'c': tools.setTool('circle'); break;
                    case 'l': tools.setTool('line'); break;
                    case 'a': tools.setTool('arrow'); break;
                    case 't': tools.setTool('text'); break;
                    case 'n': tools.setTool('sticky'); break;
                    case 'v': tools.setTool('view'); break;
                }
            }
        });
    },

    bindCanvasEvents() {
        const viewer = ui.elements.viewerContainer();
        if (!viewer) return;

        // Drag-to-pan functionality
        viewer.addEventListener('mousedown', (e) => {
            // Only enable pan when in view tool and left mouse button
            if (state.currentTool === 'view' && e.button === 0) {
                state.isPanning = true;
                state.panStartX = e.clientX;
                state.panStartY = e.clientY;
                state.scrollStartLeft = viewer.scrollLeft;
                state.scrollStartTop = viewer.scrollTop;
                viewer.style.cursor = 'grabbing';
                e.preventDefault();
            } else if (state.currentTool !== 'view') {
                tools.handleMouseDown && tools.handleMouseDown(e);
            }
        });

        viewer.addEventListener('mousemove', (e) => {
            if (state.isPanning) {
                const dx = e.clientX - state.panStartX;
                const dy = e.clientY - state.panStartY;
                viewer.scrollLeft = state.scrollStartLeft - dx;
                viewer.scrollTop = state.scrollStartTop - dy;
            } else if (state.currentTool !== 'view') {
                tools.handleMouseMove && tools.handleMouseMove(e);
            }
        });

        viewer.addEventListener('mouseup', (e) => {
            if (state.isPanning) {
                state.isPanning = false;
                viewer.style.cursor = 'default';
            } else if (state.currentTool !== 'view') {
                tools.handleMouseUp && tools.handleMouseUp(e);
            }
        });

        viewer.addEventListener('mouseleave', () => {
            if (state.isPanning) {
                state.isPanning = false;
                viewer.style.cursor = 'default';
            }
        });

        // Double-click action - Smart Zoom
        viewer.addEventListener('dblclick', (e) => {
            const action = settings.get('doubleClickAction') || 'zoom';
            if (action === 'nothing') return;

            // Cycle Zoom Logic
            let newZoom;
            if (state.currentZoom < 1.0) {
                newZoom = 1.0; // Standard size
            } else if (Math.abs(state.currentZoom - 1.0) < 0.1) {
                // To Fit Width
                const page = state.pageDimensions[state.currentPage] || [600, 800];
                newZoom = (viewer.clientWidth - 40) / page[0];
            } else if (state.currentZoom < 2.0) {
                newZoom = 2.0; // Close up
            } else {
                // Reset to Fit Page
                const page = state.pageDimensions[state.currentPage] || [600, 800];
                newZoom = Math.min((viewer.clientWidth - 40) / page[0], (viewer.clientHeight - 40) / page[1]);
            }

            events.updateZoom(newZoom);

            // Optional: Scroll to mouse position
            const rect = viewer.getBoundingClientRect();
            const mouseX = e.clientX - rect.left;
            const mouseY = e.clientY - rect.top;
            // No easy way to focus on exact point without complex math here, 
            // but centering the click is a good start.
        });
    },

    bindColorPickerEvents() {
        // Sidebar color picker buttons
        document.querySelectorAll('.color-picker-row .color-btn[data-color]').forEach(btn => {
            btn.addEventListener('click', () => {
                document.querySelectorAll('.color-picker-row .color-btn').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
                state.selectedColor = btn.dataset.color;
            });
        });

        // Custom color input
        document.getElementById('custom-color')?.addEventListener('input', (e) => {
            state.selectedColor = e.target.value;
            // Remove active from preset buttons
            document.querySelectorAll('.color-picker-row .color-btn').forEach(b => b.classList.remove('active'));
        });
    },

    bindPageInputEvents() {
        const pageInput = document.getElementById('page-input');
        if (!pageInput) return;

        const navigateToPage = () => {
            let page = parseInt(pageInput.value) - 1; // Convert to 0-indexed
            if (isNaN(page)) return;
            page = Math.max(0, Math.min(page, state.totalPages - 1));
            state.currentPage = page;
            renderer.scrollToPage(page);
            ui.updateUI();
        };

        pageInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                navigateToPage();
                pageInput.blur();
            }
        });

        pageInput.addEventListener('change', () => {
            navigateToPage();
        });
    }
};
