//! Page Box Management for Print Production
//!
//! PDF defines 5 page boxes for different purposes in the print workflow:
//!
//! - **MediaBox**: Physical medium size (required, largest)
//! - **CropBox**: Default clipping region (defaults to MediaBox)
//! - **BleedBox**: Extended content area for trimming (defaults to CropBox)
//! - **TrimBox**: Intended final page size after trimming (defaults to CropBox)
//! - **ArtBox**: Meaningful content area (defaults to CropBox)
//!
//! ## Box Hierarchy (from largest to smallest typical usage):
//!
//! ```text
//! ┌─────────────────────────────────────────┐  MediaBox (physical size)
//! │  ┌───────────────────────────────────┐  │  CropBox (clipping)
//! │  │  ┌─────────────────────────────┐  │  │  BleedBox (includes bleed)
//! │  │  │  ┌───────────────────────┐  │  │  │  TrimBox (final size)
//! │  │  │  │  ┌─────────────────┐  │  │  │  │  ArtBox (content)
//! │  │  │  │  │                 │  │  │  │  │
//! │  │  │  │  │    Content      │  │  │  │  │
//! │  │  │  │  │                 │  │  │  │  │
//! │  │  │  │  └─────────────────┘  │  │  │  │
//! │  │  │  └───────────────────────┘  │  │  │
//! │  │  └─────────────────────────────┘  │  │
//! │  └───────────────────────────────────┘  │
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use micropdf::enhanced::page_boxes::{PageBoxManager, BoxType};
//!
//! // Add 3mm bleed to all pages
//! let manager = PageBoxManager::new("input.pdf")?;
//! manager.add_bleed(3.0, Unit::Mm)?;
//! manager.save("output.pdf")?;
//!
//! // Set specific boxes
//! manager.set_box(BoxType::TrimBox, 0, 0.0, 0.0, 210.0, 297.0)?;  // A4
//! ```

use super::error::{EnhancedError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// ============================================================================
// Units
// ============================================================================

/// Measurement units for page dimensions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unit {
    /// Points (1/72 inch) - PDF native unit
    Point,
    /// Inches
    Inch,
    /// Millimeters
    Mm,
    /// Centimeters
    Cm,
}

impl Unit {
    /// Convert value to points
    pub fn to_points(&self, value: f32) -> f32 {
        match self {
            Unit::Point => value,
            Unit::Inch => value * 72.0,
            Unit::Mm => value * 72.0 / 25.4,
            Unit::Cm => value * 72.0 / 2.54,
        }
    }

    /// Convert points to this unit
    pub fn from_points(&self, points: f32) -> f32 {
        match self {
            Unit::Point => points,
            Unit::Inch => points / 72.0,
            Unit::Mm => points * 25.4 / 72.0,
            Unit::Cm => points * 2.54 / 72.0,
        }
    }
}

impl Default for Unit {
    fn default() -> Self {
        Unit::Point
    }
}

// ============================================================================
// Box Types
// ============================================================================

/// The 5 PDF page box types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoxType {
    /// Physical medium size (required)
    MediaBox,
    /// Default clipping region
    CropBox,
    /// Region for trimming (includes bleed area)
    BleedBox,
    /// Final page dimensions after trimming
    TrimBox,
    /// Meaningful content extent
    ArtBox,
}

impl BoxType {
    /// PDF key name for this box type
    pub fn pdf_key(&self) -> &'static str {
        match self {
            BoxType::MediaBox => "MediaBox",
            BoxType::CropBox => "CropBox",
            BoxType::BleedBox => "BleedBox",
            BoxType::TrimBox => "TrimBox",
            BoxType::ArtBox => "ArtBox",
        }
    }

    /// Description of box purpose
    pub fn description(&self) -> &'static str {
        match self {
            BoxType::MediaBox => "Physical medium size (paper size)",
            BoxType::CropBox => "Default clipping region for display/printing",
            BoxType::BleedBox => "Extended area for trimming (includes bleed)",
            BoxType::TrimBox => "Intended final page size after trimming",
            BoxType::ArtBox => "Meaningful content area",
        }
    }

    /// All box types in hierarchy order (largest to smallest typical usage)
    pub fn all() -> &'static [BoxType] {
        &[
            BoxType::MediaBox,
            BoxType::CropBox,
            BoxType::BleedBox,
            BoxType::TrimBox,
            BoxType::ArtBox,
        ]
    }
}

// ============================================================================
// Rectangle
// ============================================================================

/// A rectangle representing a page box
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rectangle {
    /// Lower-left X coordinate (in points)
    pub llx: f32,
    /// Lower-left Y coordinate (in points)
    pub lly: f32,
    /// Upper-right X coordinate (in points)
    pub urx: f32,
    /// Upper-right Y coordinate (in points)
    pub ury: f32,
}

impl Rectangle {
    /// Create a new rectangle from coordinates
    pub fn new(llx: f32, lly: f32, urx: f32, ury: f32) -> Self {
        Self { llx, lly, urx, ury }
    }

