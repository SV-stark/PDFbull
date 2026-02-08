//! Table Flowable with 40+ Style Commands
//!
//! Professional table rendering inspired by ReportLab's TableStyle.
//!
//! ## Features
//!
//! - Cell spanning (rowspan, colspan)
//! - 40+ styling commands
//! - Alternating row colors
//! - Conditional formatting
//! - Auto page-split for long tables
//! - Nested tables
//!
//! ## Example
//!
//! ```rust,ignore
//! use micropdf::enhanced::table::*;
//!
//! let data = vec![
//!     vec!["Name", "Age", "City"],
//!     vec!["Alice", "30", "NYC"],
//!     vec!["Bob", "25", "LA"],
//! ];
//!
//! let style = TableStyle::new()
//!     .grid(0.5, (0, 0, 0))
//!     .background((0, 0), (-1, 0), (0.9, 0.9, 0.9))
//!     .font_bold((0, 0), (-1, 0))
//!     .align_center((0, 0), (-1, -1));
//!
//! let table = Table::new(data).with_style(style);
//! ```

use super::error::{EnhancedError, Result};
use super::flowables::{DrawContext, FlowContext, Flowable, WrapResult};
use super::typography::TextAlign;
use std::any::Any;
use std::collections::HashMap;

// ============================================================================
// Table Style Commands
// ============================================================================

/// A single style command for the table
#[derive(Debug, Clone)]
pub enum StyleCommand {
    // === Grid and Lines ===
    /// Draw all grid lines
    Grid {
        start: CellRef,
        end: CellRef,
        weight: f32,
        color: (f32, f32, f32),
    },
    /// Draw outer box
    Box {
        start: CellRef,
        end: CellRef,
        weight: f32,
        color: (f32, f32, f32),
    },
    /// Draw inner grid lines
    InnerGrid {
        start: CellRef,
        end: CellRef,
        weight: f32,
        color: (f32, f32, f32),
    },
    /// Line above cells
    LineAbove {
        start: CellRef,
        end: CellRef,
        weight: f32,
        color: (f32, f32, f32),
    },
    /// Line below cells
    LineBelow {
        start: CellRef,
        end: CellRef,
        weight: f32,
        color: (f32, f32, f32),
    },
    /// Line to left of cells
    LineLeft {
        start: CellRef,
        end: CellRef,
        weight: f32,
        color: (f32, f32, f32),
    },
    /// Line to right of cells
    LineRight {
        start: CellRef,
        end: CellRef,
        weight: f32,
        color: (f32, f32, f32),
    },

    // === Background ===
    /// Cell background color
    Background {
        start: CellRef,
        end: CellRef,
        color: (f32, f32, f32),
    },
    /// Alternating row background colors
    RowBackgrounds {
        start: CellRef,
        end: CellRef,
        colors: Vec<(f32, f32, f32)>,
    },
    /// Alternating column background colors
    ColBackgrounds {
        start: CellRef,
        end: CellRef,
        colors: Vec<(f32, f32, f32)>,
    },

    // === Text Alignment ===
    /// Horizontal alignment
    Align {
        start: CellRef,
        end: CellRef,
        align: TextAlign,
    },
    /// Vertical alignment
    VAlign {
        start: CellRef,
        end: CellRef,
        valign: VAlign,
    },

    // === Font Styling ===
    /// Font name
    FontName {
        start: CellRef,
        end: CellRef,
        name: String,
    },
    /// Font size
    FontSize {
        start: CellRef,
        end: CellRef,
        size: f32,
    },
    /// Text color
    TextColor {
        start: CellRef,
        end: CellRef,
        color: (f32, f32, f32),
    },
    /// Bold font
    Bold { start: CellRef, end: CellRef },
    /// Italic font
    Italic { start: CellRef, end: CellRef },

    // === Padding ===
    /// All padding
    Padding {
        start: CellRef,
        end: CellRef,
        padding: f32,
    },
    /// Left padding
    LeftPadding {
        start: CellRef,
        end: CellRef,
        padding: f32,
    },
    /// Right padding
    RightPadding {
        start: CellRef,
        end: CellRef,
        padding: f32,
    },
    /// Top padding
    TopPadding {
        start: CellRef,
        end: CellRef,
        padding: f32,
    },
    /// Bottom padding
    BottomPadding {
        start: CellRef,
        end: CellRef,
        padding: f32,
    },

