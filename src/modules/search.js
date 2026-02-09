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
            search.clearResults();

            // Perform global search
            const results = await api.searchDocument(query)
                .catch(e => { throw e; });

            state.searchResults = results;
            state.currentSearchIndex = -1;

            ui.hideLoading();

            const counter = document.getElementById('search-counter');
            if (counter) counter.textContent = `0/${results.length}`;

            if (results.length > 0) {
                // Determine matches per page for annotation adding
                const matchesPerPage = new Map();
                results.forEach(r => {
                    if (!matchesPerPage.has(r.page)) matchesPerPage.set(r.page, []);
                    matchesPerPage.get(r.page).push(r);
                });

                // Add annotations to state
                matchesPerPage.forEach((matches, pageNum) => {
                    const pageAnns = state.annotations.get(pageNum) || [];
                    const [pageW, pageH] = state.pageDimensions[pageNum] || [0, 0]; // Note: dimensions might be lazy loaded?
                    // If dimensions are missing, we might have issues flipping Y. 
                    // However, pageDimensions are usually populated on open or first render.
                    // If not, we might need to fetch them.

                    const newAnns = matches.map(r => ({
                        id: 'search-' + Math.random(),
                        type: 'search_highlight',
                        layer: 'default',
                        x: r.x,
                        y: pageH - r.y,
                        w: r.w,
                        h: r.h,
                        color: 'rgba(255, 255, 0, 0.4)'
                    }));

                    state.annotations.set(pageNum, [...pageAnns, ...newAnns]);
                    renderer.drawAnnotations(pageNum);
                });

                ui.showToast(`Found ${results.length} matches`);
                search.nextResult(); // Jump to first result
            } else {
                ui.showToast('No matches found');
            }

        } catch (e) {
            console.error("Search failed:", e);
            ui.hideLoading();
            ui.showToast('Search failed: ' + e.message, 'error');
        }
    },

    nextResult() {
        if (state.searchResults.length === 0) return;
        state.currentSearchIndex = (state.currentSearchIndex + 1) % state.searchResults.length;
        search.jumptoResult(state.currentSearchIndex);
    },

    prevResult() {
        if (state.searchResults.length === 0) return;
        state.currentSearchIndex = (state.currentSearchIndex - 1 + state.searchResults.length) % state.searchResults.length;
        search.jumptoResult(state.currentSearchIndex);
    },

    jumptoResult(index) {
        if (index < 0 || index >= state.searchResults.length) return;
        const result = state.searchResults[index];

        // Update counter
        const counter = document.getElementById('search-counter');
        if (counter) counter.textContent = `${index + 1}/${state.searchResults.length}`;

        // Jump to page
        if (state.currentPage !== result.page) {
            state.currentPage = result.page;
            renderer.scrollToPage(result.page);
        } else {
            // If already on page, make sure we scroll to the specific element?
            // Since we render whole page, scrollToPage is enough to show the page.
            // Maybe advanced: scroll to specific Y?
            const pageContainer = document.getElementById(`page-container-${result.page}`);
            // Calculate Y offset %?
            // For now, page functionality is good enough.
        }

        // Highlight current Result differently?
        // We could update the specific annotation color.
        // For efficiency, we might just blink it or rely on the user seeing it.
        // Let's keep it simple for now.
    },

    clearResults() {
        state.searchResults = [];
        state.currentSearchIndex = -1;
        // Remove search annotations
        state.annotations.forEach((anns, page) => {
            const filtered = anns.filter(a => a.type !== 'search_highlight');
            state.annotations.set(page, filtered);
            renderer.drawAnnotations(page); // Re-render to clear
        });
        const counter = document.getElementById('search-counter');
        if (counter) counter.textContent = '0/0';
    }
};
