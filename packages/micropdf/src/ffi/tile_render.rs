//! Multi-threaded tile-based rendering
//!
//! This module provides tile-based parallel rendering for PDF pages,
//! enabling efficient use of multi-core processors for large documents.

use std::ffi::c_int;
use std::sync::LazyLock;

use crate::ffi::{Handle, HandleStore, new_handle};
use crate::fitz::geometry::{IRect, Rect};
use crate::fitz::pixmap::Pixmap;

// ============================================================================
// Handle Management
// ============================================================================

/// Handle store for tile renderers
static TILE_RENDERERS: LazyLock<HandleStore<TileRenderer>> = LazyLock::new(HandleStore::new);

/// Handle store for render tasks
static RENDER_TASKS: LazyLock<HandleStore<RenderTask>> = LazyLock::new(HandleStore::new);

// ============================================================================
// Tile Configuration
// ============================================================================

/// Default tile size in pixels
pub const DEFAULT_TILE_SIZE: u32 = 256;

/// Minimum tile size
pub const MIN_TILE_SIZE: u32 = 64;

/// Maximum tile size
pub const MAX_TILE_SIZE: u32 = 1024;

/// Tile rendering status
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileStatus {
    /// Tile is pending rendering
    Pending = 0,
    /// Tile is currently being rendered
    InProgress = 1,
    /// Tile rendering completed successfully
    Complete = 2,
    /// Tile rendering failed
    Failed = 3,
    /// Tile was cancelled
    Cancelled = 4,
}

// ============================================================================
// Tile Structure
// ============================================================================

/// A single tile in the rendering grid
#[derive(Clone)]
pub struct Tile {
    /// Tile index (row * cols + col)
    pub index: usize,
    /// Row in the tile grid
    pub row: u32,
    /// Column in the tile grid
    pub col: u32,
    /// Tile bounds in page coordinates
    pub bounds: Rect,
    /// Tile bounds in pixel coordinates
    pub pixel_rect: IRect,
    /// Rendering status
    pub status: TileStatus,
    /// Rendered pixmap (if complete)
    pub pixmap: Option<Pixmap>,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl Tile {
    /// Create a new tile
    pub fn new(index: usize, row: u32, col: u32, bounds: Rect, pixel_rect: IRect) -> Self {
        Self {
            index,
            row,
            col,
            bounds,
            pixel_rect,
            status: TileStatus::Pending,
            pixmap: None,
            error: None,
        }
    }
}

// ============================================================================
// Tile Renderer
// ============================================================================

/// Configuration for tile rendering
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TileConfig {
    /// Tile width in pixels
    pub tile_width: u32,
    /// Tile height in pixels
    pub tile_height: u32,
    /// Number of threads (0 = auto)
    pub num_threads: u32,
    /// Include alpha channel
    pub alpha: bool,
    /// Scale factor
    pub scale: f32,
    /// Rotation in degrees
    pub rotation: f32,
}

impl Default for TileConfig {
    fn default() -> Self {
        Self {
            tile_width: DEFAULT_TILE_SIZE,
            tile_height: DEFAULT_TILE_SIZE,
            num_threads: 0, // Auto-detect
            alpha: true,
            scale: 1.0,
            rotation: 0.0,
        }
    }
}

/// Tile renderer for parallel page rendering
pub struct TileRenderer {
    /// Rendering configuration
    config: TileConfig,
    /// Page bounds
    page_bounds: Rect,
    /// Total width in pixels
    total_width: u32,
    /// Total height in pixels
    total_height: u32,
    /// Number of tile columns
    cols: u32,
    /// Number of tile rows
    rows: u32,
    /// All tiles
    tiles: Vec<Tile>,
    /// Completed tile count
    completed: usize,
    /// Failed tile count
    failed: usize,
}

impl TileRenderer {
    /// Create a new tile renderer for a page
    pub fn new(page_bounds: Rect, config: TileConfig) -> Self {
        let scale = config.scale;

        // Calculate total pixel dimensions
        let total_width = ((page_bounds.x1 - page_bounds.x0) * scale) as u32;
        let total_height = ((page_bounds.y1 - page_bounds.y0) * scale) as u32;

        // Calculate grid dimensions
        let cols = (total_width + config.tile_width - 1) / config.tile_width;
        let rows = (total_height + config.tile_height - 1) / config.tile_height;

        // Create tiles
        let mut tiles = Vec::with_capacity((rows * cols) as usize);
        for row in 0..rows {
            for col in 0..cols {
                let index = (row * cols + col) as usize;

                // Pixel coordinates
                let px0 = col * config.tile_width;
                let py0 = row * config.tile_height;
                let px1 = ((col + 1) * config.tile_width).min(total_width);
                let py1 = ((row + 1) * config.tile_height).min(total_height);

                let pixel_rect = IRect::new(px0 as i32, py0 as i32, px1 as i32, py1 as i32);

                // Page coordinates (inverse scale)
                let x0 = page_bounds.x0 + (px0 as f32 / scale);
                let y0 = page_bounds.y0 + (py0 as f32 / scale);
                let x1 = page_bounds.x0 + (px1 as f32 / scale);
                let y1 = page_bounds.y0 + (py1 as f32 / scale);

                let bounds = Rect::new(x0, y0, x1, y1);

                tiles.push(Tile::new(index, row, col, bounds, pixel_rect));
            }
        }

        Self {
            config,
            page_bounds,
            total_width,
            total_height,
            cols,
            rows,
            tiles,
            completed: 0,
            failed: 0,
        }
    }