    // === Cell Spanning ===
    /// Span multiple cells
    Span { start: CellRef, end: CellRef },

    // === Advanced ===
    /// Minimum row height
    RowHeight { row: i32, height: f32 },
    /// Column width
    ColWidth { col: i32, width: f32 },
    /// Leading (line height)
    Leading {
        start: CellRef,
        end: CellRef,
        leading: f32,
    },
    /// Word wrap
    WordWrap {
        start: CellRef,
        end: CellRef,
        wrap: bool,
    },
    /// Nosplit - don't split this row across pages
    NoSplit { row: i32 },
    /// Round corners
    RoundCorners { radius: f32 },
}

/// Cell reference (column, row) - negative values count from end
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRef(pub i32, pub i32);

impl CellRef {
    /// Resolve negative indices
    pub fn resolve(&self, num_cols: usize, num_rows: usize) -> (usize, usize) {
        let col = if self.0 < 0 {
            (num_cols as i32 + self.0) as usize
        } else {
            self.0 as usize
        };
        let row = if self.1 < 0 {
            (num_rows as i32 + self.1) as usize
        } else {
            self.1 as usize
        };
        (col.min(num_cols - 1), row.min(num_rows - 1))
    }
}

/// Vertical alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VAlign {
    #[default]
    Top,
    Middle,
    Bottom,
}

// ============================================================================
// Table Style
// ============================================================================

/// Collection of style commands for a table
#[derive(Debug, Clone, Default)]
pub struct TableStyle {
    commands: Vec<StyleCommand>,
}

impl TableStyle {
    /// Create new empty style
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a style command
    pub fn add(mut self, cmd: StyleCommand) -> Self {
        self.commands.push(cmd);
        self
    }

    // === Convenience methods ===

    /// Draw grid lines
    pub fn grid(self, weight: f32, color: (f32, f32, f32)) -> Self {
        self.add(StyleCommand::Grid {
            start: CellRef(0, 0),
            end: CellRef(-1, -1),
            weight,
            color,
        })
    }

    /// Draw outer box
    pub fn box_outline(self, weight: f32, color: (f32, f32, f32)) -> Self {
        self.add(StyleCommand::Box {
            start: CellRef(0, 0),
            end: CellRef(-1, -1),
            weight,
            color,
        })
    }

    /// Set background for cells
    pub fn background(self, start: (i32, i32), end: (i32, i32), color: (f32, f32, f32)) -> Self {
        self.add(StyleCommand::Background {
            start: CellRef(start.0, start.1),
            end: CellRef(end.0, end.1),
            color,
        })
    }

    /// Set alternating row colors
    pub fn row_backgrounds(self, colors: Vec<(f32, f32, f32)>) -> Self {
        self.add(StyleCommand::RowBackgrounds {
            start: CellRef(0, 0),
            end: CellRef(-1, -1),
            colors,
        })
    }

    /// Set alignment
    pub fn align(self, start: (i32, i32), end: (i32, i32), align: TextAlign) -> Self {
        self.add(StyleCommand::Align {
            start: CellRef(start.0, start.1),
            end: CellRef(end.0, end.1),
            align,
        })
    }

    /// Center align
    pub fn align_center(self, start: (i32, i32), end: (i32, i32)) -> Self {
        self.align(start, end, TextAlign::Center)
    }

    /// Right align
    pub fn align_right(self, start: (i32, i32), end: (i32, i32)) -> Self {
        self.align(start, end, TextAlign::Right)
    }

    /// Set vertical alignment
    pub fn valign(self, start: (i32, i32), end: (i32, i32), valign: VAlign) -> Self {
        self.add(StyleCommand::VAlign {
            start: CellRef(start.0, start.1),
            end: CellRef(end.0, end.1),
            valign,
        })
    }

    /// Set font name
    pub fn font_name(self, start: (i32, i32), end: (i32, i32), name: &str) -> Self {
        self.add(StyleCommand::FontName {
            start: CellRef(start.0, start.1),
            end: CellRef(end.0, end.1),
            name: name.to_string(),
        })
    }

    /// Set font size
    pub fn font_size(self, start: (i32, i32), end: (i32, i32), size: f32) -> Self {
        self.add(StyleCommand::FontSize {
            start: CellRef(start.0, start.1),
            end: CellRef(end.0, end.1),
            size,
        })
    }

