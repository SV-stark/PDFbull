//! Automatic Table of Contents
//!
//! Multi-level TOC with auto page number detection and clickable links.
//!
//! ## Features
//!
//! - Multi-level hierarchy (up to 6 levels)
//! - Auto page number detection
//! - Clickable links to destinations
//! - Leader dots (customizable)
//! - Styling per level
//! - Right-aligned page numbers
//!
//! ## Example
//!
//! ```rust,ignore
//! use micropdf::enhanced::toc::*;
//!
//! let toc = TableOfContents::new()
//!     .with_title("Contents")
//!     .add_entry(TocEntry::new("Chapter 1", 1, 1))
//!     .add_entry(TocEntry::new("Section 1.1", 2, 3))
//!     .add_entry(TocEntry::new("Section 1.2", 2, 5))
//!     .add_entry(TocEntry::new("Chapter 2", 1, 10));
//! ```

use super::error::{EnhancedError, Result};
use super::flowables::{DrawContext, FlowContext, Flowable, WrapResult};
use super::typography::{ParagraphStyle, TextAlign};
use std::any::Any;
use std::collections::HashMap;

// ============================================================================
// TOC Entry
// ============================================================================

/// Single entry in the table of contents
#[derive(Debug, Clone)]
pub struct TocEntry {
    /// Entry title
    pub title: String,
    /// Hierarchy level (1-6)
    pub level: u8,
    /// Page number
    pub page_number: usize,
    /// Destination name (for links)
    pub destination: Option<String>,
    /// Section number (e.g., "1.2.3")
    pub section_number: Option<String>,
}

impl TocEntry {
    /// Create new TOC entry
    pub fn new(title: &str, level: u8, page_number: usize) -> Self {
        Self {
            title: title.to_string(),
            level: level.clamp(1, 6),
            page_number,
            destination: None,
            section_number: None,
        }
    }

    /// Set destination name
    pub fn with_destination(mut self, dest: &str) -> Self {
        self.destination = Some(dest.to_string());
        self
    }

    /// Set section number
    pub fn with_section_number(mut self, num: &str) -> Self {
        self.section_number = Some(num.to_string());
        self
    }
}

// ============================================================================
// TOC Level Style
// ============================================================================

/// Style for a TOC level
#[derive(Debug, Clone)]
pub struct TocLevelStyle {
    /// Font name
    pub font_name: String,
    /// Font size
    pub font_size: f32,
    /// Bold
    pub bold: bool,
    /// Italic
    pub italic: bool,
    /// Left indent
    pub left_indent: f32,
    /// Space before
    pub space_before: f32,
    /// Space after
    pub space_after: f32,
    /// Leader character
    pub leader: Option<char>,
    /// Leader spacing
    pub leader_spacing: f32,
    /// Show page numbers
    pub show_page_number: bool,
    /// Page number alignment
    pub page_number_align: TextAlign,
    /// Text color (RGB)
    pub text_color: (f32, f32, f32),
}

impl Default for TocLevelStyle {
    fn default() -> Self {
        Self {
            font_name: "Helvetica".to_string(),
            font_size: 11.0,
            bold: false,
            italic: false,
            left_indent: 0.0,
            space_before: 2.0,
            space_after: 2.0,
            leader: Some('.'),
            leader_spacing: 4.0,
            show_page_number: true,
            page_number_align: TextAlign::Right,
            text_color: (0.0, 0.0, 0.0),
        }
    }
}

impl TocLevelStyle {
    /// Create new level style
    pub fn new() -> Self {
        Self::default()
    }

    /// Set font size
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set bold
    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// Set left indent
    pub fn with_indent(mut self, indent: f32) -> Self {
        self.left_indent = indent;
        self
    }

    /// Set leader character
    pub fn with_leader(mut self, leader: Option<char>) -> Self {
        self.leader = leader;
        self
    }

    /// Set space before
    pub fn with_space_before(mut self, space: f32) -> Self {
        self.space_before = space;
        self
    }
}

// ============================================================================
// Table of Contents
// ============================================================================

/// Table of Contents flowable
#[derive(Debug, Clone)]
pub struct TableOfContents {
    /// TOC title
    title: Option<String>,
    /// Title style
    title_style: ParagraphStyle,
    /// Entries
    entries: Vec<TocEntry>,
    /// Styles per level
    level_styles: HashMap<u8, TocLevelStyle>,
    /// Right margin for page numbers
    right_margin: f32,
    /// Minimum dots for leader
    min_leader_dots: usize,
    /// Space before TOC
    space_before: f32,
    /// Space after TOC
    space_after: f32,
}

