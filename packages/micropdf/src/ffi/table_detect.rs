//! Table detection and structure recognition
//!
//! This module provides algorithms for detecting tables in PDF pages,
//! extracting their structure, and converting them to various formats.

use std::ffi::{CString, c_char, c_int};
use std::sync::LazyLock;

use crate::ffi::{Handle, HandleStore};
use crate::fitz::geometry::Rect;

// ============================================================================
// Handle Management
// ============================================================================

/// Handle store for detected tables
static TABLES: LazyLock<HandleStore<Table>> = LazyLock::new(HandleStore::new);

/// Handle store for table detectors
static TABLE_DETECTORS: LazyLock<HandleStore<TableDetector>> = LazyLock::new(HandleStore::new);

// ============================================================================
// Table Cell
// ============================================================================

/// A single cell in a table
#[derive(Debug, Clone)]
pub struct TableCell {
    /// Row index (0-based)
    pub row: usize,
    /// Column index (0-based)
    pub col: usize,
    /// Number of rows this cell spans
    pub rowspan: usize,
    /// Number of columns this cell spans
    pub colspan: usize,
    /// Cell bounds
    pub bounds: Rect,
    /// Cell text content
    pub text: String,
    /// Is this a header cell
    pub is_header: bool,
    /// Alignment (0=left, 1=center, 2=right)
    pub alignment: u8,
}

impl TableCell {
    /// Create a new table cell
    pub fn new(row: usize, col: usize, bounds: Rect) -> Self {
        Self {
            row,
            col,
            rowspan: 1,
            colspan: 1,
            bounds,
            text: String::new(),
            is_header: false,
            alignment: 0, // Left
        }
    }

    /// Set cell text
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
}

// ============================================================================
// Table Row
// ============================================================================

/// A row in a table
#[derive(Debug, Clone)]
pub struct TableRow {
    /// Row index
    pub index: usize,
    /// Cells in this row
    pub cells: Vec<TableCell>,
    /// Row bounds
    pub bounds: Rect,
    /// Is this a header row
    pub is_header: bool,
}

impl TableRow {
    /// Create a new row
    pub fn new(index: usize, bounds: Rect) -> Self {
        Self {
            index,
            cells: Vec::new(),
            bounds,
            is_header: false,
        }
    }

    /// Add a cell to this row
    pub fn add_cell(&mut self, cell: TableCell) {
        self.cells.push(cell);
    }

    /// Get number of columns (accounting for colspan)
    pub fn column_count(&self) -> usize {
        self.cells.iter().map(|c| c.colspan).sum()
    }
}

// ============================================================================
// Table
// ============================================================================

/// A detected table
#[derive(Debug, Clone)]
pub struct Table {
    /// Table bounds
    pub bounds: Rect,
    /// Table rows
    pub rows: Vec<TableRow>,
    /// Number of columns
    pub num_cols: usize,
    /// Has header row
    pub has_header: bool,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
}

impl Table {
    /// Create a new empty table
    pub fn new(bounds: Rect) -> Self {
        Self {
            bounds,
            rows: Vec::new(),
            num_cols: 0,
            has_header: false,
            confidence: 0.0,
        }
    }

    /// Add a row to the table
    pub fn add_row(&mut self, row: TableRow) {
        let cols = row.column_count();
        if cols > self.num_cols {
            self.num_cols = cols;
        }
        self.rows.push(row);
    }

    /// Get number of rows
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Get a cell by row and column
    pub fn get_cell(&self, row: usize, col: usize) -> Option<&TableCell> {
        self.rows.get(row)?.cells.iter().find(|c| c.col == col)
    }

    /// Export to CSV
    pub fn to_csv(&self) -> String {
        let mut output = String::new();

        for row in &self.rows {
            let mut col_values: Vec<String> = vec![String::new(); self.num_cols];

            for cell in &row.cells {
                if cell.col < self.num_cols {
                    // Escape quotes and wrap in quotes if contains comma
                    let text = cell.text.replace('"', "\"\"");
                    if text.contains(',') || text.contains('\n') || text.contains('"') {
                        col_values[cell.col] = format!("\"{}\"", text);
                    } else {
                        col_values[cell.col] = text;
                    }
                }
            }

            output.push_str(&col_values.join(","));
            output.push('\n');
        }

        output
    }

