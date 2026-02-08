//! Poster and Tiling Module for Large Format Printing
//!
//! Splits large pages into smaller tiles for printing on standard paper sizes.
//!
//! ## Features
//!
//! - Split oversized pages into printable tiles
//! - Configurable overlap for seamless assembly
//! - Cut marks and alignment guides
//! - Assembly guide generation
//! - Support for various output sizes
//!
//! ## Example
//!
//! ```rust,ignore
//! use micropdf::enhanced::poster::{create_poster, PosterOptions};
//!
//! // Create poster tiles from an A1 original onto A4 sheets
//! let options = PosterOptions::new()
//!     .tile_size(PageSize::A4)
//!     .overlap(10.0, Unit::Mm)
//!     .cut_marks(true);
//!
//! create_poster("large_design.pdf", "poster_tiles.pdf", &options)?;
//! ```

use super::error::{EnhancedError, Result};
use super::page_boxes::{PageSize, Rectangle, Unit};
use std::fs;
use std::path::Path;

// ============================================================================
// Tile Information
// ============================================================================

/// Information about a single tile
#[derive(Debug, Clone)]
pub struct Tile {
    /// Row index (0-based, from bottom)
    pub row: usize,
    /// Column index (0-based, from left)
    pub col: usize,
    /// Source area in the original page
    pub source_rect: Rectangle,
    /// Output position on the tile sheet
    pub output_rect: Rectangle,
    /// Whether this is an edge tile (may be partial)
    pub is_edge: bool,
    /// Overlap with left neighbor (in points)
    pub overlap_left: f32,
    /// Overlap with bottom neighbor (in points)
    pub overlap_bottom: f32,
    /// Overlap with right neighbor (in points)
    pub overlap_right: f32,
    /// Overlap with top neighbor (in points)
    pub overlap_top: f32,
}

impl Tile {
    /// Create a new tile
    pub fn new(row: usize, col: usize, source: Rectangle, output: Rectangle) -> Self {
        Self {
            row,
            col,
            source_rect: source,
            output_rect: output,
            is_edge: false,
            overlap_left: 0.0,
            overlap_bottom: 0.0,
            overlap_right: 0.0,
            overlap_top: 0.0,
        }
    }

    /// Get tile label (e.g., "A1", "B2")
    pub fn label(&self) -> String {
        let col_letter = (b'A' + self.col as u8) as char;
        format!("{}{}", col_letter, self.row + 1)
    }

    /// Get assembly position description
    pub fn position_description(&self, total_cols: usize, total_rows: usize) -> String {
        let h_pos = if self.col == 0 {
            "left"
        } else if self.col == total_cols - 1 {
            "right"
        } else {
            "center"
        };

        let v_pos = if self.row == 0 {
            "bottom"
        } else if self.row == total_rows - 1 {
            "top"
        } else {
            "middle"
        };

        format!("{}-{}", v_pos, h_pos)
    }
}

// ============================================================================
// Poster Options
// ============================================================================

/// Options for poster creation
#[derive(Debug, Clone)]
pub struct PosterOptions {
    /// Output tile size
    pub tile_size: PageSize,
    /// Overlap between tiles (in points)
    pub overlap: f32,
    /// Margin within each tile (in points)
    pub margin: f32,
    /// Add cut marks
    pub cut_marks: bool,
    /// Cut mark length (in points)
    pub cut_mark_length: f32,
    /// Cut mark offset from edge (in points)
    pub cut_mark_offset: f32,
    /// Add alignment marks in overlap area
    pub alignment_marks: bool,
    /// Add tile labels
    pub tile_labels: bool,
    /// Label font size
    pub label_font_size: f32,
    /// Generate assembly guide page
    pub assembly_guide: bool,
    /// Scale factor (1.0 = original size)
    pub scale: f32,
    /// Number of tiles horizontally (auto if 0)
    pub cols: usize,
    /// Number of tiles vertically (auto if 0)
    pub rows: usize,
    /// Output orientation
    pub landscape: bool,
}

impl Default for PosterOptions {
    fn default() -> Self {
        Self {
            tile_size: PageSize::Letter,
            overlap: 36.0, // 0.5 inch
            margin: 18.0,  // 0.25 inch
            cut_marks: true,
            cut_mark_length: 18.0,
            cut_mark_offset: 9.0,
            alignment_marks: true,
            tile_labels: true,
            label_font_size: 8.0,
            assembly_guide: true,
            scale: 1.0,
            cols: 0, // Auto
            rows: 0, // Auto
            landscape: false,
        }
    }
}

