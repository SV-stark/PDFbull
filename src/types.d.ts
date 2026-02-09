/**
 * PDFbull Type Definitions
 * 
 * This file provides TypeScript-style type definitions for JavaScript modules.
 * It enables IntelliSense and type checking in VS Code when used with jsconfig.json.
 */

// ═══════════════════════════════════════════════════════════════════════════
// DOCUMENT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Represents an open document tab
 */
interface DocumentState {
    /** Unique tab identifier */
    id: string;
    /** Absolute file path */
    path: string;
    /** File name without path */
    name: string;
    /** Total number of pages */
    totalPages: number;
    /** Current page index (0-based) */
    currentPage: number;
    /** Current zoom level */
    zoom: number;
}

// ═══════════════════════════════════════════════════════════════════════════
// ANNOTATION TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Annotation types supported by PDFbull
 */
type AnnotationType =
    | 'highlight'
    | 'rectangle'
    | 'circle'
    | 'line'
    | 'arrow'
    | 'text'
    | 'note'
    | 'search_highlight';

/**
 * Base annotation properties
 */
interface AnnotationBase {
    /** Annotation type */
    type: AnnotationType;
    /** X coordinate (in page units) */
    x: number;
    /** Y coordinate (in page units) */
    y: number;
    /** Hex color string */
    color: string;
    /** Layer identifier */
    layer?: string;
}

/**
 * Shape annotation (rectangle, circle, highlight)
 */
interface ShapeAnnotation extends AnnotationBase {
    type: 'highlight' | 'rectangle' | 'circle';
    /** Width */
    w: number;
    /** Height */
    h: number;
}

/**
 * Line annotation (line, arrow)
 */
interface LineAnnotation extends AnnotationBase {
    type: 'line' | 'arrow';
    /** Start X */
    x1: number;
    /** Start Y */
    y1: number;
    /** End X */
    x2: number;
    /** End Y */
    y2: number;
}

/**
 * Text annotation (text box, sticky note)
 */
interface TextAnnotation extends AnnotationBase {
    type: 'text' | 'note';
    /** Text content */
    text: string;
    /** Width (optional) */
    w?: number;
    /** Height (optional) */
    h?: number;
}

/**
 * Union type for all annotations
 */
type Annotation = ShapeAnnotation | LineAnnotation | TextAnnotation;

// ═══════════════════════════════════════════════════════════════════════════
// APPLICATION STATE
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Global application state
 */
interface AppState {
    /** Currently active document path */
    currentDoc: string | null;
    /** Total pages in current document */
    totalPages: number;
    /** Current page index (0-based) */
    currentPage: number;
    /** Current zoom level */
    currentZoom: number;
    /** Render scale factor */
    renderScale: number;
    /** Page dimensions cache */
    pageDimensions: Array<[number, number]>;
    /** Page render cache */
    pageCache: Map<number, ImageData>;
    /** Annotations per page */
    annotations: Map<number, Annotation[]>;
    /** Currently selected tool */
    currentTool: AnnotationType | null;
    /** Currently selected color */
    currentColor: string;
    /** History stack for undo */
    history: Array<any>;
    /** Current history index */
    historyIndex: number;
    /** Open document tabs */
    openDocuments: Map<string, DocumentState>;
    /** Active tab ID */
    activeTabId: string | null;
    /** Tab counter for unique IDs */
    tabCounter: number;
    /** Visible layers */
    visibleLayers: Set<string>;
    /** Filter state */
    activeFilter: string | null;
    /** Scroll lock state */
    isScrollLocked: boolean;
}

// ═══════════════════════════════════════════════════════════════════════════
// SETTINGS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Application settings
 */
interface Settings {
    /** UI Theme */
    theme: 'dark' | 'light' | 'high-contrast';
    /** Accent color hex */
    accentColor: string;
    /** Sidebar width in pixels */
    sidebarWidth: number;
    /** Show toolbar labels */
    showToolbarLabels: boolean;
    /** Default zoom level */
    defaultZoom: 'fit-width' | 'fit-page' | number;
    /** Auto-save interval in seconds (0 = disabled) */
    autosaveInterval: number;
    /** Restore previous session */
    restoreSession: boolean;
    /** Smooth scrolling */
    smoothScrolling: boolean;
    /** Double-click action */
    doubleClickAction: 'nothing' | 'zoom' | 'fit-width';
    /** Page cache size */
    cacheSize: number;
    /** Render quality */
    renderQuality: 'low' | 'medium' | 'high';
    /** Hardware acceleration */
    hardwareAcceleration: boolean;
    /** Recent files limit */
    recentFilesLimit: number;
    /** Auto-open last file */
    autoOpenLastFile: boolean;
    /** Default save path */
    defaultSavePath: string;
    /** Default annotation color */
    defaultAnnotationColor: string;
    /** Default sticky note dimensions */
    stickyNoteWidth: number;
    stickyNoteHeight: number;
}

// ═══════════════════════════════════════════════════════════════════════════
// TAURI API
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Tauri dialog options
 */
interface SaveDialogOptions {
    filters?: Array<{ name: string; extensions: string[] }>;
    defaultPath?: string;
    title?: string;
}

interface OpenDialogOptions {
    filters?: Array<{ name: string; extensions: string[] }>;
    title?: string;
    multiple?: boolean;
    directory?: boolean;
}

// ═══════════════════════════════════════════════════════════════════════════
// GLOBAL AUGMENTATION
// ═══════════════════════════════════════════════════════════════════════════

declare global {
    interface Window {
        __TAURI__: {
            core: {
                invoke<T = any>(command: string, args?: Record<string, any>): Promise<T>;
            };
            dialog: {
                save(options?: SaveDialogOptions): Promise<string | null>;
                open(options?: OpenDialogOptions): Promise<string | string[] | null>;
            };
        };
    }
}

export {
    DocumentState,
    AnnotationType,
    Annotation,
    ShapeAnnotation,
    LineAnnotation,
    TextAnnotation,
    AppState,
    Settings,
    SaveDialogOptions,
    OpenDialogOptions
};
