//! N-Up and Grid Layout Module for Print Production
//!
//! Creates multi-page layouts for printing multiple pages on a single sheet.
//!
//! ## Supported Layouts
//!
//! - **2-up**: 2 pages per sheet (1x2 or 2x1)
//! - **4-up**: 4 pages per sheet (2x2)
//! - **6-up**: 6 pages per sheet (2x3 or 3x2)
//! - **9-up**: 9 pages per sheet (3x3)
//! - **Custom**: Any MxN grid
//!
//! ## Page Ordering
//!
//! - **LTR** (Left-to-Right): →
//! - **RTL** (Right-to-Left): ←
//! - **TTB** (Top-to-Bottom): ↓
//! - **BTT** (Bottom-to-Top): ↑
//!
//! ## Example
//!
//! ```rust,ignore
//! use micropdf::enhanced::nup::{NupLayout, PageOrder, create_nup};
//!
//! // Create 4-up layout on A4
//! create_nup("input.pdf", "output.pdf", NupLayout::FourUp, PageSize::A4)?;
//!
//! // Create custom 3x2 grid with borders
//! let options = NupOptions::new()
//!     .grid(3, 2)
//!     .page_size(PageSize::A3)
//!     .border(true)
//!     .spacing(5.0, Unit::Mm);
//! create_nup_with_options("input.pdf", "output.pdf", &options)?;
//! ```

use super::error::{EnhancedError, Result};
use super::page_boxes::{PageSize, Rectangle, Unit};
use std::fs;
use std::path::Path;

// ============================================================================
// Page Ordering
// ============================================================================

/// Page ordering direction for N-up layouts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageOrder {
    /// Left to Right, Top to Bottom (default Western reading order)
    #[default]
    LeftToRightTopToBottom,
    /// Right to Left, Top to Bottom (RTL languages)
    RightToLeftTopToBottom,
    /// Top to Bottom, Left to Right (columns first)
    TopToBottomLeftToRight,
    /// Top to Bottom, Right to Left (RTL columns)
    TopToBottomRightToLeft,
    /// Bottom to Top, Left to Right
    BottomToTopLeftToRight,
    /// Bottom to Top, Right to Left
    BottomToTopRightToLeft,
    /// Left to Right, Bottom to Top
    LeftToRightBottomToTop,
    /// Right to Left, Bottom to Top
    RightToLeftBottomToTop,
}

impl PageOrder {
    /// Get cell indices for a grid position
    pub fn get_indices(&self, cols: usize, rows: usize) -> Vec<(usize, usize)> {
        let mut indices = Vec::with_capacity(cols * rows);

        match self {
            PageOrder::LeftToRightTopToBottom => {
                for row in 0..rows {
                    for col in 0..cols {
                        indices.push((row, col));
                    }
                }
            }
            PageOrder::RightToLeftTopToBottom => {
                for row in 0..rows {
                    for col in (0..cols).rev() {
                        indices.push((row, col));
                    }
                }
            }
            PageOrder::TopToBottomLeftToRight => {
                for col in 0..cols {
                    for row in 0..rows {
                        indices.push((row, col));
                    }
                }
            }
            PageOrder::TopToBottomRightToLeft => {
                for col in (0..cols).rev() {
                    for row in 0..rows {
                        indices.push((row, col));
                    }
                }
            }
            PageOrder::BottomToTopLeftToRight => {
                for col in 0..cols {
                    for row in (0..rows).rev() {
                        indices.push((row, col));
                    }
                }
            }
            PageOrder::BottomToTopRightToLeft => {
                for col in (0..cols).rev() {
                    for row in (0..rows).rev() {
                        indices.push((row, col));
                    }
                }
            }
            PageOrder::LeftToRightBottomToTop => {
                for row in (0..rows).rev() {
                    for col in 0..cols {
                        indices.push((row, col));
                    }
                }
            }
            PageOrder::RightToLeftBottomToTop => {
                for row in (0..rows).rev() {
                    for col in (0..cols).rev() {
                        indices.push((row, col));
                    }
                }
            }
        }

        indices
    }
}