    /// Create a rectangle from origin and size
    pub fn from_size(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            llx: x,
            lly: y,
            urx: x + width,
            ury: y + height,
        }
    }

    /// Create a rectangle for a standard page size
    pub fn from_page_size(size: PageSize) -> Self {
        let (width, height) = size.dimensions();
        Self::from_size(0.0, 0.0, width, height)
    }

    /// Width of the rectangle
    pub fn width(&self) -> f32 {
        self.urx - self.llx
    }

    /// Height of the rectangle
    pub fn height(&self) -> f32 {
        self.ury - self.lly
    }

    /// Center X coordinate
    pub fn center_x(&self) -> f32 {
        (self.llx + self.urx) / 2.0
    }

    /// Center Y coordinate
    pub fn center_y(&self) -> f32 {
        (self.lly + self.ury) / 2.0
    }

    /// Area of the rectangle
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    /// Is this rectangle valid (positive dimensions)?
    pub fn is_valid(&self) -> bool {
        self.width() > 0.0 && self.height() > 0.0
    }

    /// Expand rectangle by given amount on all sides
    pub fn expand(&self, amount: f32) -> Self {
        Self {
            llx: self.llx - amount,
            lly: self.lly - amount,
            urx: self.urx + amount,
            ury: self.ury + amount,
        }
    }

    /// Expand rectangle by different amounts on each side
    pub fn expand_sides(&self, left: f32, bottom: f32, right: f32, top: f32) -> Self {
        Self {
            llx: self.llx - left,
            lly: self.lly - bottom,
            urx: self.urx + right,
            ury: self.ury + top,
        }
    }

    /// Shrink rectangle by given amount on all sides
    pub fn shrink(&self, amount: f32) -> Self {
        self.expand(-amount)
    }

    /// Check if this rectangle contains another
    pub fn contains(&self, other: &Rectangle) -> bool {
        self.llx <= other.llx
            && self.lly <= other.lly
            && self.urx >= other.urx
            && self.ury >= other.ury
    }

    /// Get intersection with another rectangle
    pub fn intersect(&self, other: &Rectangle) -> Option<Rectangle> {
        let llx = self.llx.max(other.llx);
        let lly = self.lly.max(other.lly);
        let urx = self.urx.min(other.urx);
        let ury = self.ury.min(other.ury);

        if llx < urx && lly < ury {
            Some(Rectangle { llx, lly, urx, ury })
        } else {
            None
        }
    }

    /// Get union (bounding box) with another rectangle
    pub fn union(&self, other: &Rectangle) -> Rectangle {
        Rectangle {
            llx: self.llx.min(other.llx),
            lly: self.lly.min(other.lly),
            urx: self.urx.max(other.urx),
            ury: self.ury.max(other.ury),
        }
    }

    /// Format as PDF array string
    pub fn to_pdf_array(&self) -> String {
        format!("[{} {} {} {}]", self.llx, self.lly, self.urx, self.ury)
    }

    /// Parse from PDF array string
    pub fn from_pdf_array(s: &str) -> Result<Self> {
        let s = s.trim().trim_start_matches('[').trim_end_matches(']');
        let parts: Vec<&str> = s.split_whitespace().collect();

        if parts.len() != 4 {
            return Err(EnhancedError::InvalidParameter(
                "Rectangle requires 4 values".to_string(),
            ));
        }

        let llx: f32 = parts[0]
            .parse()
            .map_err(|_| EnhancedError::InvalidParameter("Invalid llx value".to_string()))?;
        let lly: f32 = parts[1]
            .parse()
            .map_err(|_| EnhancedError::InvalidParameter("Invalid lly value".to_string()))?;
        let urx: f32 = parts[2]
            .parse()
            .map_err(|_| EnhancedError::InvalidParameter("Invalid urx value".to_string()))?;
        let ury: f32 = parts[3]
            .parse()
            .map_err(|_| EnhancedError::InvalidParameter("Invalid ury value".to_string()))?;

        Ok(Rectangle { llx, lly, urx, ury })
    }

    /// Convert dimensions to specified unit
    pub fn to_unit(&self, unit: Unit) -> (f32, f32, f32, f32) {
        (
            unit.from_points(self.llx),
            unit.from_points(self.lly),
            unit.from_points(self.urx),
            unit.from_points(self.ury),
        )
    }
}

impl Default for Rectangle {
    fn default() -> Self {
        // Default to US Letter size
        Rectangle::from_page_size(PageSize::Letter)
    }
}

// ============================================================================
// Standard Page Sizes
// ============================================================================

/// Standard page sizes for print production
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageSize {
    // ISO A Series
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    A7,
    A8,

    // ISO B Series
    B0,
    B1,
    B2,
    B3,
    B4,
    B5,
    B6,

    // ISO C Series (envelopes)
    C4,
    C5,
    C6,

    // North American
    Letter,
    Legal,
    Tabloid,
    Ledger,
    Executive,

    // Newspaper/Magazine
    Broadsheet,
    Berliner,
    Compact,

    // Custom
    Custom(f32, f32),
}