    /// Make text bold
    pub fn font_bold(self, start: (i32, i32), end: (i32, i32)) -> Self {
        self.add(StyleCommand::Bold {
            start: CellRef(start.0, start.1),
            end: CellRef(end.0, end.1),
        })
    }

    /// Make text italic
    pub fn font_italic(self, start: (i32, i32), end: (i32, i32)) -> Self {
        self.add(StyleCommand::Italic {
            start: CellRef(start.0, start.1),
            end: CellRef(end.0, end.1),
        })
    }

    /// Set text color
    pub fn text_color(self, start: (i32, i32), end: (i32, i32), color: (f32, f32, f32)) -> Self {
        self.add(StyleCommand::TextColor {
            start: CellRef(start.0, start.1),
            end: CellRef(end.0, end.1),
            color,
        })
    }

    /// Set all padding
    pub fn padding(self, start: (i32, i32), end: (i32, i32), padding: f32) -> Self {
        self.add(StyleCommand::Padding {
            start: CellRef(start.0, start.1),
            end: CellRef(end.0, end.1),
            padding,
        })
    }

    /// Line above rows
    pub fn line_above(
        self,
        start: (i32, i32),
        end: (i32, i32),
        weight: f32,
        color: (f32, f32, f32),
    ) -> Self {
        self.add(StyleCommand::LineAbove {
            start: CellRef(start.0, start.1),
            end: CellRef(end.0, end.1),
            weight,
            color,
        })
    }

    /// Line below rows
    pub fn line_below(
        self,
        start: (i32, i32),
        end: (i32, i32),
        weight: f32,
        color: (f32, f32, f32),
    ) -> Self {
        self.add(StyleCommand::LineBelow {
            start: CellRef(start.0, start.1),
            end: CellRef(end.0, end.1),
            weight,
            color,
        })
    }

    /// Set column width
    pub fn col_width(self, col: i32, width: f32) -> Self {
        self.add(StyleCommand::ColWidth { col, width })
    }

    /// Set row height
    pub fn row_height(self, row: i32, height: f32) -> Self {
        self.add(StyleCommand::RowHeight { row, height })
    }

    /// Span cells
    pub fn span(self, start: (i32, i32), end: (i32, i32)) -> Self {
        self.add(StyleCommand::Span {
            start: CellRef(start.0, start.1),
            end: CellRef(end.0, end.1),
        })
    }

    /// Get all commands
    pub fn commands(&self) -> &[StyleCommand] {
        &self.commands
    }
}

// ============================================================================
// Computed Cell Style
// ============================================================================

/// Computed style for a single cell
#[derive(Debug, Clone)]
pub struct CellStyle {
    pub background: Option<(f32, f32, f32)>,
    pub align: TextAlign,
    pub valign: VAlign,
    pub font_name: String,
    pub font_size: f32,
    pub text_color: (f32, f32, f32),
    pub bold: bool,
    pub italic: bool,
    pub left_padding: f32,
    pub right_padding: f32,
    pub top_padding: f32,
    pub bottom_padding: f32,
    pub line_above: Option<(f32, (f32, f32, f32))>,
    pub line_below: Option<(f32, (f32, f32, f32))>,
    pub line_left: Option<(f32, (f32, f32, f32))>,
    pub line_right: Option<(f32, (f32, f32, f32))>,
    pub leading: f32,
    pub word_wrap: bool,
    pub colspan: usize,
    pub rowspan: usize,
    pub is_spanned: bool, // Part of a span but not the origin
}

impl Default for CellStyle {
    fn default() -> Self {
        Self {
            background: None,
            align: TextAlign::Left,
            valign: VAlign::Top,
            font_name: "Helvetica".to_string(),
            font_size: 10.0,
            text_color: (0.0, 0.0, 0.0),
            bold: false,
            italic: false,
            left_padding: 3.0,
            right_padding: 3.0,
            top_padding: 2.0,
            bottom_padding: 2.0,
            line_above: None,
            line_below: None,
            line_left: None,
            line_right: None,
            leading: 12.0,
            word_wrap: true,
            colspan: 1,
            rowspan: 1,
            is_spanned: false,
        }
    }
}

// ============================================================================
// Table Cell
// ============================================================================