    /// Get the number of tiles
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Get a tile by index
    pub fn get_tile(&self, index: usize) -> Option<&Tile> {
        self.tiles.get(index)
    }

    /// Get a mutable tile by index
    pub fn get_tile_mut(&mut self, index: usize) -> Option<&mut Tile> {
        self.tiles.get_mut(index)
    }

    /// Get tiles in a specific row
    pub fn get_row(&self, row: u32) -> Vec<&Tile> {
        self.tiles.iter().filter(|t| t.row == row).collect()
    }

    /// Get tiles in a specific column
    pub fn get_col(&self, col: u32) -> Vec<&Tile> {
        self.tiles.iter().filter(|t| t.col == col).collect()
    }

    /// Get all pending tiles
    pub fn pending_tiles(&self) -> Vec<usize> {
        self.tiles
            .iter()
            .filter(|t| t.status == TileStatus::Pending)
            .map(|t| t.index)
            .collect()
    }

    /// Get progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.tiles.is_empty() {
            return 1.0;
        }
        self.completed as f32 / self.tiles.len() as f32
    }

    /// Check if all tiles are complete
    pub fn is_complete(&self) -> bool {
        self.completed + self.failed == self.tiles.len()
    }

    /// Mark a tile as complete with its pixmap
    pub fn complete_tile(&mut self, index: usize, pixmap: Pixmap) {
        if let Some(tile) = self.tiles.get_mut(index) {
            tile.status = TileStatus::Complete;
            tile.pixmap = Some(pixmap);
            self.completed += 1;
        }
    }

    /// Mark a tile as failed
    pub fn fail_tile(&mut self, index: usize, error: String) {
        if let Some(tile) = self.tiles.get_mut(index) {
            tile.status = TileStatus::Failed;
            tile.error = Some(error);
            self.failed += 1;
        }
    }

    /// Composite all completed tiles into a single pixmap
    pub fn composite(&self) -> Option<Pixmap> {
        // Create the output pixmap
        let mut output = Pixmap::new(
            None,
            self.total_width as i32,
            self.total_height as i32,
            self.config.alpha,
        )
        .ok()?;

        // Copy each completed tile
        for tile in &self.tiles {
            if let Some(ref tile_pix) = tile.pixmap {
                let dest_x = tile.pixel_rect.x0;
                let dest_y = tile.pixel_rect.y0;

                // Copy tile pixels to output
                let src_stride = tile_pix.stride();
                let dst_stride = output.stride();
                let n = tile_pix.n() as usize;
                let w = tile_pix.width() as usize;
                let h = tile_pix.height() as usize;

                let src_data = tile_pix.samples();
                let dst_data = output.samples_mut();

                for y in 0..h {
                    let src_row = y * src_stride;
                    let dst_row = (dest_y as usize + y) * dst_stride + dest_x as usize * n;

                    if src_row + w * n <= src_data.len() && dst_row + w * n <= dst_data.len() {
                        dst_data[dst_row..dst_row + w * n]
                            .copy_from_slice(&src_data[src_row..src_row + w * n]);
                    }
                }
            }
        }

        Some(output)
    }

    /// Get configuration
    pub fn config(&self) -> &TileConfig {
        &self.config
    }

    /// Get total dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.total_width, self.total_height)
    }

    /// Get grid dimensions
    pub fn grid(&self) -> (u32, u32) {
        (self.cols, self.rows)
    }
}

// ============================================================================
// Render Task
// ============================================================================

/// A rendering task for tracking progress
pub struct RenderTask {
    /// Associated tile renderer
    renderer_handle: Handle,
    /// Current tile index being processed
    current_tile: Option<usize>,
    /// Cancelled flag
    cancelled: bool,
}

impl RenderTask {
    /// Create a new render task
    pub fn new(renderer_handle: Handle) -> Self {
        Self {
            renderer_handle,
            current_tile: None,
            cancelled: false,
        }
    }