impl PageSize {
    /// Get dimensions in points (width, height)
    pub fn dimensions(&self) -> (f32, f32) {
        match self {
            // A Series (width x height in points)
            PageSize::A0 => (2383.94, 3370.39),
            PageSize::A1 => (1683.78, 2383.94),
            PageSize::A2 => (1190.55, 1683.78),
            PageSize::A3 => (841.89, 1190.55),
            PageSize::A4 => (595.28, 841.89),
            PageSize::A5 => (419.53, 595.28),
            PageSize::A6 => (297.64, 419.53),
            PageSize::A7 => (209.76, 297.64),
            PageSize::A8 => (147.40, 209.76),

            // B Series
            PageSize::B0 => (2834.65, 4008.19),
            PageSize::B1 => (2004.09, 2834.65),
            PageSize::B2 => (1417.32, 2004.09),
            PageSize::B3 => (1000.63, 1417.32),
            PageSize::B4 => (708.66, 1000.63),
            PageSize::B5 => (498.90, 708.66),
            PageSize::B6 => (354.33, 498.90),

            // C Series (envelopes)
            PageSize::C4 => (649.13, 918.43),
            PageSize::C5 => (459.21, 649.13),
            PageSize::C6 => (323.15, 459.21),

            // North American
            PageSize::Letter => (612.0, 792.0),
            PageSize::Legal => (612.0, 1008.0),
            PageSize::Tabloid => (792.0, 1224.0),
            PageSize::Ledger => (1224.0, 792.0),
            PageSize::Executive => (522.0, 756.0),

            // Newspaper
            PageSize::Broadsheet => (1296.0, 1728.0),
            PageSize::Berliner => (906.0, 1278.0),
            PageSize::Compact => (720.0, 1080.0),

            // Custom
            PageSize::Custom(w, h) => (*w, *h),
        }
    }

    /// Get landscape orientation
    pub fn landscape(&self) -> (f32, f32) {
        let (w, h) = self.dimensions();
        if w > h { (w, h) } else { (h, w) }
    }

    /// Get portrait orientation
    pub fn portrait(&self) -> (f32, f32) {
        let (w, h) = self.dimensions();
        if h > w { (w, h) } else { (h, w) }
    }

    /// Create from dimensions in specified unit
    pub fn from_dimensions(width: f32, height: f32, unit: Unit) -> Self {
        let w = unit.to_points(width);
        let h = unit.to_points(height);
        PageSize::Custom(w, h)
    }

    /// Parse from string (e.g., "A4", "Letter", "210x297mm")
    pub fn from_str(s: &str) -> Result<Self> {
        let s = s.trim().to_uppercase();

        // Try standard sizes
        match s.as_str() {
            "A0" => return Ok(PageSize::A0),
            "A1" => return Ok(PageSize::A1),
            "A2" => return Ok(PageSize::A2),
            "A3" => return Ok(PageSize::A3),
            "A4" => return Ok(PageSize::A4),
            "A5" => return Ok(PageSize::A5),
            "A6" => return Ok(PageSize::A6),
            "A7" => return Ok(PageSize::A7),
            "A8" => return Ok(PageSize::A8),
            "B0" => return Ok(PageSize::B0),
            "B1" => return Ok(PageSize::B1),
            "B2" => return Ok(PageSize::B2),
            "B3" => return Ok(PageSize::B3),
            "B4" => return Ok(PageSize::B4),
            "B5" => return Ok(PageSize::B5),
            "B6" => return Ok(PageSize::B6),
            "C4" => return Ok(PageSize::C4),
            "C5" => return Ok(PageSize::C5),
            "C6" => return Ok(PageSize::C6),
            "LETTER" => return Ok(PageSize::Letter),
            "LEGAL" => return Ok(PageSize::Legal),
            "TABLOID" => return Ok(PageSize::Tabloid),
            "LEDGER" => return Ok(PageSize::Ledger),
            "EXECUTIVE" => return Ok(PageSize::Executive),
            _ => {}
        }

        // Try parsing custom size (e.g., "210x297mm" or "8.5x11in")
        if let Some(idx) = s.find('X') {
            let (w_str, rest) = s.split_at(idx);
            let rest = &rest[1..]; // Skip 'X'

            // Determine unit
            let unit = if rest.ends_with("MM") {
                Unit::Mm
            } else if rest.ends_with("CM") {
                Unit::Cm
            } else if rest.ends_with("IN") {
                Unit::Inch
            } else if rest.ends_with("PT") {
                Unit::Point
            } else {
                Unit::Point
            };

            // Remove unit suffix
            let h_str = rest
                .trim_end_matches("MM")
                .trim_end_matches("CM")
                .trim_end_matches("IN")
                .trim_end_matches("PT");

            let width: f32 = w_str.parse().map_err(|_| {
                EnhancedError::InvalidParameter(format!("Invalid width: {}", w_str))
            })?;
            let height: f32 = h_str.parse().map_err(|_| {
                EnhancedError::InvalidParameter(format!("Invalid height: {}", h_str))
            })?;

            return Ok(PageSize::from_dimensions(width, height, unit));
        }

        Err(EnhancedError::InvalidParameter(format!(
            "Unknown page size: {}",
            s
        )))
    }
}

