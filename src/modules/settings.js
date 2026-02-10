export const settings = {
    defaults: {
        theme: 'dark',
        accentColor: '#646cff',
        sidebarWidth: 250,
        showToolbarLabels: true,
        defaultZoom: '100',
        autoSaveInterval: 30,
        restoreSession: false,
        smoothScroll: true,
        doubleClickAction: 'nothing',
        cacheSize: 15,
        renderQuality: 'medium',
        hardwareAccel: true,
        recentFilesLimit: 10,
        autoOpenLast: false,
        defaultSavePath: '',
        defaultAnnoColor: '#ffeb3b',
        stickyNoteWidth: 150,
        stickyNoteHeight: 100
    },

    load() {
        const saved = JSON.parse(localStorage.getItem('appSettings') || '{}');
        // Migration logic
        const savedTheme = localStorage.getItem('theme');
        if (savedTheme && !localStorage.getItem('appSettings')) {
            saved.theme = savedTheme;
            localStorage.removeItem('theme');
        }
        return { ...this.defaults, ...saved };
    },

    save(newSettings) {
        localStorage.setItem('appSettings', JSON.stringify(newSettings));
    },

    get(key) {
        const s = this.load();
        return s[key];
    },

    set(key, value) {
        const s = this.load();
        s[key] = value;
        this.save(s);
    },

    // Helper to adjust color
    adjustColor(color, amount) {
        if (!color) return '#000000';
        const hex = color.replace('#', '');
        const r = Math.max(0, Math.min(255, parseInt(hex.substr(0, 2), 16) + amount));
        const g = Math.max(0, Math.min(255, parseInt(hex.substr(2, 2), 16) + amount));
        const b = Math.max(0, Math.min(255, parseInt(hex.substr(4, 2), 16) + amount));
        return `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`;
    }
};

export function applySettings() {
    const s = settings.load();

    // Theme
    document.documentElement.setAttribute('data-theme', s.theme);

    // Accent color
    document.documentElement.style.setProperty('--accent-color', s.accentColor);
    document.documentElement.style.setProperty('--accent-hover', settings.adjustColor(s.accentColor, -20));

    // Sidebar width (applies to right sidebar where tools are)
    const sidebarRight = document.getElementById('sidebar-right');
    if (sidebarRight) sidebarRight.style.width = s.sidebarWidth + 'px';

    // Toolbar labels
    const toolbar = document.querySelector('.toolbar');
    if (toolbar) {
        if (!s.showToolbarLabels) {
            toolbar.classList.add('hide-labels');
        } else {
            toolbar.classList.remove('hide-labels');
        }
    }
}
