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

        // Read search options
        const caseSensitive = document.getElementById('search-case-sensitive')?.checked || false;
        const wholeWord = document.getElementById('search-whole-word')?.checked || false;

        try {
            ui.showLoading('Searching...');
            search.clearResults();

            // Perform global search
            const results = await api.searchDocument(query)
                .catch(e => { throw e; });

            // Client-side filtering for case sensitivity and whole word
            let filteredResults = results;

            if (caseSensitive || wholeWord) {
                // Collect unique pages that need text fetching
                const uniquePages = [...new Set(results.map(r => r.page))];
                const pageTextMap = new Map();

                // Fetch page text in parallel
                await Promise.all(uniquePages.map(async (page) => {
                    try {
                        const text = await api.getPageText(page);
                        pageTextMap.set(page, text || '');
                    } catch (e) {
                        console.warn(`Failed to fetch text for page ${page}`, e);
                        pageTextMap.set(page, '');
                    }
                }));

                // Verify matches against fetched text
                const verifiedResults = [];
                for (const r of results) {
                    const pageText = pageTextMap.get(r.page);
                    if (pageText) {
                        let found = false;
                        if (wholeWord && caseSensitive) {
                            const regex = new RegExp(`\\b${escapeRegex(query)}\\b`);
                            found = regex.test(pageText);
                        } else if (wholeWord) {
                            const regex = new RegExp(`\\b${escapeRegex(query)}\\b`, 'i');
                            found = regex.test(pageText);
                        } else if (caseSensitive) {
                            found = pageText.includes(query);
                        }
                        if (found) verifiedResults.push(r);
                    }
                }
                filteredResults = verifiedResults;
            }

            state.searchResults = filteredResults;
            state.currentSearchIndex = -1;

            ui.hideLoading();

            const counter = document.getElementById('search-counter');
            if (counter) counter.textContent = `0/${filteredResults.length}`;

            // Populate search results panel
            const resultsPanel = document.getElementById('search-results');
            if (resultsPanel) {
                if (filteredResults.length > 0) {
                    resultsPanel.innerHTML = filteredResults.map((r, i) =>
                        `<div class="search-result-item" data-index="${i}" data-page="${r.page}">
                            <span class="search-result-page">Page ${r.page + 1}</span>
                        </div>`
                    ).join('');

                    // Wire click handlers
                    resultsPanel.querySelectorAll('.search-result-item').forEach(item => {
                        item.addEventListener('click', () => {
                            const idx = parseInt(item.dataset.index);
                            search.jumptoResult(idx);
                        });
                    });
                } else {
                    resultsPanel.innerHTML = '<div class="search-no-results">No matches found</div>';
                }
            }

            if (filteredResults.length > 0) {
                // Determine matches per page for annotation adding
                const matchesPerPage = new Map();
                filteredResults.forEach(r => {
                    if (!matchesPerPage.has(r.page)) matchesPerPage.set(r.page, []);
                    matchesPerPage.get(r.page).push(r);
                });

                // Add annotations to state
                matchesPerPage.forEach((matches, pageNum) => {
                    const pageAnns = state.annotations.get(pageNum) || [];
                    const [pageW, pageH] = state.pageDimensions[pageNum] || [0, 0];

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

                ui.showToast(`Found ${filteredResults.length} matches`);
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
        state.currentSearchIndex = index;

        // Update counter
        const counter = document.getElementById('search-counter');
        if (counter) counter.textContent = `${index + 1}/${state.searchResults.length}`;

        // Highlight active result in results panel
        document.querySelectorAll('.search-result-item').forEach((el, i) => {
            el.classList.toggle('active', i === index);
        });

        // Jump to page
        if (state.currentPage !== result.page) {
            state.currentPage = result.page;
            renderer.scrollToPage(result.page);
        }
    },

    clearResults() {
        state.searchResults = [];
        state.currentSearchIndex = -1;
        // Remove search annotations
        state.annotations.forEach((anns, page) => {
            const filtered = anns.filter(a => a.type !== 'search_highlight');
            state.annotations.set(page, filtered);
            renderer.drawAnnotations(page);
        });
        const counter = document.getElementById('search-counter');
        if (counter) counter.textContent = '0/0';

        const resultsPanel = document.getElementById('search-results');
        if (resultsPanel) resultsPanel.innerHTML = '<span class="search-placeholder">Enter search term</span>';
    }
};

function escapeRegex(string) {
    return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
