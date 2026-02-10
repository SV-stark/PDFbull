import { state } from './state.js';
import { events } from './events.js';
import { ui } from './ui.js';

export class CommandPalette {
    constructor() {
        this.isOpen = false;
        this.commands = this.getCommands();
        this.activeIndex = 0;
        this.filteredCommands = [...this.commands];
        this.init();
    }

    init() {
        this.overlay = document.getElementById('command-palette-overlay');
        this.input = document.getElementById('command-palette-input');
        this.list = document.getElementById('command-palette-list');

        if (!this.input) return;

        this.input.addEventListener('input', (e) => this.filterCommands(e.target.value));
        this.input.addEventListener('keydown', (e) => this.handleKeyDown(e));
        this.overlay.addEventListener('click', (e) => {
            if (e.target === this.overlay) this.close();
        });
    }

    getCommands() {
        return [
            { id: 'open', name: 'Open PDF', icon: 'ph-file-pdf', action: () => document.getElementById('btn-open').click() },
            { id: 'save', name: 'Save Changes', icon: 'ph-floppy-disk', action: () => document.dispatchEvent(new CustomEvent('app:save')) },
            { id: 'zoom-in', name: 'Zoom In', icon: 'ph-plus', action: () => events.updateZoom(state.currentZoom * 1.2) },
            { id: 'zoom-out', name: 'Zoom Out', icon: 'ph-minus', action: () => events.updateZoom(state.currentZoom / 1.2) },
            { id: 'fit-width', name: 'Fit to Width', icon: 'ph-arrows-out-horizontal', action: () => document.getElementById('btn-fit-width').click() },
            { id: 'fit-page', name: 'Fit to Page', icon: 'ph-arrows-out', action: () => document.getElementById('btn-fit-page').click() },
            { id: 'rotate', name: 'Rotate Page', icon: 'ph-arrow-clockwise', action: () => document.getElementById('btn-rotate').click() },
            { id: 'dark-mode', name: 'Theme: Dark', icon: 'ph-moon', action: () => this.setTheme('dark') },
            { id: 'light-mode', name: 'Theme: Light', icon: 'ph-sun', action: () => this.setTheme('light') },
            { id: 'hc-mode', name: 'Theme: High Contrast', icon: 'ph-circle-half-tilt', action: () => this.setTheme('high-contrast') },
            { id: 'toggle-sidebar-l', name: 'Toggle Thumbnails', icon: 'ph-sidebar', action: () => document.getElementById('btn-sidebar-left-toggle').click() },
            { id: 'toggle-sidebar-r', name: 'Toggle Tools', icon: 'ph-sidebar', action: () => document.getElementById('btn-sidebar-right-toggle').click() },
            { id: 'ocr', name: 'Extract Text (OCR)', icon: 'ph-text-aa', action: () => document.getElementById('btn-ocr').click() },
            { id: 'crop', name: 'Auto Crop', icon: 'ph-crop', action: () => document.getElementById('btn-crop').click() },
            { id: 'export', name: 'Export Options', icon: 'ph-export', action: () => document.getElementById('btn-export').click() },
            { id: 'shortcuts', name: 'Keyboard Shortcuts', icon: 'ph-keyboard', action: () => document.getElementById('keyboard-help-modal').classList.remove('hidden') },
            { id: 'settings', name: 'Open Settings', icon: 'ph-gear', action: () => document.getElementById('btn-settings').click() },
        ];
    }

    setTheme(theme) {
        document.documentElement.setAttribute('data-theme', theme);
        ui.showToast(`Switched to ${theme} theme`);
    }

    open() {
        this.isOpen = true;
        this.overlay.classList.remove('hidden');
        this.input.value = '';
        this.filterCommands('');
        setTimeout(() => this.input.focus(), 10);
    }

    close() {
        this.isOpen = false;
        this.overlay.classList.add('hidden');
    }

    filterCommands(query) {
        const q = query.toLowerCase();
        this.filteredCommands = this.commands.filter(c =>
            c.name.toLowerCase().includes(q)
        );
        this.activeIndex = 0;
        this.render();
    }

    render() {
        this.list.innerHTML = this.filteredCommands.map((c, i) => `
            <div class="command-item ${i === this.activeIndex ? 'active' : ''}" data-index="${i}">
                <i class="ph ${c.icon}"></i>
                <span>${c.name}</span>
                <span class="command-id">${c.id}</span>
            </div>
        `).join('');

        // Scroll active into view
        const activeEl = this.list.children[this.activeIndex];
        if (activeEl) {
            activeEl.scrollIntoView({ block: 'nearest' });
        }

        // Add click listeners
        Array.from(this.list.children).forEach((el, i) => {
            el.onclick = () => {
                this.activeIndex = i;
                this.execute();
            };
        });
    }

    handleKeyDown(e) {
        if (e.key === 'ArrowDown') {
            e.preventDefault();
            this.activeIndex = (this.activeIndex + 1) % this.filteredCommands.length;
            this.render();
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            this.activeIndex = (this.activeIndex - 1 + this.filteredCommands.length) % this.filteredCommands.length;
            this.render();
        } else if (e.key === 'Enter') {
            e.preventDefault();
            this.execute();
        } else if (e.key === 'Escape') {
            e.preventDefault();
            this.close();
        }
    }

    execute() {
        const cmd = this.filteredCommands[this.activeIndex];
        if (cmd) {
            this.close();
            cmd.action();
        }
    }
}