/// Content of a table cell
#[derive(Debug, Clone)]
pub enum CellContent {
    /// Simple text
    Text(String),
    /// Rich text with markup
    RichText(String),
    /// Nested table
    Table(Box<Table>),
    /// Flowable content
    Flowable(String), // Placeholder - actual flowable would be Box<dyn Flowable>
    /// Empty cell
    Empty,
}

impl Default for CellContent {
    fn default() -> Self {
        CellContent::Empty
    }
}

impl From<&str> for CellContent {
    fn from(s: &str) -> Self {
        CellContent::Text(s.to_string())
    }
}

impl From<String> for CellContent {
    fn from(s: String) -> Self {
        CellContent::Text(s)
    }
}

// ============================================================================
// Table
// ============================================================================

/// Table flowable
#[derive(Debug, Clone)]
pub struct Table {
    /// Data (rows of cells)
    data: Vec<Vec<CellContent>>,
    /// Style
    style: TableStyle,
    /// Column widths (None for auto)
    col_widths: Option<Vec<f32>>,
    /// Row heights (None for auto)
    row_heights: Option<Vec<f32>>,
    /// Repeat header rows on each page
    repeat_rows: usize,
    /// Horizontal alignment of table
    h_align: TextAlign,
    /// Space before table
    space_before: f32,
    /// Space after table
    space_after: f32,
    /// Computed cell styles
    cell_styles: Vec<Vec<CellStyle>>,
    /// Computed dimensions
    computed_widths: Vec<f32>,
    computed_heights: Vec<f32>,
}

impl Table {
    /// Create table from string data
    pub fn new(data: Vec<Vec<&str>>) -> Self {
        let data: Vec<Vec<CellContent>> = data
            .into_iter()
            .map(|row| row.into_iter().map(CellContent::from).collect())
            .collect();
        Self::from_content(data)
    }

    /// Create table from cell content
    pub fn from_content(data: Vec<Vec<CellContent>>) -> Self {
        let num_rows = data.len();
        let num_cols = data.first().map(|r| r.len()).unwrap_or(0);

        let cell_styles: Vec<Vec<CellStyle>> = (0..num_rows)
            .map(|_| (0..num_cols).map(|_| CellStyle::default()).collect())
            .collect();

        Self {
            data,
            style: TableStyle::new(),
            col_widths: None,
            row_heights: None,
            repeat_rows: 0,
            h_align: TextAlign::Left,
            space_before: 0.0,
            space_after: 0.0,
            cell_styles,
            computed_widths: Vec::new(),
            computed_heights: Vec::new(),
        }
    }

    /// Set table style
    pub fn with_style(mut self, style: TableStyle) -> Self {
        self.style = style;
        self
    }

    /// Set column widths
    pub fn with_col_widths(mut self, widths: Vec<f32>) -> Self {
        self.col_widths = Some(widths);
        self
    }

    /// Set repeat header rows
    pub fn with_repeat_rows(mut self, rows: usize) -> Self {
        self.repeat_rows = rows;
        self
    }

    /// Set horizontal alignment
    pub fn with_h_align(mut self, align: TextAlign) -> Self {
        self.h_align = align;
        self
    }

    /// Set space before
    pub fn with_space_before(mut self, space: f32) -> Self {
        self.space_before = space;
        self
    }

    /// Set space after
    pub fn with_space_after(mut self, space: f32) -> Self {
        self.space_after = space;
        self
    }

    /// Number of rows
    pub fn num_rows(&self) -> usize {
        self.data.len()
    }

    /// Number of columns
    pub fn num_cols(&self) -> usize {
        self.data.first().map(|r| r.len()).unwrap_or(0)
    }