    /// Cancel the rendering task
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

// ============================================================================
// FFI Functions
// ============================================================================

/// Create a new tile renderer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_tile_renderer(
    _ctx: Handle,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    tile_width: u32,
    tile_height: u32,
    scale: f32,
    alpha: c_int,
) -> Handle {
    let config = TileConfig {
        tile_width: tile_width.clamp(MIN_TILE_SIZE, MAX_TILE_SIZE),
        tile_height: tile_height.clamp(MIN_TILE_SIZE, MAX_TILE_SIZE),
        num_threads: 0,
        alpha: alpha != 0,
        scale: scale.max(0.1),
        rotation: 0.0,
    };

    let bounds = Rect::new(x0, y0, x1, y1);
    let renderer = TileRenderer::new(bounds, config);
    TILE_RENDERERS.insert(renderer)
}

/// Drop a tile renderer
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_tile_renderer(_ctx: Handle, renderer: Handle) {
    TILE_RENDERERS.remove(renderer);
}

/// Get the number of tiles
#[unsafe(no_mangle)]
pub extern "C" fn fz_tile_renderer_count(_ctx: Handle, renderer: Handle) -> c_int {
    if let Some(arc) = TILE_RENDERERS.get(renderer) {
        if let Ok(r) = arc.lock() {
            return r.tile_count() as c_int;
        }
    }
    0
}

/// Get tile grid dimensions
#[unsafe(no_mangle)]
pub extern "C" fn fz_tile_renderer_grid(
    _ctx: Handle,
    renderer: Handle,
    cols: *mut u32,
    rows: *mut u32,
) {
    if let Some(arc) = TILE_RENDERERS.get(renderer) {
        if let Ok(r) = arc.lock() {
            let (c, row) = r.grid();
            if !cols.is_null() {
                unsafe { *cols = c };
            }
            if !rows.is_null() {
                unsafe { *rows = row };
            }
        }
    }
}

/// Get total pixel dimensions
#[unsafe(no_mangle)]
pub extern "C" fn fz_tile_renderer_dimensions(
    _ctx: Handle,
    renderer: Handle,
    width: *mut u32,
    height: *mut u32,
) {
    if let Some(arc) = TILE_RENDERERS.get(renderer) {
        if let Ok(r) = arc.lock() {
            let (w, h) = r.dimensions();
            if !width.is_null() {
                unsafe { *width = w };
            }
            if !height.is_null() {
                unsafe { *height = h };
            }
        }
    }
}

/// Get rendering progress (0.0 to 1.0)
#[unsafe(no_mangle)]
pub extern "C" fn fz_tile_renderer_progress(_ctx: Handle, renderer: Handle) -> f32 {
    if let Some(arc) = TILE_RENDERERS.get(renderer) {
        if let Ok(r) = arc.lock() {
            return r.progress();
        }
    }
    0.0
}

/// Check if rendering is complete
#[unsafe(no_mangle)]
pub extern "C" fn fz_tile_renderer_is_complete(_ctx: Handle, renderer: Handle) -> c_int {
    if let Some(arc) = TILE_RENDERERS.get(renderer) {
        if let Ok(r) = arc.lock() {
            return if r.is_complete() { 1 } else { 0 };
        }
    }
    0
}

/// Get tile bounds for a specific tile index
#[unsafe(no_mangle)]
pub extern "C" fn fz_tile_renderer_get_bounds(
    _ctx: Handle,
    renderer: Handle,
    index: c_int,
    x0: *mut f32,
    y0: *mut f32,
    x1: *mut f32,
    y1: *mut f32,
) -> c_int {
    if let Some(arc) = TILE_RENDERERS.get(renderer) {
        if let Ok(r) = arc.lock() {
            if let Some(tile) = r.get_tile(index as usize) {
                if !x0.is_null() {
                    unsafe { *x0 = tile.bounds.x0 };
                }
                if !y0.is_null() {
                    unsafe { *y0 = tile.bounds.y0 };
                }
                if !x1.is_null() {
                    unsafe { *x1 = tile.bounds.x1 };
                }
                if !y1.is_null() {
                    unsafe { *y1 = tile.bounds.y1 };
                }
                return 1;
            }
        }
    }
    0
}

/// Create a render task
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_render_task(_ctx: Handle, renderer: Handle) -> Handle {
    let task = RenderTask::new(renderer);
    RENDER_TASKS.insert(task)
}

/// Drop a render task
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_render_task(_ctx: Handle, task: Handle) {
    RENDER_TASKS.remove(task);
}

