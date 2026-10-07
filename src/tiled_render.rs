use std::collections::HashMap;

/// Standard dimension for square rendering tiles (512x512 pixels).
pub const TILE_DIMENSION: u32 = 512;

/// Coordinate identifying a discrete rendering tile on a page at a given scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoordinate {
    pub page: usize,
    pub col: u32,
    pub row: u32,
    pub zoom_level_quantized: u32, // Quantized to integer percentage (e.g. 400 for 4.0x)
}

/// Bounding rectangle in page raster pixel space for a tile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TilePixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Geometry metadata for a page at deep zoom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TiledPageGeometry {
    pub total_width_px: u32,
    pub total_height_px: u32,
    pub cols: u32,
    pub rows: u32,
    pub tile_size: u32,
}

impl TiledPageGeometry {
    /// Computes tiled grid partition for a given page dimension and zoom scale.
    pub fn compute(page_width_pt: f32, page_height_pt: f32, scale: f32) -> Self {
        let total_w = ((page_width_pt * scale).ceil() as u32).max(1);
        let total_h = ((page_height_pt * scale).ceil() as u32).max(1);

        let cols = (total_w + TILE_DIMENSION - 1) / TILE_DIMENSION;
        let rows = (total_h + TILE_DIMENSION - 1) / TILE_DIMENSION;

        Self {
            total_width_px: total_w,
            total_height_px: total_h,
            cols,
            rows,
            tile_size: TILE_DIMENSION,
        }
    }

    /// Computes the pixel rectangle for a given tile column and row.
    pub fn tile_rect(&self, col: u32, row: u32) -> TilePixelRect {
        let x = col * self.tile_size;
        let y = row * self.tile_size;
        let w = (self.total_width_px.saturating_sub(x)).min(self.tile_size);
        let h = (self.total_height_px.saturating_sub(y)).min(self.tile_size);

        TilePixelRect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    /// Determines which tiles are visible given a visible viewport rectangle in pixel space.
    pub fn intersecting_tiles(
        &self,
        viewport_x: f32,
        viewport_y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Vec<(u32, u32)> {
        let min_x = viewport_x.max(0.0) as u32;
        let min_y = viewport_y.max(0.0) as u32;
        let max_x = ((viewport_x + viewport_w).ceil() as u32).min(self.total_width_px);
        let max_y = ((viewport_y + viewport_h).ceil() as u32).min(self.total_height_px);

        let start_col = min_x / self.tile_size;
        let end_col = (max_x + self.tile_size - 1) / self.tile_size;
        let start_row = min_y / self.tile_size;
        let end_row = (max_y + self.tile_size - 1) / self.tile_size;

        let mut tiles = Vec::new();
        for r in start_row..end_row.min(self.rows) {
            for c in start_col..end_col.min(self.cols) {
                tiles.push((c, r));
            }
        }
        tiles
    }
}

/// Rendered raster data for a single tile.
#[derive(Debug, Clone)]
pub struct RenderedTile {
    pub coordinate: TileCoordinate,
    pub rect: TilePixelRect,
    /// RGBA8 raster pixel buffer (width * height * 4 bytes).
    pub rgba_data: std::sync::Arc<[u8]>,
}

/// High-performance cache for deep-zoom viewport tiles.
#[derive(Debug, Default)]
pub struct TileCache {
    tiles: HashMap<TileCoordinate, RenderedTile>,
    max_cached_tiles: usize,
}

impl TileCache {
    /// Creates a tile cache with a given tile capacity limit (e.g. 128 tiles = ~128MB RAM at 512x512 RGBA).
    pub fn new(max_cached_tiles: usize) -> Self {
        Self {
            tiles: HashMap::new(),
            max_cached_tiles: max_cached_tiles.max(16),
        }
    }

    pub fn get(&self, coord: &TileCoordinate) -> Option<&RenderedTile> {
        self.tiles.get(coord)
    }

    pub fn put(&mut self, tile: RenderedTile) {
        if self.tiles.len() >= self.max_cached_tiles {
            // Simple eviction of oldest entries
            if let Some(first_key) = self.tiles.keys().next().cloned() {
                self.tiles.remove(&first_key);
            }
        }
        self.tiles.insert(tile.coordinate, tile);
    }

    pub fn clear(&mut self) {
        self.tiles.clear();
    }
}