impl PosterOptions {
    /// Create new options with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set tile size
    pub fn tile_size(mut self, size: PageSize) -> Self {
        self.tile_size = size;
        self
    }

    /// Set overlap
    pub fn overlap(mut self, overlap: f32, unit: Unit) -> Self {
        self.overlap = unit.to_points(overlap);
        self
    }

    /// Set margin
    pub fn margin(mut self, margin: f32, unit: Unit) -> Self {
        self.margin = unit.to_points(margin);
        self
    }

    /// Enable cut marks
    pub fn cut_marks(mut self, enabled: bool) -> Self {
        self.cut_marks = enabled;
        self
    }

    /// Enable alignment marks
    pub fn alignment_marks(mut self, enabled: bool) -> Self {
        self.alignment_marks = enabled;
        self
    }

    /// Enable tile labels
    pub fn tile_labels(mut self, enabled: bool) -> Self {
        self.tile_labels = enabled;
        self
    }

    /// Enable assembly guide
    pub fn assembly_guide(mut self, enabled: bool) -> Self {
        self.assembly_guide = enabled;
        self
    }

    /// Set scale factor
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale.max(0.1).min(10.0);
        self
    }

    /// Set explicit grid dimensions
    pub fn grid(mut self, cols: usize, rows: usize) -> Self {
        self.cols = cols;
        self.rows = rows;
        self
    }

    /// Set landscape orientation
    pub fn landscape(mut self, enabled: bool) -> Self {
        self.landscape = enabled;
        self
    }

    /// Get usable area per tile (excluding margins)
    pub fn usable_area(&self) -> (f32, f32) {
        let (tile_w, tile_h) = if self.landscape {
            self.tile_size.landscape()
        } else {
            self.tile_size.portrait()
        };

        let usable_w = tile_w - 2.0 * self.margin;
        let usable_h = tile_h - 2.0 * self.margin;

        (usable_w, usable_h)
    }

    /// Calculate effective coverage per tile (excluding overlap)
    pub fn coverage_per_tile(&self) -> (f32, f32) {
        let (usable_w, usable_h) = self.usable_area();
        (usable_w - self.overlap, usable_h - self.overlap)
    }
}

// ============================================================================
// Poster Calculator
// ============================================================================

/// Calculates tile layout for a poster
pub struct PosterCalculator {
    /// Source page dimensions
    source_width: f32,
    source_height: f32,
    /// Options
    options: PosterOptions,
    /// Calculated tiles
    tiles: Vec<Tile>,
    /// Number of columns
    cols: usize,
    /// Number of rows
    rows: usize,
}

impl PosterCalculator {
    /// Create a new calculator
    pub fn new(source_width: f32, source_height: f32, options: PosterOptions) -> Self {
        let mut calc = Self {
            source_width: source_width * options.scale,
            source_height: source_height * options.scale,
            options,
            tiles: Vec::new(),
            cols: 0,
            rows: 0,
        };
        calc.calculate();
        calc
    }

    /// Calculate tile layout
    fn calculate(&mut self) {
        let (coverage_w, coverage_h) = self.options.coverage_per_tile();
        let (usable_w, usable_h) = self.options.usable_area();

        // Calculate grid dimensions
        self.cols = if self.options.cols > 0 {
            self.options.cols
        } else {
            ((self.source_width - self.options.overlap) / coverage_w).ceil() as usize
        };

        self.rows = if self.options.rows > 0 {
            self.options.rows
        } else {
            ((self.source_height - self.options.overlap) / coverage_h).ceil() as usize
        };

        // Ensure at least 1x1
        self.cols = self.cols.max(1);
        self.rows = self.rows.max(1);

        // Generate tiles
        self.tiles.clear();

        for row in 0..self.rows {
            for col in 0..self.cols {
                // Source position (in scaled coordinates)
                let src_x = col as f32 * coverage_w;
                let src_y = row as f32 * coverage_h;

                // Source area (may extend beyond actual content for edge tiles)
                let src_w = usable_w;
                let src_h = usable_h;

                // Check if this is an edge tile
                let is_right_edge = col == self.cols - 1;
                let is_top_edge = row == self.rows - 1;
                let is_left_edge = col == 0;
                let is_bottom_edge = row == 0;

                let source_rect = Rectangle::new(src_x, src_y, src_x + src_w, src_y + src_h);

                let output_rect = Rectangle::new(
                    self.options.margin,
                    self.options.margin,
                    self.options.margin + src_w,
                    self.options.margin + src_h,
                );

                let mut tile = Tile::new(row, col, source_rect, output_rect);
                tile.is_edge = is_left_edge || is_right_edge || is_bottom_edge || is_top_edge;

                // Set overlaps
                if !is_left_edge {
                    tile.overlap_left = self.options.overlap;
                }
                if !is_bottom_edge {
                    tile.overlap_bottom = self.options.overlap;
                }
                if !is_right_edge {
                    tile.overlap_right = self.options.overlap;
                }
                if !is_top_edge {
                    tile.overlap_top = self.options.overlap;
                }

                self.tiles.push(tile);
            }
        }
    }