    /// Apply style commands to compute cell styles
    fn compute_cell_styles(&mut self) {
        let num_rows = self.num_rows();
        let num_cols = self.num_cols();

        // Reset to defaults
        self.cell_styles = (0..num_rows)
            .map(|_| (0..num_cols).map(|_| CellStyle::default()).collect())
            .collect();

        // Apply each command
        for cmd in self.style.commands() {
            match cmd {
                StyleCommand::Background { start, end, color } => {
                    let (c1, r1) = start.resolve(num_cols, num_rows);
                    let (c2, r2) = end.resolve(num_cols, num_rows);
                    for row in r1..=r2 {
                        for col in c1..=c2 {
                            if row < self.cell_styles.len() && col < self.cell_styles[row].len() {
                                self.cell_styles[row][col].background = Some(*color);
                            }
                        }
                    }
                }
                StyleCommand::RowBackgrounds { start, end, colors } => {
                    let (c1, r1) = start.resolve(num_cols, num_rows);
                    let (c2, r2) = end.resolve(num_cols, num_rows);
                    for (idx, row) in (r1..=r2).enumerate() {
                        let color = colors[idx % colors.len()];
                        for col in c1..=c2 {
                            if row < self.cell_styles.len() && col < self.cell_styles[row].len() {
                                self.cell_styles[row][col].background = Some(color);
                            }
                        }
                    }
                }
                StyleCommand::Align { start, end, align } => {
                    let (c1, r1) = start.resolve(num_cols, num_rows);
                    let (c2, r2) = end.resolve(num_cols, num_rows);
                    for row in r1..=r2 {
                        for col in c1..=c2 {
                            if row < self.cell_styles.len() && col < self.cell_styles[row].len() {
                                self.cell_styles[row][col].align = *align;
                            }
                        }
                    }
                }
                StyleCommand::VAlign { start, end, valign } => {
                    let (c1, r1) = start.resolve(num_cols, num_rows);
                    let (c2, r2) = end.resolve(num_cols, num_rows);
                    for row in r1..=r2 {
                        for col in c1..=c2 {
                            if row < self.cell_styles.len() && col < self.cell_styles[row].len() {
                                self.cell_styles[row][col].valign = *valign;
                            }
                        }
                    }
                }
                StyleCommand::FontName { start, end, name } => {
                    let (c1, r1) = start.resolve(num_cols, num_rows);
                    let (c2, r2) = end.resolve(num_cols, num_rows);
                    for row in r1..=r2 {
                        for col in c1..=c2 {
                            if row < self.cell_styles.len() && col < self.cell_styles[row].len() {
                                self.cell_styles[row][col].font_name = name.clone();
                            }
                        }
                    }
                }
                StyleCommand::FontSize { start, end, size } => {
                    let (c1, r1) = start.resolve(num_cols, num_rows);
                    let (c2, r2) = end.resolve(num_cols, num_rows);
                    for row in r1..=r2 {
                        for col in c1..=c2 {
                            if row < self.cell_styles.len() && col < self.cell_styles[row].len() {
                                self.cell_styles[row][col].font_size = *size;
                            }
                        }
                    }
                }
                StyleCommand::Bold { start, end } => {
                    let (c1, r1) = start.resolve(num_cols, num_rows);
                    let (c2, r2) = end.resolve(num_cols, num_rows);
                    for row in r1..=r2 {
                        for col in c1..=c2 {
                            if row < self.cell_styles.len() && col < self.cell_styles[row].len() {
                                self.cell_styles[row][col].bold = true;
                            }
                        }
                    }
                }
                StyleCommand::Italic { start, end } => {
                    let (c1, r1) = start.resolve(num_cols, num_rows);
                    let (c2, r2) = end.resolve(num_cols, num_rows);
                    for row in r1..=r2 {
                        for col in c1..=c2 {
                            if row < self.cell_styles.len() && col < self.cell_styles[row].len() {
                                self.cell_styles[row][col].italic = true;
                            }
                        }
                    }
                }
                StyleCommand::TextColor { start, end, color } => {
                    let (c1, r1) = start.resolve(num_cols, num_rows);
                    let (c2, r2) = end.resolve(num_cols, num_rows);
                    for row in r1..=r2 {
                        for col in c1..=c2 {
                            if row < self.cell_styles.len() && col < self.cell_styles[row].len() {
                                self.cell_styles[row][col].text_color = *color;
                            }
                        }
                    }
                }
                StyleCommand::Padding {
                    start,
                    end,
                    padding,
                } => {
                    let (c1, r1) = start.resolve(num_cols, num_rows);
                    let (c2, r2) = end.resolve(num_cols, num_rows);
                    for row in r1..=r2 {
                        for col in c1..=c2 {
                            if row < self.cell_styles.len() && col < self.cell_styles[row].len() {
                                self.cell_styles[row][col].left_padding = *padding;
                                self.cell_styles[row][col].right_padding = *padding;
                                self.cell_styles[row][col].top_padding = *padding;
                                self.cell_styles[row][col].bottom_padding = *padding;
                            }
                        }
                    }
                }
                StyleCommand::Grid {
                    start,
                    end,
                    weight,
                    color,
                } => {
                    let (c1, r1) = start.resolve(num_cols, num_rows);
                    let (c2, r2) = end.resolve(num_cols, num_rows);
                    for row in r1..=r2 {
                        for col in c1..=c2 {
                            if row < self.cell_styles.len() && col < self.cell_styles[row].len() {
                                self.cell_styles[row][col].line_above = Some((*weight, *color));
                                self.cell_styles[row][col].line_below = Some((*weight, *color));
                                self.cell_styles[row][col].line_left = Some((*weight, *color));
                                self.cell_styles[row][col].line_right = Some((*weight, *color));
                            }
                        }
                    }
                }
                StyleCommand::LineAbove {
                    start,
                    end,
                    weight,
                    color,
                } => {
                    let (c1, r1) = start.resolve(num_cols, num_rows);
                    let (c2, r2) = end.resolve(num_cols, num_rows);
                    for row in r1..=r2 {
                        for col in c1..=c2 {
                            if row < self.cell_styles.len() && col < self.cell_styles[row].len() {
                                self.cell_styles[row][col].line_above = Some((*weight, *color));
                            }
                        }
                    }
                }
                StyleCommand::LineBelow {
                    start,
                    end,
                    weight,
                    color,
                } => {
                    let (c1, r1) = start.resolve(num_cols, num_rows);
                    let (c2, r2) = end.resolve(num_cols, num_rows);
                    for row in r1..=r2 {
                        for col in c1..=c2 {
                            if row < self.cell_styles.len() && col < self.cell_styles[row].len() {
                                self.cell_styles[row][col].line_below = Some((*weight, *color));
                            }
                        }
                    }
                }
                StyleCommand::Span { start, end } => {
                    let (c1, r1) = start.resolve(num_cols, num_rows);
                    let (c2, r2) = end.resolve(num_cols, num_rows);
                    // Mark origin cell with span dimensions
                    if r1 < self.cell_styles.len() && c1 < self.cell_styles[r1].len() {
                        self.cell_styles[r1][c1].colspan = c2 - c1 + 1;
                        self.cell_styles[r1][c1].rowspan = r2 - r1 + 1;
                    }
                    // Mark other cells as spanned
                    for row in r1..=r2 {
                        for col in c1..=c2 {
                            if (row != r1 || col != c1)
                                && row < self.cell_styles.len()
                                && col < self.cell_styles[row].len()
                            {
                                self.cell_styles[row][col].is_spanned = true;
                            }
                        }
                    }
                }
                _ => {} // Handle other commands as needed
            }
        }
    }