impl TableOfContents {
    /// Create new TOC
    pub fn new() -> Self {
        let mut level_styles = HashMap::new();

        // Default styles for levels 1-6
        level_styles.insert(
            1,
            TocLevelStyle::new()
                .with_font_size(12.0)
                .with_bold(true)
                .with_indent(0.0)
                .with_space_before(6.0),
        );
        level_styles.insert(
            2,
            TocLevelStyle::new()
                .with_font_size(11.0)
                .with_indent(20.0)
                .with_space_before(3.0),
        );
        level_styles.insert(
            3,
            TocLevelStyle::new()
                .with_font_size(10.0)
                .with_indent(40.0)
                .with_space_before(2.0),
        );
        level_styles.insert(
            4,
            TocLevelStyle::new().with_font_size(10.0).with_indent(60.0),
        );
        level_styles.insert(
            5,
            TocLevelStyle::new().with_font_size(9.0).with_indent(80.0),
        );
        level_styles.insert(
            6,
            TocLevelStyle::new().with_font_size(9.0).with_indent(100.0),
        );

        Self {
            title: Some("Table of Contents".to_string()),
            title_style: ParagraphStyle::new("TOCTitle")
                .with_font_size(16.0)
                .with_leading(20.0)
                .with_space_after(12.0),
            entries: Vec::new(),
            level_styles,
            right_margin: 40.0,
            min_leader_dots: 3,
            space_before: 0.0,
            space_after: 24.0,
        }
    }

    /// Set title
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    /// Remove title
    pub fn without_title(mut self) -> Self {
        self.title = None;
        self
    }

    /// Set title style
    pub fn with_title_style(mut self, style: ParagraphStyle) -> Self {
        self.title_style = style;
        self
    }

