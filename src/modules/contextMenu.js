import { state } from './state.js';
import { tools } from './tools.js';

export class ContextMenu {
    constructor() {
        this.isOpen = false;
        this.init();
    }

    init() {
        this.menu = document.getElementById('custom-context-menu');
        if (!this.menu) return;

        document.addEventListener('click', () => this.close());
        this.menu.addEventListener('click', (e) => e.stopPropagation());
    }

    show(x, y) {
        this.isOpen = true;
        this.menu.classList.remove('hidden');

        // Adjust position if near edges
        const menuWidth = 180;
        const menuHeight = 250;
        if (x + menuWidth > window.innerWidth) x -= menuWidth;
        if (y + menuHeight > window.innerHeight) y -= menuHeight;

        this.menu.style.left = `${x}px`;
        this.menu.style.top = `${y}px`;

        this.render();
    }

    close() {
        this.isOpen = false;
        this.menu.classList.add('hidden');
    }

    render() {
        // Dynamic context: show "Copy" if there is text selection?
        // For simplicity, we'll keep a standard set of quick tools
        this.menu.innerHTML = `
            <div class="context-item" onclick="document.dispatchEvent(new CustomEvent('app:tool', {detail: 'highlight'}))">
                <i class="ph ph-highlighter"></i>
                <span>Highlight</span>
                <kbd>H</kbd>
            </div>
            <div class="context-item" onclick="document.dispatchEvent(new CustomEvent('app:tool', {detail: 'sticky'}))">
                <i class="ph ph-note"></i>
                <span>Add Note</span>
                <kbd>N</kbd>
            </div>
            <div class="context-item" onclick="document.dispatchEvent(new CustomEvent('app:tool', {detail: 'text'}))">
                <i class="ph ph-text-t"></i>
                <span>Add Text</span>
                <kbd>T</kbd>
            </div>
            <div class="context-separator"></div>
            <div class="context-item" onclick="document.getElementById('btn-undo').click()">
                <i class="ph ph-arrow-u-up-left"></i>
                <span>Undo</span>
                <kbd>Ctrl+Z</kbd>
            </div>
            <div class="context-item" onclick="document.getElementById('btn-redo').click()">
                <i class="ph ph-arrow-u-up-right"></i>
                <span>Redo</span>
                <kbd>Ctrl+Y</kbd>
            </div>
            <div class="context-separator"></div>
            <div class="context-item" onclick="document.getElementById('btn-zoom-in').click()">
                <i class="ph ph-plus"></i>
                <span>Zoom In</span>
            </div>
            <div class="context-item" onclick="document.getElementById('btn-zoom-out').click()">
                <i class="ph ph-minus"></i>
                <span>Zoom Out</span>
            </div>
            <div class="context-item" onclick="document.getElementById('btn-rotate').click()">
                <i class="ph ph-arrow-clockwise"></i>
                <span>Rotate</span>
            </div>
        `;
    }
}