    /// Export to HTML
    pub fn to_html(&self) -> String {
        let mut output = String::from("<table>\n");

        for row in &self.rows {
            if row.is_header {
                output.push_str("  <thead>\n    <tr>\n");
            } else {
                output.push_str("  <tr>\n");
            }

            for cell in &row.cells {
                let tag = if cell.is_header { "th" } else { "td" };
                let mut attrs = String::new();

                if cell.rowspan > 1 {
                    attrs.push_str(&format!(" rowspan=\"{}\"", cell.rowspan));
                }
                if cell.colspan > 1 {
                    attrs.push_str(&format!(" colspan=\"{}\"", cell.colspan));
                }

                let align = match cell.alignment {
                    1 => " style=\"text-align: center\"",
                    2 => " style=\"text-align: right\"",
                    _ => "",
                };

                // Escape HTML
                let text = cell
                    .text
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");

                output.push_str(&format!(
                    "      <{}{}{}>{}</{}>\n",
                    tag, attrs, align, text, tag
                ));
            }

            if row.is_header {
                output.push_str("    </tr>\n  </thead>\n");
            } else {
                output.push_str("  </tr>\n");
            }
        }

        output.push_str("</table>\n");
        output
    }

    /// Export to Markdown
    pub fn to_markdown(&self) -> String {
        if self.rows.is_empty() || self.num_cols == 0 {
            return String::new();
        }

        let mut output = String::new();

        // Header row
        if let Some(first_row) = self.rows.first() {
            output.push('|');
            for i in 0..self.num_cols {
                let text = first_row
                    .cells
                    .iter()
                    .find(|c| c.col == i)
                    .map(|c| c.text.as_str())
                    .unwrap_or("");
                output.push_str(&format!(" {} |", text));
            }
            output.push('\n');

            // Separator
            output.push('|');
            for _ in 0..self.num_cols {
                output.push_str(" --- |");
            }
            output.push('\n');
        }

        // Data rows
        for row in self.rows.iter().skip(1) {
            output.push('|');
            for i in 0..self.num_cols {
                let text = row
                    .cells
                    .iter()
                    .find(|c| c.col == i)
                    .map(|c| c.text.as_str())
                    .unwrap_or("");
                output.push_str(&format!(" {} |", text));
            }
            output.push('\n');
        }

        output
    }
}

// ============================================================================
// Table Detection Configuration
// ============================================================================

/// Configuration for table detection
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TableDetectorConfig {
    /// Minimum number of rows to consider as table
    pub min_rows: usize,
    /// Minimum number of columns to consider as table
    pub min_cols: usize,
    /// Tolerance for column alignment (in points)
    pub alignment_tolerance: f32,
    /// Minimum confidence threshold
    pub min_confidence: f32,
    /// Detect header rows
    pub detect_headers: bool,
    /// Use ruling lines for detection
    pub use_ruling_lines: bool,
    /// Use whitespace patterns for detection
    pub use_whitespace: bool,
}

impl Default for TableDetectorConfig {
    fn default() -> Self {
        Self {
            min_rows: 2,
            min_cols: 2,
            alignment_tolerance: 5.0,
            min_confidence: 0.5,
            detect_headers: true,
            use_ruling_lines: true,
            use_whitespace: true,
        }
    }
}

// ============================================================================
// Table Detector
// ============================================================================

/// Table detector for finding tables in pages
pub struct TableDetector {
    /// Configuration
    config: TableDetectorConfig,
    /// Detected tables
    tables: Vec<Table>,
}

impl TableDetector {
    /// Create a new table detector
    pub fn new(config: TableDetectorConfig) -> Self {
        Self {
            config,
            tables: Vec::new(),
        }
    }