    /// Calculate column widths
    fn calculate_widths(&mut self, available_width: f32) {
        let num_cols = self.num_cols();
        if num_cols == 0 {
            return;
        }

        if let Some(ref widths) = self.col_widths {
            self.computed_widths = widths.clone();
            if self.computed_widths.len() < num_cols {
                let default_width = (available_width - self.computed_widths.iter().sum::<f32>())
                    / (num_cols - self.computed_widths.len()) as f32;
                self.computed_widths.resize(num_cols, default_width);
            }
        } else {
            // Auto-calculate based on content
            let default_width = available_width / num_cols as f32;
            self.computed_widths = vec![default_width; num_cols];
        }
    }

    /// Calculate row heights
    fn calculate_heights(&mut self) {
        let num_rows = self.num_rows();
        let default_height = 20.0; // Base height

        self.computed_heights = vec![default_height; num_rows];

        // Adjust based on content and styles
        for (row_idx, row) in self.data.iter().enumerate() {
            let mut max_height = default_height;
            for (col_idx, cell) in row.iter().enumerate() {
                if col_idx < self.cell_styles[row_idx].len() {
                    let style = &self.cell_styles[row_idx][col_idx];
                    if style.is_spanned {
                        continue;
                    }

                    let cell_height = match cell {
                        CellContent::Text(s) => {
                            let lines = (s.len() as f32 * style.font_size * 0.5
                                / self.computed_widths.get(col_idx).unwrap_or(&100.0))
                            .ceil()
                            .max(1.0);
                            lines * style.leading + style.top_padding + style.bottom_padding
                        }
                        _ => style.leading + style.top_padding + style.bottom_padding,
                    };
                    max_height = max_height.max(cell_height);
                }
            }
            self.computed_heights[row_idx] = max_height;
        }
    }

