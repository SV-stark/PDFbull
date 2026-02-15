import { state } from './state.js';

export const ui = {
    // Elements
    elements: {
        pageIndicator: () => document.getElementById('page-indicator'),
        zoomIndicator: () => document.getElementById('zoom-level'),
        loadingSpinner: () => document.getElementById('loading-spinner'),
        recentFilesDropdown: () => document.getElementById('recent-files-dropdown'),
        viewerContainer: () => document.getElementById('viewer-container'),
        toastContainer: () => document.getElementById('toast-container'),
        statusDoc: () => document.querySelector('#status-doc span'),
        statusPages: () => document.getElementById('status-pages'),
        statusDimensions: () => document.getElementById('status-dimensions'),
        statusMessage: () => document.querySelector('#status-message span'),
        undoBtn: () => document.getElementById('btn-undo'),
        redoBtn: () => document.getElementById('btn-redo'),
        tabsContainer: () => document.getElementById('tabs-container'),
        sidebar: () => document.getElementById('sidebar-left'), // Default to left for backward compatibility
        sidebarLeft: () => document.getElementById('sidebar-left'),
        sidebarRight: () => document.getElementById('sidebar-right'),
        currentTool: () => document.getElementById('current-tool'),
        pagesContainer: () => document.getElementById('pages-container'),
    },

    // Toast
    showToast(message, type = 'info', duration = 3000) {
        const container = ui.elements.toastContainer();
        if (!container) return;

        const toast = document.createElement('div');
        toast.className = `toast ${type}`;

        const icon = type === 'success' ? 'ph-check-circle' :
            type === 'error' ? 'ph-x-circle' : 'ph-info';

        toast.innerHTML = `
      <i class="ph ${icon}"></i>
      <span>${message}</span>
    `;

        container.appendChild(toast);

        setTimeout(() => {
            toast.classList.add('hide');
            setTimeout(() => toast.remove(), 300);
        }, duration);
    },

    // Loading
    showLoading(text = 'Loading...') {
        const spinner = ui.elements.loadingSpinner();
        if (spinner) {
            spinner.querySelector('.loading-text').textContent = text;
            spinner.classList.remove('hidden');
        }
    },

    hideLoading() {
        const spinner = ui.elements.loadingSpinner();
        if (spinner) spinner.classList.add('hidden');
    },

    showSkeleton() {
        const container = ui.elements.viewerContainer();
        if (container) container.classList.add('skeleton-loading');
    },

    hideSkeleton() {
        const container = ui.elements.viewerContainer();
        if (container) container.classList.remove('skeleton-loading');
    },

    // Status Bar
    updateStatusBar() {
        const { statusDoc, statusPages, statusDimensions } = ui.elements;

        if (state.currentDoc) {
            const fileName = state.currentDoc.split(/[/\\]/).pop();
            const elDoc = statusDoc();
            if (elDoc) elDoc.textContent = fileName;

            const elPages = statusPages();
            if (elPages) elPages.textContent = `${state.totalPages} pages`;

            const elDim = statusDimensions();
            if (elDim) {
                if (state.pageDimensions[state.currentPage]) {
                    const [w, h] = state.pageDimensions[state.currentPage];
                    elDim.textContent = `${w}×${h}px`;
                } else {
                    elDim.textContent = '-';
                }
            }
        } else {
            const elDoc = statusDoc();
            if (elDoc) elDoc.textContent = 'No document open';

            const elPages = statusPages();
            if (elPages) elPages.textContent = '0 pages';

            const elDim = statusDimensions();
            if (elDim) elDim.textContent = '-';
        }
    },

    setStatusMessage(message) {
        const el = ui.elements.statusMessage();
        if (el) el.textContent = message;
    },

    // Undo/Redo
    updateUndoRedoButtons() {
        const { undoBtn, redoBtn } = ui.elements;
        const ub = undoBtn();
        const rb = redoBtn();

        if (ub) {
            ub.disabled = state.historyIndex <= 0;
            ub.style.opacity = state.historyIndex <= 0 ? '0.5' : '1';
        }
        if (rb) {
            rb.disabled = state.historyIndex >= state.history.length - 1;
            rb.style.opacity = state.historyIndex >= state.history.length - 1 ? '0.5' : '1';
        }
    },

    // Recent Files
    updateRecentFilesDropdown(recentFiles, openNewTabCallback) {
        const dropdown = ui.elements.recentFilesDropdown();
        if (!dropdown) return;

        if (recentFiles.length === 0) {
            dropdown.innerHTML = `
          <div class="recent-file-empty">
            <i class="ph ph-clock" style="font-size: 24px; margin-bottom: 8px;"></i>
            <div>No recent files</div>
          </div>
        `;
            return;
        }

        dropdown.innerHTML = recentFiles.map(file => `
        <div class="recent-file-item" data-path="${file.path}">
          <i class="ph ph-file-pdf recent-file-icon"></i>
          <div class="recent-file-info">
            <div class="recent-file-name">${file.name}</div>
            <div class="recent-file-path">${file.path}</div>
          </div>
        </div>
      `).join('');

        dropdown.querySelectorAll('.recent-file-item').forEach(item => {
            item.addEventListener('click', async () => {
                const path = item.getAttribute('data-path');
                if (openNewTabCallback) await openNewTabCallback(path);
                dropdown.classList.remove('visible');
            });
        });
    },

    updateUI() {
        const pi = ui.elements.pageIndicator();
        const zi = ui.elements.zoomIndicator();

        if (pi) {
            if (state.totalPages === 0) {
                pi.textContent = '- / -';
            } else {
                pi.textContent = `${state.currentPage + 1} / ${state.totalPages}`;
            }
        }

        if (zi) {
            zi.textContent = `${Math.round(state.currentZoom * 100)}%`;
        }

        const pageInput = document.getElementById('page-input');
        if (pageInput) pageInput.value = state.currentPage + 1;

        // Update Bookmark Icon
        ui.updateBookmarkUI(state.currentPage);
    },

    updateBookmarkUI(pageNum) {
        const btn = document.getElementById('btn-bookmark');
        if (btn) {
            const isBookmarked = state.bookmarks.has(pageNum);
            const icon = btn.querySelector('i');
            if (isBookmarked) {
                icon.className = 'ph ph-bookmark-simple-fill';
                icon.style.color = 'var(--accent-color)';
            } else {
                icon.className = 'ph ph-bookmark-simple';
                icon.style.color = '';
            }
        }
    },

    createTabUI(tabId, docInfo, switchToTabCallback, closeTabCallback) {
        const tabsContainer = ui.elements.tabsContainer();
        const tab = document.createElement('div');
        tab.className = 'tab';
        tab.id = tabId;
        tab.innerHTML = `
        <i class="ph ph-file-pdf"></i>
        <span class="tab-title">${docInfo.name}</span>
        <button class="tab-close" data-tab="${tabId}">
          <i class="ph ph-x"></i>
        </button>
      `;

        tab.addEventListener('click', (e) => {
            if (!e.target.closest('.tab-close')) {
                switchToTabCallback(tabId);
            }
        });

        tab.querySelector('.tab-close').addEventListener('click', (e) => {
            e.stopPropagation(); // Prevent tab switch when closing
            closeTabCallback(tabId);
        });

        tabsContainer.appendChild(tab);
    },

    updateActiveTab(tabId) {
        document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
        const activeTab = document.getElementById(tabId);
        if (activeTab) activeTab.classList.add('active');
    },

    // Batch
    renderBatchControls(size, events) {
        const sidebar = ui.elements.sidebarLeft();
        const existing = document.getElementById('batch-controls');
        if (existing) existing.remove();

        const controls = document.createElement('div');
        controls.id = 'batch-controls';
        controls.className = 'batch-controls';
        controls.innerHTML = `
        <div class="sidebar-header">Batch Operations (${size} selected)</div>
        <div class="tools-container">
          <button class="tool-btn" id="btn-batch-export">
            <i class="ph ph-export"></i>
            <span>Export Selected</span>
          </button>
          <button class="tool-btn" id="btn-batch-delete">
            <i class="ph ph-trash"></i>
            <span>Delete Selected</span>
          </button>
          <button class="tool-btn" id="btn-batch-clear">
            <i class="ph ph-x"></i>
            <span>Clear Selection</span>
          </button>
        </div>
      `;

        sidebar.insertBefore(controls, sidebar.firstChild);

        // Bind events
        document.getElementById('btn-batch-export')?.addEventListener('click', events.onExport);
        document.getElementById('btn-batch-delete')?.addEventListener('click', events.onDelete);
        document.getElementById('btn-batch-clear')?.addEventListener('click', events.onClear);
    },

    hideBatchControls() {
        const controls = document.getElementById('batch-controls');
        if (controls) controls.remove();
    },

    renderOutline(outline) {
        const container = document.getElementById('outline-list');
        if (!container) return;
        
        container.innerHTML = '';
        
        if (!outline || outline.length === 0) {
            container.innerHTML = '<div class="outline-empty">No table of contents</div>';
            return;
        }
        
        const renderItem = (item, level = 0) => {
            const div = document.createElement('div');
            div.className = 'outline-item';
            div.style.paddingLeft = `${12 + level * 16}px`;
            
            const icon = item.children?.length > 0 ? 'ph-folder' : 'ph-file';
            div.innerHTML = `
                <i class="ph ${icon}"></i>
                <span class="outline-title">${item.title}</span>
                ${item.page !== null ? `<span class="outline-page">${item.page + 1}</span>` : ''}
            `;
            
            if (item.page !== null) {
                div.addEventListener('click', () => {
                    const { renderer } = window.__PDFBULL__?.modules || {};
                    if (renderer && item.page !== null) {
                        state.currentPage = item.page;
                        renderer.scrollToPage(item.page);
                    }
                });
            }
            
            container.appendChild(div);
            
            if (item.children?.length > 0) {
                item.children.forEach(child => renderItem(child, level + 1));
            }
        };
        
        outline.forEach(item => renderItem(item));
    }
};