// ============================================================================
// Page Boxes Structure
// ============================================================================

/// Collection of page boxes for a single page
#[derive(Debug, Clone, Default)]
pub struct PageBoxes {
    /// MediaBox is required
    pub media_box: Option<Rectangle>,
    /// CropBox (defaults to MediaBox if not set)
    pub crop_box: Option<Rectangle>,
    /// BleedBox (defaults to CropBox if not set)
    pub bleed_box: Option<Rectangle>,
    /// TrimBox (defaults to CropBox if not set)
    pub trim_box: Option<Rectangle>,
    /// ArtBox (defaults to CropBox if not set)
    pub art_box: Option<Rectangle>,
}

impl PageBoxes {
    /// Create empty page boxes
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with MediaBox
    pub fn with_media_box(media_box: Rectangle) -> Self {
        Self {
            media_box: Some(media_box),
            ..Default::default()
        }
    }

    /// Get a specific box, falling back to parent boxes if not set
    pub fn get(&self, box_type: BoxType) -> Option<Rectangle> {
        match box_type {
            BoxType::MediaBox => self.media_box,
            BoxType::CropBox => self.crop_box.or(self.media_box),
            BoxType::BleedBox => self.bleed_box.or(self.crop_box).or(self.media_box),
            BoxType::TrimBox => self.trim_box.or(self.crop_box).or(self.media_box),
            BoxType::ArtBox => self.art_box.or(self.crop_box).or(self.media_box),
        }
    }

    /// Get the effective (resolved) box value
    pub fn effective(&self, box_type: BoxType) -> Rectangle {
        self.get(box_type).unwrap_or_default()
    }

    /// Set a specific box
    pub fn set(&mut self, box_type: BoxType, rect: Rectangle) {
        match box_type {
            BoxType::MediaBox => self.media_box = Some(rect),
            BoxType::CropBox => self.crop_box = Some(rect),
            BoxType::BleedBox => self.bleed_box = Some(rect),
            BoxType::TrimBox => self.trim_box = Some(rect),
            BoxType::ArtBox => self.art_box = Some(rect),
        }
    }

    /// Clear a specific box (will fall back to parent)
    pub fn clear(&mut self, box_type: BoxType) {
        match box_type {
            BoxType::MediaBox => self.media_box = None,
            BoxType::CropBox => self.crop_box = None,
            BoxType::BleedBox => self.bleed_box = None,
            BoxType::TrimBox => self.trim_box = None,
            BoxType::ArtBox => self.art_box = None,
        }
    }

    /// Add bleed to create BleedBox from TrimBox
    pub fn add_bleed(&mut self, bleed: f32) {
        if let Some(trim) = self.get(BoxType::TrimBox) {
            self.bleed_box = Some(trim.expand(bleed));
        }
    }

    /// Add bleed with different amounts per side
    pub fn add_bleed_sides(&mut self, left: f32, bottom: f32, right: f32, top: f32) {
        if let Some(trim) = self.get(BoxType::TrimBox) {
            self.bleed_box = Some(trim.expand_sides(left, bottom, right, top));
        }
    }

    /// Validate box hierarchy (larger boxes should contain smaller ones)
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();

        // MediaBox is required
        let media = match self.media_box {
            Some(m) => m,
            None => {
                issues.push("MediaBox is required but not set".to_string());
                return issues;
            }
        };

        // Check that other boxes are within MediaBox
        for box_type in [
            BoxType::CropBox,
            BoxType::BleedBox,
            BoxType::TrimBox,
            BoxType::ArtBox,
        ] {
            if let Some(rect) = self.get(box_type) {
                if !media.contains(&rect) {
                    issues.push(format!("{} extends beyond MediaBox", box_type.pdf_key()));
                }
            }
        }

        // Typically TrimBox should be within BleedBox
        if let (Some(bleed), Some(trim)) = (self.bleed_box, self.trim_box) {
            if !bleed.contains(&trim) {
                issues.push("TrimBox should be within BleedBox".to_string());
            }
        }

        // ArtBox should be within TrimBox
        if let (Some(trim), Some(art)) = (self.get(BoxType::TrimBox), self.art_box) {
            if !trim.contains(&art) {
                issues.push("ArtBox should be within TrimBox".to_string());
            }
        }

        issues
    }

    /// Get all explicitly set boxes
    pub fn all_set(&self) -> HashMap<BoxType, Rectangle> {
        let mut boxes = HashMap::new();
        if let Some(r) = self.media_box {
            boxes.insert(BoxType::MediaBox, r);
        }
        if let Some(r) = self.crop_box {
            boxes.insert(BoxType::CropBox, r);
        }
        if let Some(r) = self.bleed_box {
            boxes.insert(BoxType::BleedBox, r);
        }
        if let Some(r) = self.trim_box {
            boxes.insert(BoxType::TrimBox, r);
        }
        if let Some(r) = self.art_box {
            boxes.insert(BoxType::ArtBox, r);
        }
        boxes
    }
}