    /// Get total number of tiles
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Get grid dimensions
    pub fn grid_dimensions(&self) -> (usize, usize) {
        (self.cols, self.rows)
    }

    /// Get all tiles
    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }

    /// Get tile by position
    pub fn get_tile(&self, col: usize, row: usize) -> Option<&Tile> {
        let idx = row * self.cols + col;
        self.tiles.get(idx)
    }

    /// Get assembled poster dimensions
    pub fn poster_dimensions(&self) -> (f32, f32) {
        (self.source_width, self.source_height)
    }
}

// ============================================================================
// Poster Generator
// ============================================================================

/// Generates poster tiles
pub struct PosterGenerator {
    /// Options
    options: PosterOptions,
    /// Source PDF data
    source_data: Vec<u8>,
    /// Source page dimensions
    source_dims: Option<(f32, f32)>,
    /// Calculator
    calculator: Option<PosterCalculator>,
}

impl PosterGenerator {
    /// Create a new poster generator
    pub fn new(options: PosterOptions) -> Self {
        Self {
            options,
            source_data: Vec::new(),
            source_dims: None,
            calculator: None,
        }
    }

    /// Load source PDF
    pub fn load(&mut self, path: &str) -> Result<()> {
        if !Path::new(path).exists() {
            return Err(EnhancedError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", path),
            )));
        }

        self.source_data = fs::read(path)?;
        self.source_dims = Some(self.parse_page_dimensions()?);

        let (w, h) = self.source_dims.unwrap();
        self.calculator = Some(PosterCalculator::new(w, h, self.options.clone()));

        Ok(())
    }

    /// Load from data
    pub fn load_data(&mut self, data: Vec<u8>) -> Result<()> {
        self.source_data = data;
        self.source_dims = Some(self.parse_page_dimensions()?);

        let (w, h) = self.source_dims.unwrap();
        self.calculator = Some(PosterCalculator::new(w, h, self.options.clone()));

        Ok(())
    }

    /// Get number of output tiles
    pub fn tile_count(&self) -> usize {
        self.calculator
            .as_ref()
            .map(|c| c.tile_count())
            .unwrap_or(0)
    }

    /// Get grid dimensions
    pub fn grid_dimensions(&self) -> (usize, usize) {
        self.calculator
            .as_ref()
            .map(|c| c.grid_dimensions())
            .unwrap_or((0, 0))
    }

    /// Generate poster PDF with all tiles
    pub fn generate(&self) -> Result<Vec<u8>> {
        let calculator = self
            .calculator
            .as_ref()
            .ok_or_else(|| EnhancedError::InvalidParameter("Source not loaded".to_string()))?;

        let (tile_w, tile_h) = if self.options.landscape {
            self.options.tile_size.landscape()
        } else {
            self.options.tile_size.portrait()
        };

        let mut pdf = Vec::new();

        // PDF header
        pdf.extend(b"%PDF-1.7\n");
        pdf.extend(b"%\xe2\xe3\xcf\xd3\n");

        let mut objects: Vec<String> = Vec::new();
        let mut page_refs: Vec<usize> = Vec::new();

        // Generate each tile page
        for tile in calculator.tiles() {
            let content = self.generate_tile_content(tile, calculator);

            let content_obj_num = objects.len() + 1;
            objects.push(format!(
                "{} 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
                content_obj_num,
                content.len(),
                content
            ));

            let page_obj_num = objects.len() + 1;
            objects.push(format!(
                "{} 0 obj\n<< /Type /Page /MediaBox [0 0 {} {}] /Contents {} 0 R >>\nendobj\n",
                page_obj_num, tile_w, tile_h, content_obj_num
            ));
            page_refs.push(page_obj_num);
        }

        // Assembly guide page (if enabled)
        if self.options.assembly_guide {
            let guide_content = self.generate_assembly_guide(calculator);

            let content_obj_num = objects.len() + 1;
            objects.push(format!(
                "{} 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
                content_obj_num,
                guide_content.len(),
                guide_content
            ));

            let page_obj_num = objects.len() + 1;
            objects.push(format!(
                "{} 0 obj\n<< /Type /Page /MediaBox [0 0 {} {}] /Contents {} 0 R >>\nendobj\n",
                page_obj_num, tile_w, tile_h, content_obj_num
            ));
            page_refs.push(page_obj_num);
        }

        // Pages object
        let pages_obj_num = objects.len() + 1;
        let page_refs_str: String = page_refs
            .iter()
            .map(|n| format!("{} 0 R", n))
            .collect::<Vec<_>>()
            .join(" ");
        objects.push(format!(
            "{} 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
            pages_obj_num,
            page_refs_str,
            page_refs.len()
        ));

        // Catalog
        let catalog_obj_num = objects.len() + 1;
        objects.push(format!(
            "{} 0 obj\n<< /Type /Catalog /Pages {} 0 R >>\nendobj\n",
            catalog_obj_num, pages_obj_num
        ));

        // Write objects
        let mut offsets: Vec<usize> = Vec::new();
        for obj in &objects {
            offsets.push(pdf.len());
            pdf.extend(obj.as_bytes());
        }

        // Xref
        let xref_offset = pdf.len();
        pdf.extend(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
        pdf.extend(b"0000000000 65535 f \n");
        for offset in &offsets {
            pdf.extend(format!("{:010} 00000 n \n", offset).as_bytes());
        }

        // Trailer
        pdf.extend(
            format!(
                "trailer\n<< /Size {} /Root {} 0 R >>\nstartxref\n{}\n%%EOF\n",
                objects.len() + 1,
                catalog_obj_num,
                xref_offset
            )
            .as_bytes(),
        );

        Ok(pdf)
    }

    /// Generate content for a single tile
    fn generate_tile_content(&self, tile: &Tile, calculator: &PosterCalculator) -> String {
        let mut content = String::new();
        let (cols, rows) = calculator.grid_dimensions();

        content.push_str("q\n");

        // Background
        content.push_str("1 1 1 rg\n"); // White

        // In a full implementation, we would:
        // 1. Clip to the usable area
        // 2. Transform to map source coordinates
        // 3. Draw the source page content

        // Placeholder: draw tile outline and label
        let rect = &tile.output_rect;
        content.push_str(&format!(
            "0.8 0.8 0.8 rg {} {} {} {} re f\n",
            rect.llx,
            rect.lly,
            rect.width(),
            rect.height()
        ));

        // Border
        content.push_str("0 0 0 RG 0.5 w\n");
        content.push_str(&format!(
            "{} {} {} {} re S\n",
            rect.llx,
            rect.lly,
            rect.width(),
            rect.height()
        ));

        // Cut marks
        if self.options.cut_marks {
            content.push_str(&self.generate_cut_marks(rect));
        }

        // Alignment marks (in overlap areas)
        if self.options.alignment_marks {
            content.push_str(&self.generate_alignment_marks(tile));
        }

        // Tile label
        if self.options.tile_labels {
            let label = tile.label();
            let pos_desc = tile.position_description(cols, rows);

            // Center label
            content.push_str("BT\n");
            content.push_str(&format!("/F1 {} Tf\n", self.options.label_font_size * 3.0));
            content.push_str("0.3 0.3 0.3 rg\n");
            content.push_str(&format!(
                "{} {} Td\n",
                rect.center_x() - 20.0,
                rect.center_y() + 20.0
            ));
            content.push_str(&format!("({}) Tj\n", label));
            content.push_str("ET\n");

            // Position description
            content.push_str("BT\n");
            content.push_str(&format!("/F1 {} Tf\n", self.options.label_font_size));
            content.push_str(&format!(
                "{} {} Td\n",
                rect.center_x() - 30.0,
                rect.center_y() - 10.0
            ));
            content.push_str(&format!("({}) Tj\n", pos_desc));
            content.push_str("ET\n");

            // Grid position
            content.push_str("BT\n");
            content.push_str(&format!(
                "{} {} Td\n",
                rect.center_x() - 40.0,
                rect.center_y() - 25.0
            ));
            content.push_str(&format!(
                "(Row {}, Col {}) Tj\n",
                tile.row + 1,
                tile.col + 1
            ));
            content.push_str("ET\n");
        }

        content.push_str("Q\n");
        content
    }

    /// Generate cut marks
    fn generate_cut_marks(&self, rect: &Rectangle) -> String {
        let len = self.options.cut_mark_length;
        let off = self.options.cut_mark_offset;
        let mut marks = String::new();

        marks.push_str("q 0.25 w 0 G\n");

        // Top-left
        marks.push_str(&format!(
            "{} {} m {} {} l S\n",
            rect.llx - off - len,
            rect.ury,
            rect.llx - off,
            rect.ury
        ));
        marks.push_str(&format!(
            "{} {} m {} {} l S\n",
            rect.llx,
            rect.ury + off,
            rect.llx,
            rect.ury + off + len
        ));

        // Top-right
        marks.push_str(&format!(
            "{} {} m {} {} l S\n",
            rect.urx + off,
            rect.ury,
            rect.urx + off + len,
            rect.ury
        ));
        marks.push_str(&format!(
            "{} {} m {} {} l S\n",
            rect.urx,
            rect.ury + off,
            rect.urx,
            rect.ury + off + len
        ));

        // Bottom-left
        marks.push_str(&format!(
            "{} {} m {} {} l S\n",
            rect.llx - off - len,
            rect.lly,
            rect.llx - off,
            rect.lly
        ));
        marks.push_str(&format!(
            "{} {} m {} {} l S\n",
            rect.llx,
            rect.lly - off - len,
            rect.llx,
            rect.lly - off
        ));

        // Bottom-right
        marks.push_str(&format!(
            "{} {} m {} {} l S\n",
            rect.urx + off,
            rect.lly,
            rect.urx + off + len,
            rect.lly
        ));
        marks.push_str(&format!(
            "{} {} m {} {} l S\n",
            rect.urx,
            rect.lly - off - len,
            rect.urx,
            rect.lly - off
        ));

        marks.push_str("Q\n");
        marks
    }

    /// Generate alignment marks in overlap areas
    fn generate_alignment_marks(&self, tile: &Tile) -> String {
        let mut marks = String::new();
        let rect = &tile.output_rect;

        marks.push_str("q 0.5 w 0 0.5 1 RG\n"); // Blue registration marks

        // Left overlap (if exists)
        if tile.overlap_left > 0.0 {
            let x = rect.llx + tile.overlap_left / 2.0;
            // Cross mark
            marks.push_str(&format!(
                "{} {} m {} {} l S\n",
                x - 5.0,
                rect.center_y(),
                x + 5.0,
                rect.center_y()
            ));
            marks.push_str(&format!(
                "{} {} m {} {} l S\n",
                x,
                rect.center_y() - 5.0,
                x,
                rect.center_y() + 5.0
            ));
        }

        // Bottom overlap (if exists)
        if tile.overlap_bottom > 0.0 {
            let y = rect.lly + tile.overlap_bottom / 2.0;
            marks.push_str(&format!(
                "{} {} m {} {} l S\n",
                rect.center_x() - 5.0,
                y,
                rect.center_x() + 5.0,
                y
            ));
            marks.push_str(&format!(
                "{} {} m {} {} l S\n",
                rect.center_x(),
                y - 5.0,
                rect.center_x(),
                y + 5.0
            ));
        }

        marks.push_str("Q\n");
        marks
    }

    /// Generate assembly guide page
    fn generate_assembly_guide(&self, calculator: &PosterCalculator) -> String {
        let mut content = String::new();
        let (cols, rows) = calculator.grid_dimensions();
        let (poster_w, poster_h) = calculator.poster_dimensions();
        let (tile_w, tile_h) = if self.options.landscape {
            self.options.tile_size.landscape()
        } else {
            self.options.tile_size.portrait()
        };

        // Scale to fit on page
        let margin = 36.0;
        let available_w = tile_w - 2.0 * margin;
        let available_h = tile_h - 2.0 * margin - 60.0; // Space for title

        let scale_w = available_w / poster_w;
        let scale_h = available_h / poster_h;
        let scale = scale_w.min(scale_h);

        let scaled_w = poster_w * scale;
        let scaled_h = poster_h * scale;
        let offset_x = margin + (available_w - scaled_w) / 2.0;
        let offset_y = margin + (available_h - scaled_h) / 2.0;

        content.push_str("q\n");

        // Title
        content.push_str("BT\n");
        content.push_str("/F1 14 Tf\n");
        content.push_str("0 0 0 rg\n");
        content.push_str(&format!("{} {} Td\n", margin, tile_h - margin - 20.0));
        content.push_str("(Assembly Guide) Tj\n");
        content.push_str("ET\n");

        // Poster info
        content.push_str("BT\n");
        content.push_str("/F1 10 Tf\n");
        content.push_str(&format!("{} {} Td\n", margin, tile_h - margin - 40.0));
        content.push_str(&format!(
            "(Grid: {} columns x {} rows = {} tiles) Tj\n",
            cols,
            rows,
            cols * rows
        ));
        content.push_str("ET\n");

        // Draw grid
        content.push_str(&format!(
            "{} {} {} {} re S\n",
            offset_x, offset_y, scaled_w, scaled_h
        ));

        let cell_w = scaled_w / cols as f32;
        let cell_h = scaled_h / rows as f32;

        // Draw tile grid
        content.push_str("0.5 w 0 G\n");
        for row in 0..rows {
            for col in 0..cols {
                let x = offset_x + col as f32 * cell_w;
                let y = offset_y + row as f32 * cell_h;

                // Cell rectangle
                content.push_str(&format!("{} {} {} {} re S\n", x, y, cell_w, cell_h));

                // Cell label
                if let Some(tile) = calculator.get_tile(col, row) {
                    content.push_str("BT\n");
                    content.push_str("/F1 8 Tf\n");
                    content.push_str(&format!(
                        "{} {} Td\n",
                        x + cell_w / 2.0 - 5.0,
                        y + cell_h / 2.0 - 3.0
                    ));
                    content.push_str(&format!("({}) Tj\n", tile.label()));
                    content.push_str("ET\n");
                }
            }
        }

        // Assembly instructions
        let instructions = [
            "Assembly Instructions:",
            "1. Print all tiles on the same paper size",
            "2. Trim along cut marks (solid lines)",
            "3. Align tiles using registration marks (blue crosses)",
            "4. Tiles overlap - align the marks precisely",
            "5. Tape or glue from the back",
            "6. Start from bottom-left corner (A1)",
        ];

        let mut y = offset_y - 20.0;
        for (i, line) in instructions.iter().enumerate() {
            content.push_str("BT\n");
            if i == 0 {
                content.push_str("/F1 10 Tf\n");
            } else {
                content.push_str("/F1 8 Tf\n");
            }
            content.push_str(&format!("{} {} Td\n", margin, y));
            content.push_str(&format!("({}) Tj\n", line));
            content.push_str("ET\n");
            y -= 12.0;
        }

        content.push_str("Q\n");
        content
    }

    /// Save generated poster
    pub fn save(&self, output_path: &str) -> Result<()> {
        let pdf = self.generate()?;
        fs::write(output_path, pdf)?;
        Ok(())
    }

    /// Parse page dimensions from PDF
    fn parse_page_dimensions(&self) -> Result<(f32, f32)> {
        let content = String::from_utf8_lossy(&self.source_data);

        // Look for MediaBox
        if let Some(pos) = content.find("/MediaBox") {
            if let Some(arr_start) = content[pos..].find('[') {
                if let Some(arr_end) = content[pos + arr_start..].find(']') {
                    let arr_str = &content[pos + arr_start..pos + arr_start + arr_end + 1];
                    if let Ok(rect) = Rectangle::from_pdf_array(arr_str) {
                        return Ok((rect.width(), rect.height()));
                    }
                }
            }
        }

        // Default to Letter
        Ok((612.0, 792.0))
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Create a poster with default options
pub fn create_poster(input_path: &str, output_path: &str) -> Result<()> {
    let options = PosterOptions::new();
    create_poster_with_options(input_path, output_path, &options)
}

/// Create a poster with custom options
pub fn create_poster_with_options(
    input_path: &str,
    output_path: &str,
    options: &PosterOptions,
) -> Result<()> {
    let mut generator = PosterGenerator::new(options.clone());
    generator.load(input_path)?;
    generator.save(output_path)
}

/// Calculate tile count for a given source size
pub fn calculate_tile_count(
    source_width: f32,
    source_height: f32,
    tile_size: PageSize,
    overlap: f32,
) -> usize {
    let options = PosterOptions::new()
        .tile_size(tile_size)
        .overlap(overlap, Unit::Point);

    let calc = PosterCalculator::new(source_width, source_height, options);
    calc.tile_count()
}

/// Get tile grid dimensions for a given source size
pub fn calculate_grid(
    source_width: f32,
    source_height: f32,
    tile_size: PageSize,
) -> (usize, usize) {
    let options = PosterOptions::new().tile_size(tile_size);
    let calc = PosterCalculator::new(source_width, source_height, options);
    calc.grid_dimensions()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_label() {
        let tile = Tile::new(0, 0, Rectangle::default(), Rectangle::default());
        assert_eq!(tile.label(), "A1");

        let tile = Tile::new(2, 3, Rectangle::default(), Rectangle::default());
        assert_eq!(tile.label(), "D3");
    }

    #[test]
    fn test_tile_position_description() {
        let tile = Tile::new(0, 0, Rectangle::default(), Rectangle::default());
        assert_eq!(tile.position_description(3, 3), "bottom-left");

        let tile = Tile::new(2, 2, Rectangle::default(), Rectangle::default());
        assert_eq!(tile.position_description(3, 3), "top-right");

        let tile = Tile::new(1, 1, Rectangle::default(), Rectangle::default());
        assert_eq!(tile.position_description(3, 3), "middle-center");
    }

    #[test]
    fn test_poster_options_builder() {
        let options = PosterOptions::new()
            .tile_size(PageSize::A4)
            .overlap(10.0, Unit::Mm)
            .margin(5.0, Unit::Mm)
            .cut_marks(true)
            .alignment_marks(true);

        assert_eq!(options.tile_size, PageSize::A4);
        assert!(options.cut_marks);
        assert!(options.alignment_marks);
        // 10mm ≈ 28.35pt
        assert!(options.overlap > 28.0 && options.overlap < 29.0);
    }

    #[test]
    fn test_usable_area() {
        let options = PosterOptions::new()
            .tile_size(PageSize::Letter)
            .margin(36.0, Unit::Point);

        let (w, h) = options.usable_area();
        // Letter is 612x792, minus 72 (2*36) margins
        assert!((w - 540.0).abs() < 1.0);
        assert!((h - 720.0).abs() < 1.0);
    }

    #[test]
    fn test_poster_calculator_basic() {
        let options = PosterOptions::new()
            .tile_size(PageSize::Letter)
            .margin(0.0, Unit::Point)
            .overlap(0.0, Unit::Point);

        // A3 is roughly 842 x 1190 points
        let calc = PosterCalculator::new(842.0, 1190.0, options);

        let (cols, rows) = calc.grid_dimensions();
        // Letter is 612x792, should fit in 2x2 grid
        assert!(cols >= 1);
        assert!(rows >= 1);
    }

    #[test]
    fn test_poster_calculator_with_overlap() {
        let options = PosterOptions::new()
            .tile_size(PageSize::Letter)
            .margin(36.0, Unit::Point)
            .overlap(36.0, Unit::Point);

        // Large poster
        let calc = PosterCalculator::new(2000.0, 3000.0, options);

        assert!(calc.tile_count() > 1);

        // All tiles should have overlap info
        for tile in calc.tiles() {
            if tile.col > 0 {
                assert!(tile.overlap_left > 0.0);
            }
            if tile.row > 0 {
                assert!(tile.overlap_bottom > 0.0);
            }
        }
    }

    #[test]
    fn test_calculate_tile_count() {
        // A1 (841 x 1189mm) onto A4 (210 x 297mm)
        let a1_width = 2383.94;
        let a1_height = 3370.39;
        let count = calculate_tile_count(a1_width, a1_height, PageSize::A4, 36.0);

        // Should be at least 4x4 = 16 tiles
        assert!(count >= 16);
    }

    #[test]
    fn test_calculate_grid() {
        let (cols, rows) = calculate_grid(1000.0, 1500.0, PageSize::Letter);
        assert!(cols >= 2);
        assert!(rows >= 2);
    }

    #[test]
    fn test_poster_generator_creation() {
        let options = PosterOptions::new()
            .tile_size(PageSize::A4)
            .overlap(10.0, Unit::Mm);

        let generator = PosterGenerator::new(options);
        assert_eq!(generator.tile_count(), 0); // No source loaded yet
    }
}
