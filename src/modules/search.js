import { state } from './state.js';
import { api } from './api.js';
import { ui } from './ui.js';
import { renderer } from './renderer.js';

export const search = {
    isSearchOpen: false,

    togglePanel() {
        const searchPanel = document.getElementById('search-panel');
        if (!searchPanel) return;

        search.isSearchOpen = !search.isSearchOpen;
        searchPanel.classList.toggle('hidden', !search.isSearchOpen);
        if (search.isSearchOpen) {
            setTimeout(() => document.getElementById('ipt-search').focus(), 100);
        }
    },

    async execute() {
        const query = document.getElementById('ipt-search').value;
        if (!query) return;

        try {
            ui.showLoading('Searching...');
            // Clear previous search results from this page
            const pageAnns = state.annotations.get(state.currentPage) || [];
            const filtered = pageAnns.filter(a => a.type !== 'search_highlight');
            state.annotations.set(state.currentPage, filtered);

            // Perform search
            const results = await api.searchText(state.currentPage, query) // Need to ensure api.js has searchText
                .catch(e => { throw e; });

            ui.hideLoading();

            ui.setStatusMessage(`Found ${results.length} matches`);
            ui.showToast(`Found ${results.length} matches on this page`);

            if (results.length > 0) {
                const [pageW, pageH] = state.pageDimensions[state.currentPage] || [0, 0];

                const newAnns = results.map(r => {
                    const [x, y, w, h] = r;
                    return {
                        id: 'search-' + Math.random(),
                        type: 'search_highlight',
                        layer: 'default',
                        x: x,
                        y: pageH - y, // PDF coords usually put (0,0) at bottom-left.
                        w: w,
                        h: h,
                        color: '#ff00ff'
                    };
                });

                const currentAnns = state.annotations.get(state.currentPage) || [];
                state.annotations.set(state.currentPage, [...currentAnns, ...newAnns]);
                renderer.drawAnnotations(state.currentPage);
            }

        } catch (e) {
            console.error("Search failed:", e);
            ui.hideLoading();
            ui.showToast('Search failed: ' + e, 'error');
        }
    }
};