// ============================================================================
// N-up Layout Presets
// ============================================================================

/// Predefined N-up layouts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NupLayout {
    /// 2 pages per sheet (1x2 portrait or 2x1 landscape)
    TwoUp,
    /// 2 pages per sheet, horizontal (2x1)
    TwoUpHorizontal,
    /// 4 pages per sheet (2x2)
    FourUp,
    /// 6 pages per sheet (2x3 or 3x2)
    SixUp,
    /// 6 pages per sheet, horizontal (3x2)
    SixUpHorizontal,
    /// 8 pages per sheet (2x4 or 4x2)
    EightUp,
    /// 9 pages per sheet (3x3)
    NineUp,
    /// 12 pages per sheet (3x4 or 4x3)
    TwelveUp,
    /// 16 pages per sheet (4x4)
    SixteenUp,
    /// Custom MxN grid
    Custom(usize, usize),
}

impl NupLayout {
    /// Get grid dimensions (columns, rows)
    pub fn grid(&self) -> (usize, usize) {
        match self {
            NupLayout::TwoUp => (1, 2),
            NupLayout::TwoUpHorizontal => (2, 1),
            NupLayout::FourUp => (2, 2),
            NupLayout::SixUp => (2, 3),
            NupLayout::SixUpHorizontal => (3, 2),
            NupLayout::EightUp => (2, 4),
            NupLayout::NineUp => (3, 3),
            NupLayout::TwelveUp => (3, 4),
            NupLayout::SixteenUp => (4, 4),
            NupLayout::Custom(cols, rows) => (*cols, *rows),
        }
    }

    /// Total pages per sheet
    pub fn pages_per_sheet(&self) -> usize {
        let (cols, rows) = self.grid();
        cols * rows
    }

    /// Create from grid dimensions
    pub fn from_grid(cols: usize, rows: usize) -> Result<Self> {
        if cols == 0 || rows == 0 {
            return Err(EnhancedError::InvalidParameter(
                "Grid dimensions must be > 0".to_string(),
            ));
        }

        Ok(match (cols, rows) {
            (1, 2) => NupLayout::TwoUp,
            (2, 1) => NupLayout::TwoUpHorizontal,
            (2, 2) => NupLayout::FourUp,
            (2, 3) => NupLayout::SixUp,
            (3, 2) => NupLayout::SixUpHorizontal,
            (2, 4) => NupLayout::EightUp,
            (3, 3) => NupLayout::NineUp,
            (3, 4) => NupLayout::TwelveUp,
            (4, 4) => NupLayout::SixteenUp,
            (c, r) => NupLayout::Custom(c, r),
        })
    }
}

// ============================================================================
// N-up Options
// ============================================================================

/// Options for N-up layout creation
#[derive(Debug, Clone)]
pub struct NupOptions {
    /// Layout type
    pub layout: NupLayout,
    /// Output page size
    pub page_size: PageSize,
    /// Page ordering
    pub order: PageOrder,
    /// Draw border around each cell
    pub border: bool,
    /// Border line width in points
    pub border_width: f32,
    /// Spacing between cells (horizontal, vertical) in points
    pub spacing: (f32, f32),
    /// Margin around the grid in points
    pub margin: f32,
    /// Scale pages to fit cells
    pub scale_to_fit: bool,
    /// Center pages in cells
    pub center: bool,
    /// Rotate pages if it improves fit
    pub auto_rotate: bool,
    /// Background color (None for transparent)
    pub background: Option<(f32, f32, f32)>,
    /// Output orientation
    pub landscape: bool,
}

impl Default for NupOptions {
    fn default() -> Self {
        Self {
            layout: NupLayout::FourUp,
            page_size: PageSize::Letter,
            order: PageOrder::default(),
            border: false,
            border_width: 0.5,
            spacing: (0.0, 0.0),
            margin: 0.0,
            scale_to_fit: true,
            center: true,
            auto_rotate: false,
            background: None,
            landscape: false,
        }
    }
}