/// Cancel a render task
#[unsafe(no_mangle)]
pub extern "C" fn fz_cancel_render_task(_ctx: Handle, task: Handle) {
    if let Some(arc) = RENDER_TASKS.get(task) {
        if let Ok(mut t) = arc.lock() {
            t.cancel();
        }
    }
}

/// Check if a render task is cancelled
#[unsafe(no_mangle)]
pub extern "C" fn fz_render_task_is_cancelled(_ctx: Handle, task: Handle) -> c_int {
    if let Some(arc) = RENDER_TASKS.get(task) {
        if let Ok(t) = arc.lock() {
            return if t.is_cancelled() { 1 } else { 0 };
        }
    }
    0
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_config_default() {
        let config = TileConfig::default();
        assert_eq!(config.tile_width, DEFAULT_TILE_SIZE);
        assert_eq!(config.tile_height, DEFAULT_TILE_SIZE);
        assert_eq!(config.num_threads, 0);
        assert!(config.alpha);
        assert_eq!(config.scale, 1.0);
    }

    #[test]
    fn test_tile_renderer_creation() {
        let bounds = Rect::new(0.0, 0.0, 612.0, 792.0); // US Letter
        let config = TileConfig::default();
        let renderer = TileRenderer::new(bounds, config);

        assert!(renderer.tile_count() > 0);
        assert!(!renderer.is_complete());
        assert_eq!(renderer.progress(), 0.0);
    }

    #[test]
    fn test_tile_grid_calculation() {
        let bounds = Rect::new(0.0, 0.0, 512.0, 512.0);
        let config = TileConfig {
            tile_width: 256,
            tile_height: 256,
            ..Default::default()
        };
        let renderer = TileRenderer::new(bounds, config);

        let (cols, rows) = renderer.grid();
        assert_eq!(cols, 2);
        assert_eq!(rows, 2);
        assert_eq!(renderer.tile_count(), 4);
    }

    #[test]
    fn test_tile_completion() {
        let bounds = Rect::new(0.0, 0.0, 256.0, 256.0);
        let config = TileConfig {
            tile_width: 256,
            tile_height: 256,
            ..Default::default()
        };
        let mut renderer = TileRenderer::new(bounds, config);

        assert_eq!(renderer.tile_count(), 1);
        assert!(!renderer.is_complete());

        // Complete the tile
        let pixmap = Pixmap::new(None, 256, 256, true).unwrap();
        renderer.complete_tile(0, pixmap);

        assert!(renderer.is_complete());
        assert_eq!(renderer.progress(), 1.0);
    }

    #[test]
    fn test_pending_tiles() {
        let bounds = Rect::new(0.0, 0.0, 512.0, 512.0);
        let config = TileConfig {
            tile_width: 256,
            tile_height: 256,
            ..Default::default()
        };
        let renderer = TileRenderer::new(bounds, config);

        let pending = renderer.pending_tiles();
        assert_eq!(pending.len(), 4);
        assert!(pending.contains(&0));
        assert!(pending.contains(&1));
        assert!(pending.contains(&2));
        assert!(pending.contains(&3));
    }

    #[test]
    fn test_tile_renderer_ffi() {
        let handle = fz_new_tile_renderer(0, 0.0, 0.0, 612.0, 792.0, 256, 256, 1.0, 1);

        assert!(handle != 0);

        let count = fz_tile_renderer_count(0, handle);
        assert!(count > 0);

        let progress = fz_tile_renderer_progress(0, handle);
        assert_eq!(progress, 0.0);

        let is_complete = fz_tile_renderer_is_complete(0, handle);
        assert_eq!(is_complete, 0);

        fz_drop_tile_renderer(0, handle);
    }

    #[test]
    fn test_render_task() {
        let renderer = fz_new_tile_renderer(0, 0.0, 0.0, 100.0, 100.0, 64, 64, 1.0, 1);
        let task = fz_new_render_task(0, renderer);

        assert!(task != 0);
        assert_eq!(fz_render_task_is_cancelled(0, task), 0);

        fz_cancel_render_task(0, task);
        assert_eq!(fz_render_task_is_cancelled(0, task), 1);

        fz_drop_render_task(0, task);
        fz_drop_tile_renderer(0, renderer);
    }

    #[test]
    fn test_scaled_tiles() {
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        let config = TileConfig {
            tile_width: 100,
            tile_height: 100,
            scale: 2.0, // 2x scale
            ..Default::default()
        };
        let renderer = TileRenderer::new(bounds, config);

        let (width, height) = renderer.dimensions();
        assert_eq!(width, 200);
        assert_eq!(height, 200);

        let (cols, rows) = renderer.grid();
        assert_eq!(cols, 2);
        assert_eq!(rows, 2);
    }
}