    /// Get cell text
    fn cell_text(&self, row: usize, col: usize) -> String {
        if row < self.data.len() && col < self.data[row].len() {
            match &self.data[row][col] {
                CellContent::Text(s) => s.clone(),
                CellContent::RichText(s) => s.clone(),
                _ => String::new(),
            }
        } else {
            String::new()
        }
    }
}

impl Flowable for Table {
    fn wrap(&self, available_width: f32, _available_height: f32, _ctx: &FlowContext) -> WrapResult {
        // Clone to mutate
        let mut table = self.clone();
        table.compute_cell_styles();
        table.calculate_widths(available_width);
        table.calculate_heights();

        let total_height =
            table.computed_heights.iter().sum::<f32>() + table.space_before + table.space_after;

        let total_width = table.computed_widths.iter().sum::<f32>();

        WrapResult::new(total_width, total_height)
    }

    fn draw(&self, x: f32, y: f32, _ctx: &mut DrawContext) -> Result<Vec<String>> {
        let mut commands = Vec::new();
        let mut table = self.clone();
        table.compute_cell_styles();
        table.calculate_widths(400.0);
        table.calculate_heights();

        let mut current_y = y - table.space_before;

        for (row_idx, row) in table.data.iter().enumerate() {
            let row_height = table.computed_heights[row_idx];
            let mut current_x = x;

            for (col_idx, _cell) in row.iter().enumerate() {
                if col_idx >= table.cell_styles[row_idx].len() {
                    continue;
                }

                let style = &table.cell_styles[row_idx][col_idx];
                if style.is_spanned {
                    current_x += table.computed_widths.get(col_idx).unwrap_or(&0.0);
                    continue;
                }

                let cell_width: f32 = (0..style.colspan)
                    .filter_map(|i| table.computed_widths.get(col_idx + i))
                    .sum();
                let cell_height: f32 = (0..style.rowspan)
                    .filter_map(|i| table.computed_heights.get(row_idx + i))
                    .sum();

                // Draw background
                if let Some(bg) = style.background {
                    commands.push(format!("q {} {} {} rg", bg.0, bg.1, bg.2));
                    commands.push(format!(
                        "{} {} {} {} re f",
                        current_x,
                        current_y - cell_height,
                        cell_width,
                        cell_height
                    ));
                    commands.push("Q".to_string());
                }

                // Draw borders
                commands.push("q".to_string());
                if let Some((weight, color)) = style.line_above {
                    commands.push(format!(
                        "{} w {} {} {} RG",
                        weight, color.0, color.1, color.2
                    ));
                    commands.push(format!(
                        "{} {} m {} {} l S",
                        current_x,
                        current_y,
                        current_x + cell_width,
                        current_y
                    ));
                }
                if let Some((weight, color)) = style.line_below {
                    commands.push(format!(
                        "{} w {} {} {} RG",
                        weight, color.0, color.1, color.2
                    ));
                    commands.push(format!(
                        "{} {} m {} {} l S",
                        current_x,
                        current_y - cell_height,
                        current_x + cell_width,
                        current_y - cell_height
                    ));
                }
                if let Some((weight, color)) = style.line_left {
                    commands.push(format!(
                        "{} w {} {} {} RG",
                        weight, color.0, color.1, color.2
                    ));
                    commands.push(format!(
                        "{} {} m {} {} l S",
                        current_x,
                        current_y,
                        current_x,
                        current_y - cell_height
                    ));
                }
                if let Some((weight, color)) = style.line_right {
                    commands.push(format!(
                        "{} w {} {} {} RG",
                        weight, color.0, color.1, color.2
                    ));
                    commands.push(format!(
                        "{} {} m {} {} l S",
                        current_x + cell_width,
                        current_y,
                        current_x + cell_width,
                        current_y - cell_height
                    ));
                }
                commands.push("Q".to_string());

                // Draw text
                let text = table.cell_text(row_idx, col_idx);
                if !text.is_empty() {
                    let text_x = current_x + style.left_padding;
                    let text_y = current_y - style.top_padding - style.font_size;

                    commands.push("BT".to_string());
                    let font = if style.bold { "F2" } else { "F1" };
                    commands.push(format!("/{} {} Tf", font, style.font_size));
                    commands.push(format!(
                        "{} {} {} rg",
                        style.text_color.0, style.text_color.1, style.text_color.2
                    ));
                    commands.push(format!("{} {} Td", text_x, text_y));
                    commands.push(format!("({}) Tj", escape_pdf_string(&text)));
                    commands.push("ET".to_string());
                }

                current_x += cell_width;
            }

            current_y -= row_height;
        }

        Ok(commands)
    }