impl NupOptions {
    /// Create new options with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set layout
    pub fn layout(mut self, layout: NupLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Set grid dimensions
    pub fn grid(mut self, cols: usize, rows: usize) -> Self {
        self.layout = NupLayout::Custom(cols, rows);
        self
    }

    /// Set page size
    pub fn page_size(mut self, size: PageSize) -> Self {
        self.page_size = size;
        self
    }

    /// Set page order
    pub fn order(mut self, order: PageOrder) -> Self {
        self.order = order;
        self
    }

    /// Enable borders
    pub fn border(mut self, enabled: bool) -> Self {
        self.border = enabled;
        self
    }

    /// Set border width
    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = width;
        self
    }

    /// Set spacing between cells
    pub fn spacing(mut self, h: f32, v: f32, unit: Unit) -> Self {
        self.spacing = (unit.to_points(h), unit.to_points(v));
        self
    }

    /// Set margin around grid
    pub fn margin(mut self, margin: f32, unit: Unit) -> Self {
        self.margin = unit.to_points(margin);
        self
    }

    /// Set scale to fit
    pub fn scale_to_fit(mut self, enabled: bool) -> Self {
        self.scale_to_fit = enabled;
        self
    }

    /// Set centering
    pub fn center(mut self, enabled: bool) -> Self {
        self.center = enabled;
        self
    }

    /// Enable auto rotation
    pub fn auto_rotate(mut self, enabled: bool) -> Self {
        self.auto_rotate = enabled;
        self
    }

    /// Set landscape orientation
    pub fn landscape(mut self, enabled: bool) -> Self {
        self.landscape = enabled;
        self
    }

    /// Set background color
    pub fn background_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.background = Some((r, g, b));
        self
    }

    /// Calculate cell dimensions
    pub fn cell_dimensions(&self) -> (f32, f32) {
        let (cols, rows) = self.layout.grid();
        let (page_w, page_h) = if self.landscape {
            self.page_size.landscape()
        } else {
            self.page_size.portrait()
        };

        let available_width = page_w - 2.0 * self.margin - (cols - 1) as f32 * self.spacing.0;
        let available_height = page_h - 2.0 * self.margin - (rows - 1) as f32 * self.spacing.1;

        let cell_width = available_width / cols as f32;
        let cell_height = available_height / rows as f32;

        (cell_width, cell_height)
    }

    /// Get cell position for a grid index
    pub fn cell_position(&self, col: usize, row: usize) -> (f32, f32) {
        let (cell_w, cell_h) = self.cell_dimensions();
        let (_, rows) = self.layout.grid();

        let x = self.margin + col as f32 * (cell_w + self.spacing.0);
        let y = self.margin + (rows - 1 - row) as f32 * (cell_h + self.spacing.1);

        (x, y)
    }
}

// ============================================================================
// Grid Cell
// ============================================================================

/// A cell in the N-up grid
#[derive(Debug, Clone)]
pub struct GridCell {
    /// Column index (0-based)
    pub col: usize,
    /// Row index (0-based)
    pub row: usize,
    /// Position (x, y) in points from bottom-left
    pub position: (f32, f32),
    /// Size (width, height) in points
    pub size: (f32, f32),
    /// Source page index (if assigned)
    pub source_page: Option<usize>,
    /// Rotation angle (0, 90, 180, 270)
    pub rotation: i32,
    /// Scale factor
    pub scale: f32,
    /// Offset within cell for centering
    pub offset: (f32, f32),
}