// ============================================================================
// Page Box Manager
// ============================================================================

/// Manager for manipulating page boxes in PDFs
pub struct PageBoxManager {
    /// PDF file data
    pdf_data: Vec<u8>,
    /// Path to the PDF file
    file_path: String,
    /// Page boxes for each page
    page_boxes: Vec<PageBoxes>,
    /// Whether changes have been made
    modified: bool,
}

impl PageBoxManager {
    /// Open a PDF file for box management
    pub fn new(path: &str) -> Result<Self> {
        if !Path::new(path).exists() {
            return Err(EnhancedError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", path),
            )));
        }

        let pdf_data = fs::read(path)?;
        let page_boxes = Self::parse_page_boxes(&pdf_data)?;

        Ok(Self {
            pdf_data,
            file_path: path.to_string(),
            page_boxes,
            modified: false,
        })
    }

    /// Create from PDF data in memory
    pub fn from_data(data: Vec<u8>) -> Result<Self> {
        let page_boxes = Self::parse_page_boxes(&data)?;

        Ok(Self {
            pdf_data: data,
            file_path: String::new(),
            page_boxes,
            modified: false,
        })
    }

    /// Get number of pages
    pub fn page_count(&self) -> usize {
        self.page_boxes.len()
    }

    /// Get boxes for a specific page (0-indexed)
    pub fn get_page_boxes(&self, page: usize) -> Option<&PageBoxes> {
        self.page_boxes.get(page)
    }

    /// Get boxes for all pages
    pub fn get_all_boxes(&self) -> &[PageBoxes] {
        &self.page_boxes
    }

    /// Set a specific box for a page
    pub fn set_box(&mut self, page: usize, box_type: BoxType, rect: Rectangle) -> Result<()> {
        if page >= self.page_boxes.len() {
            return Err(EnhancedError::InvalidParameter(format!(
                "Page {} out of range (0-{})",
                page,
                self.page_boxes.len() - 1
            )));
        }

        self.page_boxes[page].set(box_type, rect);
        self.modified = true;
        Ok(())
    }

    /// Set a box for all pages
    pub fn set_box_all(&mut self, box_type: BoxType, rect: Rectangle) {
        for boxes in &mut self.page_boxes {
            boxes.set(box_type, rect);
        }
        self.modified = true;
    }

    /// Set box from coordinates with unit conversion
    pub fn set_box_with_unit(
        &mut self,
        page: usize,
        box_type: BoxType,
        llx: f32,
        lly: f32,
        urx: f32,
        ury: f32,
        unit: Unit,
    ) -> Result<()> {
        let rect = Rectangle::new(
            unit.to_points(llx),
            unit.to_points(lly),
            unit.to_points(urx),
            unit.to_points(ury),
        );
        self.set_box(page, box_type, rect)
    }

    /// Add bleed to all pages (uniform on all sides)
    pub fn add_bleed(&mut self, bleed: f32, unit: Unit) {
        let bleed_pts = unit.to_points(bleed);
        for boxes in &mut self.page_boxes {
            boxes.add_bleed(bleed_pts);
        }
        self.modified = true;
    }

    /// Add bleed with different amounts per side
    pub fn add_bleed_sides(&mut self, left: f32, bottom: f32, right: f32, top: f32, unit: Unit) {
        let l = unit.to_points(left);
        let b = unit.to_points(bottom);
        let r = unit.to_points(right);
        let t = unit.to_points(top);

        for boxes in &mut self.page_boxes {
            boxes.add_bleed_sides(l, b, r, t);
        }
        self.modified = true;
    }

    /// Set TrimBox from page size
    pub fn set_trim_box_from_size(&mut self, page: usize, size: PageSize) -> Result<()> {
        let (w, h) = size.dimensions();
        let rect = Rectangle::from_size(0.0, 0.0, w, h);
        self.set_box(page, BoxType::TrimBox, rect)
    }

    /// Set MediaBox from page size for all pages
    pub fn set_media_box_all(&mut self, size: PageSize) {
        let rect = Rectangle::from_page_size(size);
        self.set_box_all(BoxType::MediaBox, rect);
    }

    /// Crop pages to CropBox (removes content outside)
    pub fn crop_to_box(&mut self, box_type: BoxType) {
        // Set CropBox to match the specified box for all pages
        for boxes in &mut self.page_boxes {
            if let Some(rect) = boxes.get(box_type) {
                boxes.set(BoxType::CropBox, rect);
            }
        }
        self.modified = true;
    }

    /// Remove all boxes except MediaBox (reset to defaults)
    pub fn reset_boxes(&mut self) {
        for boxes in &mut self.page_boxes {
            boxes.crop_box = None;
            boxes.bleed_box = None;
            boxes.trim_box = None;
            boxes.art_box = None;
        }
        self.modified = true;
    }

    /// Validate all page boxes
    pub fn validate(&self) -> Vec<(usize, Vec<String>)> {
        self.page_boxes
            .iter()
            .enumerate()
            .map(|(i, boxes)| (i, boxes.validate()))
            .filter(|(_, issues)| !issues.is_empty())
            .collect()
    }

    /// Check if modifications have been made
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Save changes to the PDF
    pub fn save(&self, output_path: &str) -> Result<()> {
        let modified_pdf = self.generate_pdf()?;
        fs::write(output_path, modified_pdf)?;
        Ok(())
    }

    /// Save changes back to the original file
    pub fn save_in_place(&self) -> Result<()> {
        if self.file_path.is_empty() {
            return Err(EnhancedError::InvalidParameter(
                "No file path for in-place save".to_string(),
            ));
        }
        self.save(&self.file_path)
    }

    /// Get modified PDF data
    pub fn get_data(&self) -> Result<Vec<u8>> {
        self.generate_pdf()
    }

    // ========================================================================
    // Internal Methods
    // ========================================================================

    /// Parse page boxes from PDF data
    fn parse_page_boxes(pdf_data: &[u8]) -> Result<Vec<PageBoxes>> {
        let content = String::from_utf8_lossy(pdf_data);
        let mut page_boxes = Vec::new();

        // Find all page objects
        let page_pattern = "/Type /Page";
        let mut search_pos = 0;

        while let Some(pos) = content[search_pos..].find(page_pattern) {
            let abs_pos = search_pos + pos;
            let mut boxes = PageBoxes::new();

            // Find the object boundaries
            let obj_start = content[..abs_pos].rfind("obj").unwrap_or(0);
            let obj_end = content[abs_pos..]
                .find("endobj")
                .unwrap_or(content.len() - abs_pos)
                + abs_pos;
            let obj_content = &content[obj_start..obj_end];

            // Parse each box type
            for box_type in BoxType::all() {
                let key = format!("/{}", box_type.pdf_key());
                if let Some(box_pos) = obj_content.find(&key) {
                    if let Some(array_start) = obj_content[box_pos..].find('[') {
                        if let Some(array_end) = obj_content[box_pos + array_start..].find(']') {
                            let array_str = &obj_content
                                [box_pos + array_start..box_pos + array_start + array_end + 1];
                            if let Ok(rect) = Rectangle::from_pdf_array(array_str) {
                                boxes.set(*box_type, rect);
                            }
                        }
                    }
                }
            }

            // If no MediaBox found, try to get from parent
            if boxes.media_box.is_none() {
                // Default to Letter size if nothing found
                boxes.media_box = Some(Rectangle::from_page_size(PageSize::Letter));
            }

            page_boxes.push(boxes);
            search_pos = abs_pos + page_pattern.len();
        }

        if page_boxes.is_empty() {
            // Return at least one default page
            page_boxes.push(PageBoxes::with_media_box(Rectangle::from_page_size(
                PageSize::Letter,
            )));
        }

        Ok(page_boxes)
    }

    /// Generate modified PDF data
    fn generate_pdf(&self) -> Result<Vec<u8>> {
        let mut pdf_data = self.pdf_data.clone();
        let content = String::from_utf8_lossy(&pdf_data).to_string();

        // Find and update each page object
        let page_pattern = "/Type /Page";
        let mut search_pos = 0;
        let mut page_idx = 0;

        while let Some(pos) = content[search_pos..].find(page_pattern) {
            let abs_pos = search_pos + pos;

            if page_idx < self.page_boxes.len() {
                let boxes = &self.page_boxes[page_idx];

                // Find the dictionary boundaries
                let dict_start = content[..abs_pos].rfind("<<").unwrap_or(0);
                let dict_end_relative = content[abs_pos..].find(">>").unwrap_or(0);
                let dict_end = abs_pos + dict_end_relative;

                // Build new box entries
                let mut new_entries = String::new();
                for (box_type, rect) in boxes.all_set() {
                    new_entries.push_str(&format!(
                        "/{} {}\n",
                        box_type.pdf_key(),
                        rect.to_pdf_array()
                    ));
                }

                // This is simplified - a full implementation would properly
                // update the PDF structure. For now, we'll note that changes
                // need to be applied via incremental update.
                let _ = (dict_start, dict_end, new_entries);
            }

            search_pos = abs_pos + page_pattern.len();
            page_idx += 1;
        }

        // For a complete implementation, we would:
        // 1. Parse the PDF properly
        // 2. Update each page dictionary with new box values
        // 3. Write an incremental update or rewrite the PDF
        //
        // For now, we return a modified version that updates boxes in-place
        // where possible.

        if self.modified {
            // Apply box changes using incremental update
            pdf_data = self.apply_box_changes_incremental(&pdf_data)?;
        }

        Ok(pdf_data)
    }

    /// Apply box changes using incremental update
    fn apply_box_changes_incremental(&self, pdf_data: &[u8]) -> Result<Vec<u8>> {
        let mut output = pdf_data.to_vec();
        let content = String::from_utf8_lossy(pdf_data);

        // Find page objects and update their box values
        let page_pattern = "/Type /Page";
        let mut modifications: Vec<(usize, usize, String)> = Vec::new();
        let mut search_pos = 0;
        let mut page_idx = 0;

        while let Some(pos) = content[search_pos..].find(page_pattern) {
            let abs_pos = search_pos + pos;

            if page_idx < self.page_boxes.len() {
                let boxes = &self.page_boxes[page_idx];

                // Find object boundaries
                let obj_start = content[..abs_pos]
                    .rfind("obj")
                    .map(|p| {
                        // Find the start of the object number
                        let before = &content[..p];
                        before.rfind('\n').map(|n| n + 1).unwrap_or(0)
                    })
                    .unwrap_or(0);

                let obj_end = content[abs_pos..]
                    .find("endobj")
                    .map(|p| abs_pos + p + 6)
                    .unwrap_or(content.len());

                // Build replacement dictionary content
                let obj_content = &content[obj_start..obj_end];
                let mut new_content = obj_content.to_string();

                // Update each box
                for (box_type, rect) in boxes.all_set() {
                    let key = format!("/{}", box_type.pdf_key());
                    let new_value = format!("{} {}", key, rect.to_pdf_array());

                    // Find and replace existing box definition
                    if let Some(key_pos) = new_content.find(&key) {
                        // Find the end of the array
                        if let Some(arr_start) = new_content[key_pos..].find('[') {
                            if let Some(arr_end) = new_content[key_pos + arr_start..].find(']') {
                                let replace_start = key_pos;
                                let replace_end = key_pos + arr_start + arr_end + 1;
                                new_content = format!(
                                    "{}{}{}",
                                    &new_content[..replace_start],
                                    new_value,
                                    &new_content[replace_end..]
                                );
                            }
                        }
                    } else {
                        // Insert new box definition after /Type /Page
                        if let Some(type_pos) = new_content.find("/Type /Page") {
                            let insert_pos = type_pos + "/Type /Page".len();
                            new_content = format!(
                                "{}\n  {}{}",
                                &new_content[..insert_pos],
                                new_value,
                                &new_content[insert_pos..]
                            );
                        }
                    }
                }

                if new_content != obj_content {
                    modifications.push((obj_start, obj_end, new_content));
                }
            }

            search_pos = abs_pos + page_pattern.len();
            page_idx += 1;
        }

        // Apply modifications in reverse order to preserve positions
        for (start, end, new_content) in modifications.into_iter().rev() {
            let before = &output[..start];
            let after = &output[end..];
            output = [before, new_content.as_bytes(), after].concat();
        }

        // Update xref and trailer (simplified)
        // A full implementation would properly rebuild the xref table

        Ok(output)
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Add bleed to a PDF file
pub fn add_bleed(input_path: &str, output_path: &str, bleed: f32, unit: Unit) -> Result<()> {
    let mut manager = PageBoxManager::new(input_path)?;
    manager.add_bleed(bleed, unit);
    manager.save(output_path)
}

/// Add bleed with different amounts per side
pub fn add_bleed_sides(
    input_path: &str,
    output_path: &str,
    left: f32,
    bottom: f32,
    right: f32,
    top: f32,
    unit: Unit,
) -> Result<()> {
    let mut manager = PageBoxManager::new(input_path)?;
    manager.add_bleed_sides(left, bottom, right, top, unit);
    manager.save(output_path)
}

/// Set a specific box for all pages
pub fn set_box(
    input_path: &str,
    output_path: &str,
    box_type: BoxType,
    llx: f32,
    lly: f32,
    urx: f32,
    ury: f32,
    unit: Unit,
) -> Result<()> {
    let mut manager = PageBoxManager::new(input_path)?;
    let rect = Rectangle::new(
        unit.to_points(llx),
        unit.to_points(lly),
        unit.to_points(urx),
        unit.to_points(ury),
    );
    manager.set_box_all(box_type, rect);
    manager.save(output_path)
}

/// Crop to TrimBox
pub fn crop_to_trim(input_path: &str, output_path: &str) -> Result<()> {
    let mut manager = PageBoxManager::new(input_path)?;
    manager.crop_to_box(BoxType::TrimBox);
    manager.save(output_path)
}

/// Get page boxes for a PDF file
pub fn get_page_boxes(pdf_path: &str) -> Result<Vec<PageBoxes>> {
    let manager = PageBoxManager::new(pdf_path)?;
    Ok(manager.page_boxes)
}

/// Validate page boxes in a PDF
pub fn validate_boxes(pdf_path: &str) -> Result<Vec<(usize, Vec<String>)>> {
    let manager = PageBoxManager::new(pdf_path)?;
    Ok(manager.validate())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_conversion() {
        // Points
        assert!((Unit::Point.to_points(72.0) - 72.0).abs() < 0.01);

        // Inches
        assert!((Unit::Inch.to_points(1.0) - 72.0).abs() < 0.01);
        assert!((Unit::Inch.from_points(72.0) - 1.0).abs() < 0.01);

        // Millimeters
        assert!((Unit::Mm.to_points(25.4) - 72.0).abs() < 0.01);
        assert!((Unit::Mm.from_points(72.0) - 25.4).abs() < 0.01);

        // Centimeters
        assert!((Unit::Cm.to_points(2.54) - 72.0).abs() < 0.01);
    }

    #[test]
    fn test_rectangle_operations() {
        let rect = Rectangle::new(0.0, 0.0, 100.0, 200.0);

        assert!((rect.width() - 100.0).abs() < 0.01);
        assert!((rect.height() - 200.0).abs() < 0.01);
        assert!((rect.center_x() - 50.0).abs() < 0.01);
        assert!((rect.center_y() - 100.0).abs() < 0.01);
        assert!(rect.is_valid());

        let expanded = rect.expand(10.0);
        assert!((expanded.llx - (-10.0)).abs() < 0.01);
        assert!((expanded.urx - 110.0).abs() < 0.01);
    }

    #[test]
    fn test_rectangle_pdf_array() {
        let rect = Rectangle::new(0.0, 0.0, 612.0, 792.0);
        let array = rect.to_pdf_array();
        assert_eq!(array, "[0 0 612 792]");

        let parsed = Rectangle::from_pdf_array(&array).unwrap();
        assert!((parsed.llx - rect.llx).abs() < 0.01);
        assert!((parsed.urx - rect.urx).abs() < 0.01);
    }

    #[test]
    fn test_rectangle_contains() {
        let outer = Rectangle::new(0.0, 0.0, 100.0, 100.0);
        let inner = Rectangle::new(10.0, 10.0, 90.0, 90.0);
        let overlapping = Rectangle::new(50.0, 50.0, 150.0, 150.0);

        assert!(outer.contains(&inner));
        assert!(!inner.contains(&outer));
        assert!(!outer.contains(&overlapping));
    }

    #[test]
    fn test_rectangle_intersect() {
        let a = Rectangle::new(0.0, 0.0, 100.0, 100.0);
        let b = Rectangle::new(50.0, 50.0, 150.0, 150.0);
        let c = Rectangle::new(200.0, 200.0, 300.0, 300.0);

        let intersection = a.intersect(&b).unwrap();
        assert!((intersection.llx - 50.0).abs() < 0.01);
        assert!((intersection.urx - 100.0).abs() < 0.01);

        assert!(a.intersect(&c).is_none());
    }

    #[test]
    fn test_page_size_dimensions() {
        let (w, h) = PageSize::A4.dimensions();
        assert!((w - 595.28).abs() < 0.1);
        assert!((h - 841.89).abs() < 0.1);

        let (w, h) = PageSize::Letter.dimensions();
        assert!((w - 612.0).abs() < 0.1);
        assert!((h - 792.0).abs() < 0.1);
    }

    #[test]
    fn test_page_size_from_str() {
        assert_eq!(PageSize::from_str("A4").unwrap(), PageSize::A4);
        assert_eq!(PageSize::from_str("letter").unwrap(), PageSize::Letter);

        let custom = PageSize::from_str("210x297mm").unwrap();
        if let PageSize::Custom(w, h) = custom {
            assert!((w - 595.28).abs() < 1.0);
            assert!((h - 841.89).abs() < 1.0);
        } else {
            panic!("Expected custom size");
        }
    }

    #[test]
    fn test_page_boxes() {
        let media = Rectangle::new(0.0, 0.0, 612.0, 792.0);
        let mut boxes = PageBoxes::with_media_box(media);

        // Test fallback
        assert_eq!(boxes.get(BoxType::CropBox), Some(media));
        assert_eq!(boxes.get(BoxType::TrimBox), Some(media));

        // Set TrimBox
        let trim = Rectangle::new(10.0, 10.0, 602.0, 782.0);
        boxes.set(BoxType::TrimBox, trim);
        assert_eq!(boxes.get(BoxType::TrimBox), Some(trim));

        // Add bleed
        boxes.add_bleed(9.0);
        let bleed = boxes.get(BoxType::BleedBox).unwrap();
        assert!((bleed.llx - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_page_boxes_validation() {
        let media = Rectangle::new(0.0, 0.0, 612.0, 792.0);
        let mut boxes = PageBoxes::with_media_box(media);

        // Valid hierarchy
        let trim = Rectangle::new(10.0, 10.0, 602.0, 782.0);
        boxes.set(BoxType::TrimBox, trim);
        boxes.add_bleed(5.0);
        let issues = boxes.validate();
        assert!(issues.is_empty());

        // Invalid - TrimBox outside MediaBox
        let bad_trim = Rectangle::new(-50.0, -50.0, 700.0, 900.0);
        boxes.set(BoxType::TrimBox, bad_trim);
        let issues = boxes.validate();
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_box_type_properties() {
        assert_eq!(BoxType::MediaBox.pdf_key(), "MediaBox");
        assert_eq!(BoxType::TrimBox.pdf_key(), "TrimBox");
        assert_eq!(BoxType::all().len(), 5);
    }
}