    /// Detect tables from text lines with x positions
    ///
    /// This uses a column-alignment heuristic: if many text elements
    /// align at the same x positions, they likely form table columns.
    pub fn detect_from_positions(&mut self, lines: &[(Rect, String)]) {
        if lines.len() < self.config.min_rows {
            return;
        }

        // Group lines by y position (rows)
        let mut rows_by_y: Vec<(f32, Vec<(Rect, &str)>)> = Vec::new();
        let y_tolerance = 3.0;

        for (rect, text) in lines {
            let y = rect.y0;
            let mut found = false;

            for (row_y, row_items) in &mut rows_by_y {
                if (y - *row_y).abs() < y_tolerance {
                    row_items.push((*rect, text.as_str()));
                    found = true;
                    break;
                }
            }

            if !found {
                rows_by_y.push((y, vec![(*rect, text.as_str())]));
            }
        }

        // Sort rows by y position
        rows_by_y.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Find column positions
        let mut x_positions: Vec<f32> = Vec::new();
        for (_, items) in &rows_by_y {
            for (rect, _) in items {
                x_positions.push(rect.x0);
            }
        }
        x_positions.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Cluster x positions
        let tolerance = self.config.alignment_tolerance;
        let mut columns: Vec<f32> = Vec::new();
        let mut last_x = f32::NEG_INFINITY;

        for x in x_positions {
            if x - last_x > tolerance {
                columns.push(x);
            }
            last_x = x;
        }

        // Need at least min_cols columns
        if columns.len() < self.config.min_cols {
            return;
        }

        // Need at least min_rows rows
        if rows_by_y.len() < self.config.min_rows {
            return;
        }

        // Calculate confidence based on alignment consistency
        let mut aligned_count = 0;
        let mut total_count = 0;

        for (_, items) in &rows_by_y {
            for (rect, _) in items {
                total_count += 1;
                for &col_x in &columns {
                    if (rect.x0 - col_x).abs() < tolerance {
                        aligned_count += 1;
                        break;
                    }
                }
            }
        }

        let confidence = if total_count > 0 {
            aligned_count as f32 / total_count as f32
        } else {
            0.0
        };

        if confidence < self.config.min_confidence {
            return;
        }

        // Build table structure
        let mut table_bounds = Rect::new(f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        let mut table = Table::new(table_bounds);
        table.confidence = confidence;
        table.num_cols = columns.len();

        for (row_idx, (row_y, items)) in rows_by_y.iter().enumerate() {
            // Calculate row bounds
            let mut row_bounds = Rect::new(f32::MAX, *row_y, f32::MIN, *row_y + 12.0);
            for (rect, _) in items {
                row_bounds.x0 = row_bounds.x0.min(rect.x0);
                row_bounds.x1 = row_bounds.x1.max(rect.x1);
                row_bounds.y1 = row_bounds.y1.max(rect.y1);
            }

            let mut row = TableRow::new(row_idx, row_bounds);

            // Assign items to columns
            for (rect, text) in items {
                // Find which column this item belongs to
                let col_idx = columns
                    .iter()
                    .position(|&col_x| (rect.x0 - col_x).abs() < tolerance)
                    .unwrap_or(0);

                let mut cell = TableCell::new(row_idx, col_idx, *rect);
                cell.set_text(*text);

                // Detect alignment based on position within column
                if columns.len() > col_idx + 1 {
                    let col_width = columns.get(col_idx + 1).unwrap_or(&rect.x1) - columns[col_idx];
                    let text_width = rect.x1 - rect.x0;
                    let offset = rect.x0 - columns[col_idx];

                    if offset > col_width * 0.4 && offset + text_width < col_width * 0.6 {
                        cell.alignment = 1; // Center
                    } else if offset > col_width * 0.6 {
                        cell.alignment = 2; // Right
                    }
                }

                row.add_cell(cell);
            }

            // Detect header row (first row with all bold/caps or different styling)
            if self.config.detect_headers && row_idx == 0 {
                let all_caps = items.iter().all(|(_, t)| {
                    t.chars()
                        .filter(|c| c.is_alphabetic())
                        .all(|c| c.is_uppercase())
                });
                if all_caps && !items.is_empty() {
                    row.is_header = true;
                    for cell in &mut row.cells {
                        cell.is_header = true;
                    }
                    table.has_header = true;
                }
            }

            table_bounds.x0 = table_bounds.x0.min(row_bounds.x0);
            table_bounds.y0 = table_bounds.y0.min(row_bounds.y0);
            table_bounds.x1 = table_bounds.x1.max(row_bounds.x1);
            table_bounds.y1 = table_bounds.y1.max(row_bounds.y1);

            table.add_row(row);
        }

        table.bounds = table_bounds;
        self.tables.push(table);
    }

    /// Get detected tables
    pub fn tables(&self) -> &[Table] {
        &self.tables
    }

    /// Clear detected tables
    pub fn clear(&mut self) {
        self.tables.clear();
    }

    /// Get number of detected tables
    pub fn table_count(&self) -> usize {
        self.tables.len()
    }
}

// ============================================================================
// FFI Functions
// ============================================================================

/// Create a new table detector
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_table_detector(
    _ctx: Handle,
    min_rows: c_int,
    min_cols: c_int,
    min_confidence: f32,
) -> Handle {
    let config = TableDetectorConfig {
        min_rows: min_rows.max(2) as usize,
        min_cols: min_cols.max(2) as usize,
        min_confidence: min_confidence.clamp(0.0, 1.0),
        ..Default::default()
    };

    let detector = TableDetector::new(config);
    TABLE_DETECTORS.insert(detector)
}

/// Drop a table detector
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_table_detector(_ctx: Handle, detector: Handle) {
    TABLE_DETECTORS.remove(detector);
}

/// Get number of detected tables
#[unsafe(no_mangle)]
pub extern "C" fn fz_table_detector_count(_ctx: Handle, detector: Handle) -> c_int {
    if let Some(arc) = TABLE_DETECTORS.get(detector) {
        if let Ok(d) = arc.lock() {
            return d.table_count() as c_int;
        }
    }
    0
}

/// Clear detected tables
#[unsafe(no_mangle)]
pub extern "C" fn fz_table_detector_clear(_ctx: Handle, detector: Handle) {
    if let Some(arc) = TABLE_DETECTORS.get(detector) {
        if let Ok(mut d) = arc.lock() {
            d.clear();
        }
    }
}

/// Create a new table
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_table(_ctx: Handle, x0: f32, y0: f32, x1: f32, y1: f32) -> Handle {
    let table = Table::new(Rect::new(x0, y0, x1, y1));
    TABLES.insert(table)
}

/// Drop a table
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_table(_ctx: Handle, table: Handle) {
    TABLES.remove(table);
}

/// Get table row count
#[unsafe(no_mangle)]
pub extern "C" fn fz_table_row_count(_ctx: Handle, table: Handle) -> c_int {
    if let Some(arc) = TABLES.get(table) {
        if let Ok(t) = arc.lock() {
            return t.row_count() as c_int;
        }
    }
    0
}

/// Get table column count
#[unsafe(no_mangle)]
pub extern "C" fn fz_table_col_count(_ctx: Handle, table: Handle) -> c_int {
    if let Some(arc) = TABLES.get(table) {
        if let Ok(t) = arc.lock() {
            return t.num_cols as c_int;
        }
    }
    0
}

/// Get table confidence
#[unsafe(no_mangle)]
pub extern "C" fn fz_table_confidence(_ctx: Handle, table: Handle) -> f32 {
    if let Some(arc) = TABLES.get(table) {
        if let Ok(t) = arc.lock() {
            return t.confidence;
        }
    }
    0.0
}

/// Export table to CSV (returns allocated string, caller must free)
#[unsafe(no_mangle)]
pub extern "C" fn fz_table_to_csv(_ctx: Handle, table: Handle) -> *mut c_char {
    if let Some(arc) = TABLES.get(table) {
        if let Ok(t) = arc.lock() {
            if let Ok(s) = CString::new(t.to_csv()) {
                return s.into_raw();
            }
        }
    }
    std::ptr::null_mut()
}

/// Export table to HTML (returns allocated string, caller must free)
#[unsafe(no_mangle)]
pub extern "C" fn fz_table_to_html(_ctx: Handle, table: Handle) -> *mut c_char {
    if let Some(arc) = TABLES.get(table) {
        if let Ok(t) = arc.lock() {
            if let Ok(s) = CString::new(t.to_html()) {
                return s.into_raw();
            }
        }
    }
    std::ptr::null_mut()
}

/// Export table to Markdown (returns allocated string, caller must free)
#[unsafe(no_mangle)]
pub extern "C" fn fz_table_to_markdown(_ctx: Handle, table: Handle) -> *mut c_char {
    if let Some(arc) = TABLES.get(table) {
        if let Ok(t) = arc.lock() {
            if let Ok(s) = CString::new(t.to_markdown()) {
                return s.into_raw();
            }
        }
    }
    std::ptr::null_mut()
}

/// Free a string returned by table export functions
#[unsafe(no_mangle)]
pub extern "C" fn fz_free_table_string(_ctx: Handle, s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Get cell text (returns allocated string, caller must free)
#[unsafe(no_mangle)]
pub extern "C" fn fz_table_get_cell_text(
    _ctx: Handle,
    table: Handle,
    row: c_int,
    col: c_int,
) -> *mut c_char {
    if let Some(arc) = TABLES.get(table) {
        if let Ok(t) = arc.lock() {
            if let Some(cell) = t.get_cell(row as usize, col as usize) {
                if let Ok(s) = CString::new(cell.text.as_str()) {
                    return s.into_raw();
                }
            }
        }
    }
    std::ptr::null_mut()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_cell() {
        let mut cell = TableCell::new(0, 0, Rect::new(0.0, 0.0, 100.0, 20.0));
        assert_eq!(cell.row, 0);
        assert_eq!(cell.col, 0);
        assert_eq!(cell.rowspan, 1);
        assert_eq!(cell.colspan, 1);

        cell.set_text("Hello");
        assert_eq!(cell.text, "Hello");
    }

    #[test]
    fn test_table_row() {
        let mut row = TableRow::new(0, Rect::new(0.0, 0.0, 300.0, 20.0));
        row.add_cell(TableCell::new(0, 0, Rect::new(0.0, 0.0, 100.0, 20.0)));
        row.add_cell(TableCell::new(0, 1, Rect::new(100.0, 0.0, 200.0, 20.0)));

        assert_eq!(row.column_count(), 2);
    }

    #[test]
    fn test_table_structure() {
        let mut table = Table::new(Rect::new(0.0, 0.0, 300.0, 100.0));

        let mut row1 = TableRow::new(0, Rect::new(0.0, 0.0, 300.0, 20.0));
        let mut cell1 = TableCell::new(0, 0, Rect::new(0.0, 0.0, 150.0, 20.0));
        cell1.set_text("Header 1");
        cell1.is_header = true;
        let mut cell2 = TableCell::new(0, 1, Rect::new(150.0, 0.0, 300.0, 20.0));
        cell2.set_text("Header 2");
        cell2.is_header = true;
        row1.cells.push(cell1);
        row1.cells.push(cell2);
        row1.is_header = true;
        table.add_row(row1);

        let mut row2 = TableRow::new(1, Rect::new(0.0, 20.0, 300.0, 40.0));
        let mut cell3 = TableCell::new(1, 0, Rect::new(0.0, 20.0, 150.0, 40.0));
        cell3.set_text("Data 1");
        let mut cell4 = TableCell::new(1, 1, Rect::new(150.0, 20.0, 300.0, 40.0));
        cell4.set_text("Data 2");
        row2.cells.push(cell3);
        row2.cells.push(cell4);
        table.add_row(row2);

        assert_eq!(table.row_count(), 2);
        assert_eq!(table.num_cols, 2);
    }

    #[test]
    fn test_table_to_csv() {
        let mut table = Table::new(Rect::new(0.0, 0.0, 200.0, 40.0));
        table.num_cols = 2;

        let mut row = TableRow::new(0, Rect::new(0.0, 0.0, 200.0, 20.0));
        let mut c1 = TableCell::new(0, 0, Rect::new(0.0, 0.0, 100.0, 20.0));
        c1.set_text("A");
        let mut c2 = TableCell::new(0, 1, Rect::new(100.0, 0.0, 200.0, 20.0));
        c2.set_text("B");
        row.cells.push(c1);
        row.cells.push(c2);
        table.add_row(row);

        let csv = table.to_csv();
        assert!(csv.contains("A,B"));
    }

    #[test]
    fn test_table_to_html() {
        let mut table = Table::new(Rect::new(0.0, 0.0, 200.0, 40.0));
        table.num_cols = 2;

        let mut row = TableRow::new(0, Rect::new(0.0, 0.0, 200.0, 20.0));
        let mut c1 = TableCell::new(0, 0, Rect::new(0.0, 0.0, 100.0, 20.0));
        c1.set_text("X");
        row.cells.push(c1);
        table.add_row(row);

        let html = table.to_html();
        assert!(html.contains("<table>"));
        assert!(html.contains("<td>X</td>"));
        assert!(html.contains("</table>"));
    }

    #[test]
    fn test_table_to_markdown() {
        let mut table = Table::new(Rect::new(0.0, 0.0, 200.0, 40.0));
        table.num_cols = 2;

        let mut row1 = TableRow::new(0, Rect::new(0.0, 0.0, 200.0, 20.0));
        let mut c1 = TableCell::new(0, 0, Rect::new(0.0, 0.0, 100.0, 20.0));
        c1.set_text("H1");
        let mut c2 = TableCell::new(0, 1, Rect::new(100.0, 0.0, 200.0, 20.0));
        c2.set_text("H2");
        row1.cells.push(c1);
        row1.cells.push(c2);
        table.add_row(row1);

        let mut row2 = TableRow::new(1, Rect::new(0.0, 20.0, 200.0, 40.0));
        let mut c3 = TableCell::new(1, 0, Rect::new(0.0, 20.0, 100.0, 40.0));
        c3.set_text("D1");
        let mut c4 = TableCell::new(1, 1, Rect::new(100.0, 20.0, 200.0, 40.0));
        c4.set_text("D2");
        row2.cells.push(c3);
        row2.cells.push(c4);
        table.add_row(row2);

        let md = table.to_markdown();
        assert!(md.contains("| H1 |"));
        assert!(md.contains("| --- |"));
        assert!(md.contains("| D1 |"));
    }

    #[test]
    fn test_table_detector_config() {
        let config = TableDetectorConfig::default();
        assert_eq!(config.min_rows, 2);
        assert_eq!(config.min_cols, 2);
        assert!(config.detect_headers);
    }

    #[test]
    fn test_table_detection() {
        let config = TableDetectorConfig::default();
        let mut detector = TableDetector::new(config);

        // Create test data with aligned columns
        let lines: Vec<(Rect, String)> = vec![
            (Rect::new(10.0, 10.0, 50.0, 22.0), "Col1".to_string()),
            (Rect::new(100.0, 10.0, 150.0, 22.0), "Col2".to_string()),
            (Rect::new(200.0, 10.0, 250.0, 22.0), "Col3".to_string()),
            (Rect::new(10.0, 30.0, 50.0, 42.0), "A".to_string()),
            (Rect::new(100.0, 30.0, 150.0, 42.0), "B".to_string()),
            (Rect::new(200.0, 30.0, 250.0, 42.0), "C".to_string()),
            (Rect::new(10.0, 50.0, 50.0, 62.0), "D".to_string()),
            (Rect::new(100.0, 50.0, 150.0, 62.0), "E".to_string()),
            (Rect::new(200.0, 50.0, 250.0, 62.0), "F".to_string()),
        ];

        detector.detect_from_positions(&lines);

        assert_eq!(detector.table_count(), 1);
        let table = &detector.tables()[0];
        assert_eq!(table.row_count(), 3);
        assert_eq!(table.num_cols, 3);
        assert!(table.confidence > 0.5);
    }

    #[test]
    fn test_table_ffi() {
        let handle = fz_new_table(0, 0.0, 0.0, 100.0, 100.0);
        assert!(handle != 0);

        let rows = fz_table_row_count(0, handle);
        assert_eq!(rows, 0);

        let cols = fz_table_col_count(0, handle);
        assert_eq!(cols, 0);

        fz_drop_table(0, handle);
    }

    #[test]
    fn test_detector_ffi() {
        let handle = fz_new_table_detector(0, 2, 2, 0.5);
        assert!(handle != 0);

        let count = fz_table_detector_count(0, handle);
        assert_eq!(count, 0);

        fz_table_detector_clear(0, handle);

        fz_drop_table_detector(0, handle);
    }
}