impl GridCell {
    /// Create a new grid cell
    pub fn new(col: usize, row: usize, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            col,
            row,
            position: (x, y),
            size: (width, height),
            source_page: None,
            rotation: 0,
            scale: 1.0,
            offset: (0.0, 0.0),
        }
    }

    /// Get bounding rectangle
    pub fn bounds(&self) -> Rectangle {
        Rectangle::from_size(self.position.0, self.position.1, self.size.0, self.size.1)
    }

    /// Calculate transformation matrix for placing content
    pub fn transform_matrix(&self) -> String {
        let (x, y) = self.position;
        let (ox, oy) = self.offset;
        let s = self.scale;

        match self.rotation {
            0 => format!("{} 0 0 {} {} {}", s, s, x + ox, y + oy),
            90 => format!("0 {} {} 0 {} {}", s, -s, x + ox + self.size.0, y + oy),
            180 => format!(
                "{} 0 0 {} {} {}",
                -s,
                -s,
                x + ox + self.size.0,
                y + oy + self.size.1
            ),
            270 => format!("0 {} {} 0 {} {}", -s, s, x + ox, y + oy + self.size.1),
            _ => format!("{} 0 0 {} {} {}", s, s, x + ox, y + oy),
        }
    }
}

// ============================================================================
// N-up Generator
// ============================================================================

/// N-up layout generator
pub struct NupGenerator {
    /// Options
    options: NupOptions,
    /// Grid cells
    cells: Vec<GridCell>,
    /// Source PDF data
    source_data: Vec<u8>,
    /// Number of source pages
    source_page_count: usize,
}