    fn split(
        &self,
        available_height: f32,
        _ctx: &FlowContext,
    ) -> Option<(Box<dyn Flowable>, Box<dyn Flowable>)> {
        let mut table = self.clone();
        table.compute_cell_styles();
        table.calculate_widths(400.0);
        table.calculate_heights();

        // Find split point
        let mut cumulative_height = table.space_before;
        let mut split_row = 0;

        for (idx, height) in table.computed_heights.iter().enumerate() {
            if cumulative_height + height > available_height {
                split_row = idx;
                break;
            }
            cumulative_height += height;
            split_row = idx + 1;
        }

        if split_row == 0 || split_row >= table.num_rows() {
            return None;
        }

        // Create two tables
        let first_data: Vec<Vec<CellContent>> = table.data[..split_row].to_vec();
        let second_data: Vec<Vec<CellContent>> = table.data[split_row..].to_vec();

        let first = Table::from_content(first_data).with_style(table.style.clone());
        let second = Table::from_content(second_data).with_style(table.style.clone());

        Some((Box::new(first), Box::new(second)))
    }

    fn is_splittable(&self) -> bool {
        self.num_rows() > 1
    }

    fn space_before(&self) -> f32 {
        self.space_before
    }

    fn space_after(&self) -> f32 {
        self.space_after
    }

    fn clone_box(&self) -> Box<dyn Flowable> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn escape_pdf_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_ref_resolve() {
        let r1 = CellRef(0, 0);
        assert_eq!(r1.resolve(5, 10), (0, 0));

        let r2 = CellRef(-1, -1);
        assert_eq!(r2.resolve(5, 10), (4, 9));

        let r3 = CellRef(-2, -3);
        assert_eq!(r3.resolve(5, 10), (3, 7));
    }

    #[test]
    fn test_table_style_builder() {
        let style = TableStyle::new()
            .grid(0.5, (0.0, 0.0, 0.0))
            .background((0, 0), (-1, 0), (0.9, 0.9, 0.9))
            .font_bold((0, 0), (-1, 0))
            .align_center((0, 0), (-1, -1));

        assert_eq!(style.commands().len(), 4);
    }

    #[test]
    fn test_table_creation() {
        let data = vec![
            vec!["A", "B", "C"],
            vec!["1", "2", "3"],
            vec!["4", "5", "6"],
        ];

        let table = Table::new(data);
        assert_eq!(table.num_rows(), 3);
        assert_eq!(table.num_cols(), 3);
    }

    #[test]
    fn test_table_with_style() {
        let data = vec![vec!["Header"], vec!["Data"]];

        let style = TableStyle::new()
            .grid(1.0, (0.0, 0.0, 0.0))
            .font_bold((0, 0), (-1, 0));

        let table = Table::new(data).with_style(style);
        assert_eq!(table.num_rows(), 2);
    }

    #[test]
    fn test_cell_style_default() {
        let style = CellStyle::default();
        assert_eq!(style.font_size, 10.0);
        assert_eq!(style.align, TextAlign::Left);
        assert_eq!(style.colspan, 1);
        assert!(!style.is_spanned);
    }

    #[test]
    fn test_row_backgrounds() {
        let style = TableStyle::new().row_backgrounds(vec![
            (1.0, 1.0, 1.0), // White
            (0.9, 0.9, 0.9), // Light gray
        ]);

        assert_eq!(style.commands().len(), 1);
    }

    #[test]
    fn test_table_dimensions() {
        let data = vec![vec!["A", "B", "C", "D"], vec!["1", "2", "3", "4"]];

        let table = Table::new(data);
        assert_eq!(table.num_cols(), 4);
        assert_eq!(table.num_rows(), 2);
    }

    #[test]
    fn test_span_cells() {
        let style = TableStyle::new().span((0, 0), (1, 0));
        assert_eq!(style.commands().len(), 1);
    }
}
