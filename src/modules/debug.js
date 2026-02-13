/**
 * Debug Module - Verbose logging for easy debugging
 * @module debug
 */

import { state } from './state.js';

export const debug = {
    /**
     * Log message if verbose mode is enabled
     */
    log(...args) {
        if (state.verboseDebug) {
            console.log('[DEBUG]', ...args);
        }
    },
    
    /**
     * Log error if verbose mode is enabled
     */
    error(...args) {
        if (state.verboseDebug) {
            console.error('[ERROR]', ...args);
        }
    },
    
    /**
     * Log warning if verbose mode is enabled
     */
    warn(...args) {
        if (state.verboseDebug) {
            console.warn('[WARN]', ...args);
        }
    },
    
    /**
     * Log info if verbose mode is enabled
     */
    info(...args) {
        if (state.verboseDebug) {
            console.info('[INFO]', ...args);
        }
    },
    
    /**
     * Toggle verbose debug mode
     */
    toggle() {
        state.verboseDebug = !state.verboseDebug;
        console.log(`[DEBUG] Verbose logging ${state.verboseDebug ? 'enabled' : 'disabled'}`);
        return state.verboseDebug;
    },
    
    /**
     * Enable verbose debug mode
     */
    enable() {
        state.verboseDebug = true;
        console.log('[DEBUG] Verbose logging enabled');
    },
    
    /**
     * Disable verbose debug mode
     */
    disable() {
        state.verboseDebug = false;
        console.log('[DEBUG] Verbose logging disabled');
    },
    
    /**
     * Get current verbose state
     */
    isEnabled() {
        return state.verboseDebug;
    },
    
    /**
     * Initialize debug and bind keyboard shortcut (Ctrl+Shift+D)
     */
    init() {
        document.addEventListener('keydown', (e) => {
            if (e.ctrlKey && e.shiftKey && e.key === 'D') {
                e.preventDefault();
                const enabled = debug.toggle();
                alert(`Debug mode ${enabled ? 'ON' : 'OFF'}`);
            }
        });
        
        console.log('[DEBUG] Debug module initialized. Press Ctrl+Shift+D to toggle verbose logging');
    },
    
    /**
     * Create performance mark
     */
    mark(name) {
        if (state.verboseDebug) {
            performance.mark(name);
            console.log(`[PERF] Mark: ${name}`);
        }
    },
    
    /**
     * Measure performance between two marks
     */
    measure(name, startMark, endMark = name) {
        if (state.verboseDebug) {
            performance.measure(name, startMark, endMark);
            const measures = performance.getEntriesByName(name);
            const last = measures[measures.length - 1];
            console.log(`[PERF] ${name}: ${last.duration.toFixed(2)}ms`);
        }
    }
};