impl NupGenerator {
    /// Create a new N-up generator
    pub fn new(options: NupOptions) -> Self {
        let (cols, rows) = options.layout.grid();
        let (cell_w, cell_h) = options.cell_dimensions();
        let indices = options.order.get_indices(cols, rows);

        let mut cells = Vec::with_capacity(cols * rows);
        for (idx, (row, col)) in indices.iter().enumerate() {
            let (x, y) = options.cell_position(*col, *row);
            let mut cell = GridCell::new(*col, *row, x, y, cell_w, cell_h);
            cell.source_page = Some(idx);
            cells.push(cell);
        }

        Self {
            options,
            cells,
            source_data: Vec::new(),
            source_page_count: 0,
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
        self.source_page_count = self.count_pages()?;
        Ok(())
    }

    /// Load from data
    pub fn load_data(&mut self, data: Vec<u8>) -> Result<()> {
        self.source_data = data;
        self.source_page_count = self.count_pages()?;
        Ok(())
    }

    /// Get number of output sheets
    pub fn output_sheet_count(&self) -> usize {
        let pages_per_sheet = self.options.layout.pages_per_sheet();
        (self.source_page_count + pages_per_sheet - 1) / pages_per_sheet
    }

    /// Generate N-up PDF
    pub fn generate(&self) -> Result<Vec<u8>> {
        let pages_per_sheet = self.options.layout.pages_per_sheet();
        let num_sheets = self.output_sheet_count();
        let (page_w, page_h) = if self.options.landscape {
            self.options.page_size.landscape()
        } else {
            self.options.page_size.portrait()
        };

        let mut pdf = Vec::new();

        // PDF header
        pdf.extend(b"%PDF-1.7\n");
        pdf.extend(b"%\xe2\xe3\xcf\xd3\n");

        let mut objects: Vec<String> = Vec::new();
        let mut page_refs: Vec<usize> = Vec::new();

        // Generate each output sheet
        for sheet in 0..num_sheets {
            let start_page = sheet * pages_per_sheet;

            // Build page content
            let mut content = String::new();

            // Background
            if let Some((r, g, b)) = self.options.background {
                content.push_str(&format!(
                    "q {} {} {} rg 0 0 {} {} re f Q\n",
                    r, g, b, page_w, page_h
                ));
            }

            // Draw each cell
            for (cell_idx, cell) in self.cells.iter().enumerate() {
                let source_page = start_page + cell_idx;
                if source_page >= self.source_page_count {
                    break;
                }

                // Border
                if self.options.border {
                    let (x, y) = cell.position;
                    let (w, h) = cell.size;
                    content.push_str(&format!(
                        "q {} w {} {} {} {} re S Q\n",
                        self.options.border_width, x, y, w, h
                    ));
                }

                // Place page content (reference XObject)
                let matrix = cell.transform_matrix();
                content.push_str(&format!("q {} cm /Page{} Do Q\n", matrix, source_page));
            }

            // Content stream object
            let content_obj_num = objects.len() + 1;
            objects.push(format!(
                "{} 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
                content_obj_num,
                content.len(),
                content
            ));

            // Page object
            let page_obj_num = objects.len() + 1;
            objects.push(format!(
                "{} 0 obj\n<< /Type /Page /MediaBox [0 0 {} {}] /Contents {} 0 R >>\nendobj\n",
                page_obj_num, page_w, page_h, content_obj_num
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

    /// Save generated N-up PDF
    pub fn save(&self, output_path: &str) -> Result<()> {
        let pdf = self.generate()?;
        fs::write(output_path, pdf)?;
        Ok(())
    }

    /// Count pages in source PDF
    fn count_pages(&self) -> Result<usize> {
        let content = String::from_utf8_lossy(&self.source_data);

        // Try to find /Count in Pages object
        if let Some(pos) = content.find("/Type /Pages") {
            if let Some(count_pos) = content[pos..].find("/Count ") {
                let start = pos + count_pos + 7;
                let end = content[start..]
                    .find(|c: char| !c.is_ascii_digit())
                    .map(|p| start + p)
                    .unwrap_or(content.len());
                if let Ok(count) = content[start..end].parse::<usize>() {
                    return Ok(count);
                }
            }
        }

        // Fallback: count /Type /Page occurrences
        let count = content.matches("/Type /Page").count();
        if count > 0 {
            // Subtract 1 for /Type /Pages
            Ok(count.saturating_sub(1).max(1))
        } else {
            Ok(1)
        }
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Create an N-up layout with default options
pub fn create_nup(input_path: &str, output_path: &str, layout: NupLayout) -> Result<()> {
    let options = NupOptions::new().layout(layout);
    create_nup_with_options(input_path, output_path, &options)
}

/// Create an N-up layout with custom options
pub fn create_nup_with_options(
    input_path: &str,
    output_path: &str,
    options: &NupOptions,
) -> Result<()> {
    let mut generator = NupGenerator::new(options.clone());
    generator.load(input_path)?;
    generator.save(output_path)
}

/// Create 2-up layout
pub fn create_2up(input_path: &str, output_path: &str, page_size: PageSize) -> Result<()> {
    let options = NupOptions::new()
        .layout(NupLayout::TwoUp)
        .page_size(page_size);
    create_nup_with_options(input_path, output_path, &options)
}

/// Create 4-up layout
pub fn create_4up(input_path: &str, output_path: &str, page_size: PageSize) -> Result<()> {
    let options = NupOptions::new()
        .layout(NupLayout::FourUp)
        .page_size(page_size);
    create_nup_with_options(input_path, output_path, &options)
}

/// Create 9-up layout
pub fn create_9up(input_path: &str, output_path: &str, page_size: PageSize) -> Result<()> {
    let options = NupOptions::new()
        .layout(NupLayout::NineUp)
        .page_size(page_size);
    create_nup_with_options(input_path, output_path, &options)
}

/// Create custom MxN grid layout
pub fn create_grid(
    input_path: &str,
    output_path: &str,
    cols: usize,
    rows: usize,
    page_size: PageSize,
) -> Result<()> {
    let options = NupOptions::new().grid(cols, rows).page_size(page_size);
    create_nup_with_options(input_path, output_path, &options)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_order_ltr_ttb() {
        let indices = PageOrder::LeftToRightTopToBottom.get_indices(3, 2);
        assert_eq!(
            indices,
            vec![(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2)]
        );
    }

    #[test]
    fn test_page_order_rtl_ttb() {
        let indices = PageOrder::RightToLeftTopToBottom.get_indices(3, 2);
        assert_eq!(
            indices,
            vec![(0, 2), (0, 1), (0, 0), (1, 2), (1, 1), (1, 0)]
        );
    }

    #[test]
    fn test_page_order_ttb_ltr() {
        let indices = PageOrder::TopToBottomLeftToRight.get_indices(2, 3);
        assert_eq!(
            indices,
            vec![(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)]
        );
    }

    #[test]
    fn test_nup_layout_grid() {
        assert_eq!(NupLayout::TwoUp.grid(), (1, 2));
        assert_eq!(NupLayout::FourUp.grid(), (2, 2));
        assert_eq!(NupLayout::NineUp.grid(), (3, 3));
        assert_eq!(NupLayout::Custom(5, 3).grid(), (5, 3));
    }

    #[test]
    fn test_nup_layout_pages_per_sheet() {
        assert_eq!(NupLayout::TwoUp.pages_per_sheet(), 2);
        assert_eq!(NupLayout::FourUp.pages_per_sheet(), 4);
        assert_eq!(NupLayout::NineUp.pages_per_sheet(), 9);
        assert_eq!(NupLayout::SixteenUp.pages_per_sheet(), 16);
    }

    #[test]
    fn test_nup_layout_from_grid() {
        assert_eq!(NupLayout::from_grid(2, 2).unwrap(), NupLayout::FourUp);
        assert_eq!(NupLayout::from_grid(3, 3).unwrap(), NupLayout::NineUp);
        assert!(NupLayout::from_grid(0, 2).is_err());
    }

    #[test]
    fn test_nup_options_builder() {
        let options = NupOptions::new()
            .layout(NupLayout::FourUp)
            .page_size(PageSize::A4)
            .border(true)
            .spacing(5.0, 5.0, Unit::Mm)
            .margin(10.0, Unit::Mm);

        assert_eq!(options.layout, NupLayout::FourUp);
        assert_eq!(options.page_size, PageSize::A4);
        assert!(options.border);
        assert!(options.spacing.0 > 14.0 && options.spacing.0 < 15.0); // ~14.17pt
    }

    #[test]
    fn test_cell_dimensions() {
        let options = NupOptions::new()
            .layout(NupLayout::FourUp)
            .page_size(PageSize::Letter);

        let (cell_w, cell_h) = options.cell_dimensions();
        assert!((cell_w - 306.0).abs() < 1.0); // 612 / 2
        assert!((cell_h - 396.0).abs() < 1.0); // 792 / 2
    }

    #[test]
    fn test_cell_position() {
        let options = NupOptions::new()
            .layout(NupLayout::FourUp)
            .page_size(PageSize::Letter)
            .margin(36.0, Unit::Point); // 0.5 inch

        let (x, y) = options.cell_position(0, 0);
        assert!((x - 36.0).abs() < 0.1);

        let (x, y) = options.cell_position(1, 1);
        assert!(x > 200.0);
        assert!(y < 400.0);
    }

    #[test]
    fn test_grid_cell_bounds() {
        let cell = GridCell::new(0, 0, 10.0, 20.0, 100.0, 200.0);
        let bounds = cell.bounds();

        assert!((bounds.llx - 10.0).abs() < 0.01);
        assert!((bounds.lly - 20.0).abs() < 0.01);
        assert!((bounds.urx - 110.0).abs() < 0.01);
        assert!((bounds.ury - 220.0).abs() < 0.01);
    }

    #[test]
    fn test_grid_cell_transform() {
        let mut cell = GridCell::new(0, 0, 100.0, 200.0, 50.0, 50.0);
        cell.scale = 0.5;

        let matrix = cell.transform_matrix();
        assert!(matrix.contains("0.5 0 0 0.5"));
    }

    #[test]
    fn test_nup_generator_creation() {
        let options = NupOptions::new().layout(NupLayout::FourUp);
        let generator = NupGenerator::new(options);

        assert_eq!(generator.cells.len(), 4);
    }
}