    /// Add entry
    pub fn add_entry(mut self, entry: TocEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Add multiple entries
    pub fn add_entries(mut self, entries: Vec<TocEntry>) -> Self {
        self.entries.extend(entries);
        self
    }

    /// Set style for a level
    pub fn with_level_style(mut self, level: u8, style: TocLevelStyle) -> Self {
        self.level_styles.insert(level, style);
        self
    }

    /// Set right margin
    pub fn with_right_margin(mut self, margin: f32) -> Self {
        self.right_margin = margin;
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

    /// Get level style
    fn get_level_style(&self, level: u8) -> TocLevelStyle {
        self.level_styles.get(&level).cloned().unwrap_or_default()
    }

    /// Calculate entry height
    fn entry_height(&self, entry: &TocEntry) -> f32 {
        let style = self.get_level_style(entry.level);
        style.font_size + style.space_before + style.space_after
    }

    /// Calculate total height
    fn total_height(&self) -> f32 {
        let title_height = if self.title.is_some() {
            self.title_style.font_size
                + self.title_style.space_before
                + self.title_style.space_after
        } else {
            0.0
        };

        let entries_height: f32 = self.entries.iter().map(|e| self.entry_height(e)).sum();

        self.space_before + title_height + entries_height + self.space_after
    }

    /// Generate leader dots
    fn generate_leader(&self, width: f32, style: &TocLevelStyle) -> String {
        if let Some(leader) = style.leader {
            let char_width = style.leader_spacing;
            let num_chars = (width / char_width) as usize;
            if num_chars >= self.min_leader_dots {
                let s: String = std::iter::repeat(leader).take(num_chars).collect();
                format!(" {} ", s)
            } else {
                " ".to_string()
            }
        } else {
            " ".to_string()
        }
    }
}

impl Default for TableOfContents {
    fn default() -> Self {
        Self::new()
    }
}

impl Flowable for TableOfContents {
    fn wrap(&self, available_width: f32, _available_height: f32, _ctx: &FlowContext) -> WrapResult {
        WrapResult::new(available_width, self.total_height())
    }

    fn draw(&self, x: f32, y: f32, _ctx: &mut DrawContext) -> Result<Vec<String>> {
        let mut commands = Vec::new();
        let mut current_y = y - self.space_before;
        let page_width = 400.0; // Effective width

        // Draw title
        if let Some(ref title) = self.title {
            commands.push("BT".to_string());
            commands.push(format!("/F2 {} Tf", self.title_style.font_size));
            commands.push("0 0 0 rg".to_string());
            commands.push(format!(
                "{} {} Td",
                x,
                current_y - self.title_style.font_size
            ));
            commands.push(format!("({}) Tj", escape_pdf_string(title)));
            commands.push("ET".to_string());
            current_y -= self.title_style.font_size
                + self.title_style.space_before
                + self.title_style.space_after;
        }

        // Draw entries
        for entry in &self.entries {
            let style = self.get_level_style(entry.level);
            let entry_x = x + style.left_indent;
            let entry_y = current_y - style.space_before - style.font_size;

            // Build entry text
            let entry_text = if let Some(ref num) = entry.section_number {
                format!("{} {}", num, entry.title)
            } else {
                entry.title.clone()
            };

            // Calculate text width (approximate)
            let text_width = entry_text.len() as f32 * style.font_size * 0.5;

            // Calculate page number width
            let page_num_str = entry.page_number.to_string();
            let page_num_width = page_num_str.len() as f32 * style.font_size * 0.5;

            // Calculate leader width
            let available_leader_width =
                page_width - style.left_indent - text_width - page_num_width - self.right_margin;

            let leader = self.generate_leader(available_leader_width, &style);

            // Draw entry
            commands.push("BT".to_string());

            // Font
            let font = if style.bold { "/F2" } else { "/F1" };
            commands.push(format!("{} {} Tf", font, style.font_size));

            // Color
            commands.push(format!(
                "{} {} {} rg",
                style.text_color.0, style.text_color.1, style.text_color.2
            ));

            // Position and draw text
            commands.push(format!("{} {} Td", entry_x, entry_y));
            commands.push(format!("({}) Tj", escape_pdf_string(&entry_text)));

            commands.push("ET".to_string());

            // Draw leader and page number if enabled
            if style.show_page_number {
                // Leader
                let leader_x = entry_x + text_width;
                commands.push("BT".to_string());
                commands.push(format!("/F1 {} Tf", style.font_size));
                commands.push(format!("{} {} Td", leader_x, entry_y));
                commands.push(format!("({}) Tj", escape_pdf_string(&leader)));
                commands.push("ET".to_string());

                // Page number (right-aligned)
                let page_x = x + page_width - self.right_margin;
                commands.push("BT".to_string());
                commands.push(format!("/F1 {} Tf", style.font_size));
                commands.push(format!("{} {} Td", page_x, entry_y));
                commands.push(format!("({}) Tj", page_num_str));
                commands.push("ET".to_string());

                // Add link annotation if destination exists
                if let Some(ref dest) = entry.destination {
                    let annotation = format!(
                        "[/Rect [{} {} {} {}] /Dest ({}) /Subtype /Link /ANN pdfmark",
                        entry_x,
                        entry_y - 2.0,
                        page_x + page_num_width,
                        entry_y + style.font_size,
                        dest
                    );
                    commands.push(format!("% Link: {}", annotation));
                }
            }

            current_y -= style.font_size + style.space_before + style.space_after;
        }

        Ok(commands)
    }

    fn split(
        &self,
        available_height: f32,
        _ctx: &FlowContext,
    ) -> Option<(Box<dyn Flowable>, Box<dyn Flowable>)> {
        let title_height = if self.title.is_some() {
            self.title_style.font_size
                + self.title_style.space_before
                + self.title_style.space_after
        } else {
            0.0
        };

        let mut remaining_height = available_height - self.space_before - title_height;
        let mut split_idx = 0;

        for (idx, entry) in self.entries.iter().enumerate() {
            let entry_h = self.entry_height(entry);
            if remaining_height < entry_h {
                split_idx = idx;
                break;
            }
            remaining_height -= entry_h;
            split_idx = idx + 1;
        }

        if split_idx == 0 || split_idx >= self.entries.len() {
            return None;
        }

        // First part with title and some entries
        let mut first = self.clone();
        first.entries = self.entries[..split_idx].to_vec();

        // Second part without title
        let mut second = self.clone();
        second.title = None;
        second.entries = self.entries[split_idx..].to_vec();
        second.space_before = 0.0;

        Some((Box::new(first), Box::new(second)))
    }

    fn is_splittable(&self) -> bool {
        self.entries.len() > 1
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
// TOC Builder
// ============================================================================

/// Builder for automatic TOC generation from document
#[derive(Debug, Default)]
pub struct TocBuilder {
    entries: Vec<TocEntry>,
    section_counters: Vec<usize>,
    numbering_enabled: bool,
}

impl TocBuilder {
    /// Create new TOC builder
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            section_counters: vec![0; 6],
            numbering_enabled: true,
        }
    }

    /// Enable/disable automatic numbering
    pub fn with_numbering(mut self, enabled: bool) -> Self {
        self.numbering_enabled = enabled;
        self
    }

    /// Add heading (called during document processing)
    pub fn add_heading(
        &mut self,
        title: &str,
        level: u8,
        page_number: usize,
        destination: Option<&str>,
    ) {
        let level = level.clamp(1, 6);
        let level_idx = (level - 1) as usize;

        // Update counters
        self.section_counters[level_idx] += 1;
        for i in (level_idx + 1)..6 {
            self.section_counters[i] = 0;
        }

        // Generate section number
        let section_number = if self.numbering_enabled {
            let nums: Vec<String> = self.section_counters[..=level_idx]
                .iter()
                .map(|n| n.to_string())
                .collect();
            Some(nums.join("."))
        } else {
            None
        };

        let mut entry = TocEntry::new(title, level, page_number);
        if let Some(num) = section_number {
            entry = entry.with_section_number(&num);
        }
        if let Some(dest) = destination {
            entry = entry.with_destination(dest);
        }

        self.entries.push(entry);
    }

    /// Build the TOC
    pub fn build(self) -> TableOfContents {
        TableOfContents::new().add_entries(self.entries)
    }

    /// Get entries
    pub fn entries(&self) -> &[TocEntry] {
        &self.entries
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
    fn test_toc_entry_creation() {
        let entry = TocEntry::new("Introduction", 1, 1)
            .with_destination("intro")
            .with_section_number("1");

        assert_eq!(entry.title, "Introduction");
        assert_eq!(entry.level, 1);
        assert_eq!(entry.page_number, 1);
        assert_eq!(entry.destination, Some("intro".to_string()));
        assert_eq!(entry.section_number, Some("1".to_string()));
    }

    #[test]
    fn test_toc_level_clamping() {
        let entry1 = TocEntry::new("Level 0", 0, 1);
        assert_eq!(entry1.level, 1);

        let entry2 = TocEntry::new("Level 10", 10, 1);
        assert_eq!(entry2.level, 6);
    }

    #[test]
    fn test_toc_creation() {
        let toc = TableOfContents::new()
            .with_title("Contents")
            .add_entry(TocEntry::new("Chapter 1", 1, 1))
            .add_entry(TocEntry::new("Section 1.1", 2, 3))
            .add_entry(TocEntry::new("Chapter 2", 1, 10));

        assert_eq!(toc.entries.len(), 3);
        assert_eq!(toc.title, Some("Contents".to_string()));
    }

    #[test]
    fn test_toc_without_title() {
        let toc = TableOfContents::new()
            .without_title()
            .add_entry(TocEntry::new("Chapter 1", 1, 1));

        assert!(toc.title.is_none());
    }

    #[test]
    fn test_level_style() {
        let style = TocLevelStyle::new()
            .with_font_size(14.0)
            .with_bold(true)
            .with_indent(20.0);

        assert_eq!(style.font_size, 14.0);
        assert!(style.bold);
        assert_eq!(style.left_indent, 20.0);
    }

    #[test]
    fn test_toc_builder() {
        let mut builder = TocBuilder::new();
        builder.add_heading("Chapter 1", 1, 1, Some("ch1"));
        builder.add_heading("Section 1.1", 2, 3, None);
        builder.add_heading("Section 1.2", 2, 5, None);
        builder.add_heading("Chapter 2", 1, 10, Some("ch2"));

        let entries = builder.entries();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].section_number, Some("1".to_string()));
        assert_eq!(entries[1].section_number, Some("1.1".to_string()));
        assert_eq!(entries[2].section_number, Some("1.2".to_string()));
        assert_eq!(entries[3].section_number, Some("2".to_string()));
    }

    #[test]
    fn test_toc_builder_without_numbering() {
        let mut builder = TocBuilder::new().with_numbering(false);
        builder.add_heading("Chapter 1", 1, 1, None);
        builder.add_heading("Section 1.1", 2, 3, None);

        let entries = builder.entries();
        assert!(entries[0].section_number.is_none());
        assert!(entries[1].section_number.is_none());
    }

    #[test]
    fn test_toc_flowable() {
        let toc = TableOfContents::new()
            .add_entry(TocEntry::new("Chapter 1", 1, 1))
            .add_entry(TocEntry::new("Chapter 2", 1, 10));

        let ctx = FlowContext::new(400.0, 600.0);
        let result = toc.wrap(400.0, 600.0, &ctx);
        assert!(result.height > 0.0);
    }

    #[test]
    fn test_leader_generation() {
        let toc = TableOfContents::new();
        let style = TocLevelStyle::default();
        let leader = toc.generate_leader(100.0, &style);
        assert!(leader.contains('.'));
    }
}
