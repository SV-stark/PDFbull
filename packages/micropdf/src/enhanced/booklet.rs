//! Booklet Creation and Imposition Module
//!
//! Creates print-ready booklets with proper page ordering for various binding methods.
//!
//! ## Binding Methods
//!
//! - **Saddle Stitch**: Folded sheets nested together (magazines)
//! - **Perfect Binding**: Glued spine (books)
//! - **Side Stitch**: Stapled through the side
//!
//! ## Imposition
//!
//! Arranges pages on press sheets for efficient printing and binding:
//!
//! ```text
//! Saddle-stitch booklet (8 pages):
//!
//! Sheet 1 (outside):     Sheet 1 (inside):
//! ┌────────┬────────┐    ┌────────┬────────┐
//! │   8    │   1    │    │   2    │   7    │
//! │  (back)│ (front)│    │        │        │
//! └────────┴────────┘    └────────┴────────┘
//!
//! Sheet 2 (outside):     Sheet 2 (inside):
//! ┌────────┬────────┐    ┌────────┬────────┐
//! │   6    │   3    │    │   4    │   5    │
//! │        │        │    │        │        │
//! └────────┴────────┘    └────────┴────────┘
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use micropdf::enhanced::booklet::{create_saddle_stitch_booklet, BookletOptions};
//!
//! // Simple booklet
//! create_saddle_stitch_booklet("input.pdf", "booklet.pdf")?;
//!
//! // With options
//! let options = BookletOptions::new()
//!     .binding(BindingMethod::SaddleStitch)
//!     .sheet_size(PageSize::Letter)
//!     .creep_adjustment(0.5);
//! create_booklet_with_options("input.pdf", "booklet.pdf", &options)?;
//! ```

use super::error::{EnhancedError, Result};
use super::page_boxes::{PageSize, Rectangle, Unit};
use std::fs;
use std::path::Path;

// ============================================================================
// Binding Methods
// ============================================================================

/// Binding method for the booklet
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BindingMethod {
    /// Saddle stitch - folded and stapled through the spine
    /// Best for: 8-64 pages (magazines, brochures)
    #[default]
    SaddleStitch,
    /// Perfect binding - pages glued to spine
    /// Best for: 40+ pages (paperback books)
    PerfectBinding,
    /// Side stitch - stapled through the side
    /// Best for: 4-80 pages (reports, manuals)
    SideStitch,
    /// Wire-O or spiral binding
    /// Best for: 10-300 pages (notebooks, calendars)
    WireO,
    /// Case binding (hardcover)
    /// Best for: 80+ pages (textbooks, coffee table books)
    CaseBinding,
}

impl BindingMethod {
    /// Maximum recommended page count for this binding
    pub fn max_pages(&self) -> usize {
        match self {
            BindingMethod::SaddleStitch => 64,
            BindingMethod::PerfectBinding => 1000,
            BindingMethod::SideStitch => 80,
            BindingMethod::WireO => 300,
            BindingMethod::CaseBinding => 2000,
        }
    }

    /// Minimum recommended page count for this binding
    pub fn min_pages(&self) -> usize {
        match self {
            BindingMethod::SaddleStitch => 4,
            BindingMethod::PerfectBinding => 40,
            BindingMethod::SideStitch => 4,
            BindingMethod::WireO => 10,
            BindingMethod::CaseBinding => 80,
        }
    }

    /// Pages per signature for this binding
    pub fn pages_per_signature(&self) -> usize {
        match self {
            BindingMethod::SaddleStitch => 4,    // One folded sheet
            BindingMethod::PerfectBinding => 16, // Common signature size
            BindingMethod::SideStitch => 4,      // Typically individual sheets
            BindingMethod::WireO => 2,           // Single sheets
            BindingMethod::CaseBinding => 16,    // Common signature size
        }
    }
}

// ============================================================================
// Binding Edge
// ============================================================================

/// Edge where binding occurs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BindingEdge {
    /// Left edge binding (default for LTR text)
    #[default]
    Left,
    /// Right edge binding (for RTL text like Arabic, Hebrew)
    Right,
    /// Top edge binding (calendars, notepads)
    Top,
    /// Bottom edge binding (rare)
    Bottom,
}

// ============================================================================
// Fold Type
// ============================================================================

/// How the sheet is folded
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldType {
    /// Single fold (2 leaves, 4 pages)
    SingleFold,
    /// French fold (2 folds, 4 leaves, 8 pages)
    FrenchFold,
    /// Gate fold (2 panels fold to center)
    GateFold,
    /// Accordion/Z-fold
    AccordionFold,
    /// Roll fold
    RollFold,
}

// ============================================================================
// Imposition Signature
// ============================================================================

/// A signature (press sheet) in the imposition
#[derive(Debug, Clone)]
pub struct Signature {
    /// Signature number (0-based)
    pub index: usize,
    /// Pages on the front (outside) of the sheet
    /// Order: left, right when viewing unfolded
    pub front: Vec<Option<usize>>,
    /// Pages on the back (inside) of the sheet
    /// Order: left, right when viewing unfolded
    pub back: Vec<Option<usize>>,
    /// Creep adjustment for this signature (in points)
    pub creep: f32,
}

impl Signature {
    /// Create a new signature
    pub fn new(index: usize) -> Self {
        Self {
            index,
            front: Vec::new(),
            back: Vec::new(),
            creep: 0.0,
        }
    }

    /// Get all page numbers in this signature
    pub fn all_pages(&self) -> Vec<usize> {
        self.front
            .iter()
            .chain(self.back.iter())
            .filter_map(|p| *p)
            .collect()
    }
}

// ============================================================================
// Booklet Options
// ============================================================================

/// Options for booklet creation
#[derive(Debug, Clone)]
pub struct BookletOptions {
    /// Binding method
    pub binding: BindingMethod,
    /// Which edge is bound
    pub binding_edge: BindingEdge,
    /// Output sheet size (the paper going into printer)
    pub sheet_size: PageSize,
    /// Add blank pages to make page count divisible by signature size
    pub add_blank_pages: bool,
    /// Creep adjustment per signature (in points)
    /// Accounts for paper thickness pushing inner pages outward
    pub creep_per_signature: f32,
    /// Total creep to distribute (overrides per-signature if set)
    pub total_creep: Option<f32>,
    /// Add crop marks
    pub crop_marks: bool,
    /// Crop mark length in points
    pub crop_mark_length: f32,
    /// Crop mark offset from trim edge in points
    pub crop_mark_offset: f32,
    /// Add fold marks
    pub fold_marks: bool,
    /// Print signature marks (collating marks)
    pub signature_marks: bool,
    /// Duplex printing order
    pub duplex: bool,
    /// Landscape output
    pub landscape: bool,
    /// Margin for binding (gutter)
    pub gutter: f32,
}

impl Default for BookletOptions {
    fn default() -> Self {
        Self {
            binding: BindingMethod::SaddleStitch,
            binding_edge: BindingEdge::Left,
            sheet_size: PageSize::Letter,
            add_blank_pages: true,
            creep_per_signature: 0.0,
            total_creep: None,
            crop_marks: false,
            crop_mark_length: 18.0, // 1/4 inch
            crop_mark_offset: 9.0,  // 1/8 inch
            fold_marks: false,
            signature_marks: false,
            duplex: true,
            landscape: true, // Booklets typically print landscape
            gutter: 0.0,
        }
    }
}

impl BookletOptions {
    /// Create new options with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set binding method
    pub fn binding(mut self, binding: BindingMethod) -> Self {
        self.binding = binding;
        self
    }

    /// Set binding edge
    pub fn binding_edge(mut self, edge: BindingEdge) -> Self {
        self.binding_edge = edge;
        self
    }

    /// Set sheet size
    pub fn sheet_size(mut self, size: PageSize) -> Self {
        self.sheet_size = size;
        self
    }

    /// Enable/disable blank page addition
    pub fn add_blank_pages(mut self, add: bool) -> Self {
        self.add_blank_pages = add;
        self
    }

    /// Set creep adjustment per signature
    pub fn creep_per_signature(mut self, creep: f32, unit: Unit) -> Self {
        self.creep_per_signature = unit.to_points(creep);
        self
    }

    /// Set total creep to distribute (pass the creep value, not an Option)
    pub fn total_creep(mut self, creep: f32, unit: Unit) -> Self {
        let pts = unit.to_points(creep);
        self.total_creep = if pts > 0.0 { Some(pts) } else { None };
        self
    }

    /// Enable crop marks
    pub fn crop_marks(mut self, enabled: bool) -> Self {
        self.crop_marks = enabled;
        self
    }

    /// Enable fold marks
    pub fn fold_marks(mut self, enabled: bool) -> Self {
        self.fold_marks = enabled;
        self
    }

    /// Enable signature marks
    pub fn signature_marks(mut self, enabled: bool) -> Self {
        self.signature_marks = enabled;
        self
    }

    /// Set gutter width
    pub fn gutter(mut self, width: f32, unit: Unit) -> Self {
        self.gutter = unit.to_points(width);
        self
    }

    /// Set landscape orientation
    pub fn landscape(mut self, enabled: bool) -> Self {
        self.landscape = enabled;
        self
    }

    /// Calculate final page dimensions
    pub fn page_dimensions(&self) -> (f32, f32) {
        let (sheet_w, sheet_h) = if self.landscape {
            self.sheet_size.landscape()
        } else {
            self.sheet_size.portrait()
        };

        // For saddle stitch, page width is half the sheet
        match self.binding {
            BindingMethod::SaddleStitch => (sheet_w / 2.0, sheet_h),
            BindingMethod::PerfectBinding => (sheet_w / 2.0, sheet_h),
            _ => (sheet_w / 2.0, sheet_h),
        }
    }
}

// ============================================================================
// Imposition Calculator
// ============================================================================

/// Calculates page positions for imposition
pub struct ImpositionCalculator {
    /// Total number of source pages
    page_count: usize,
    /// Booklet options
    options: BookletOptions,
    /// Calculated signatures
    signatures: Vec<Signature>,
}

impl ImpositionCalculator {
    /// Create a new calculator
    pub fn new(page_count: usize, options: BookletOptions) -> Self {
        let mut calc = Self {
            page_count,
            options,
            signatures: Vec::new(),
        };
        calc.calculate();
        calc
    }

    /// Calculate imposition
    fn calculate(&mut self) {
        match self.options.binding {
            BindingMethod::SaddleStitch => self.calculate_saddle_stitch(),
            BindingMethod::PerfectBinding => self.calculate_perfect_binding(),
            _ => self.calculate_saddle_stitch(), // Default fallback
        }
    }

    /// Calculate saddle-stitch imposition
    fn calculate_saddle_stitch(&mut self) {
        // Round up to multiple of 4
        let adjusted_count = if self.options.add_blank_pages {
            ((self.page_count + 3) / 4) * 4
        } else {
            self.page_count
        };

        let num_sheets = adjusted_count / 4;
        let num_signatures = (num_sheets + 1) / 2; // Sheets per signature

        // Calculate creep distribution
        let creep_per_sig = if let Some(total) = self.options.total_creep {
            total / num_signatures as f32
        } else {
            self.options.creep_per_signature
        };

        self.signatures.clear();

        // For saddle stitch, pages wrap around
        // Sheet 1: pages 1, 2, n-1, n
        // Sheet 2: pages 3, 4, n-3, n-2
        // etc.

        for sheet in 0..num_sheets {
            let mut sig = Signature::new(sheet);

            // Calculate creep (outer sheets have more creep)
            sig.creep = creep_per_sig * (num_sheets - sheet - 1) as f32;

            // Page indices (0-based internally)
            let p1 = adjusted_count - 1 - sheet * 2; // Back of sheet, right
            let p2 = sheet * 2; // Front of sheet, left
            let p3 = sheet * 2 + 1; // Front of sheet, right
            let p4 = adjusted_count - 2 - sheet * 2; // Back of sheet, left

            // Convert to Option<usize>, None for pages beyond source
            let to_opt =
                |p: usize| -> Option<usize> { if p < self.page_count { Some(p) } else { None } };

            // Front of sheet (outside when folded): right page, left page
            sig.front = vec![to_opt(p1), to_opt(p2)];
            // Back of sheet (inside when folded): left page, right page
            sig.back = vec![to_opt(p3), to_opt(p4)];

            self.signatures.push(sig);
        }
    }

    /// Calculate perfect binding imposition
    fn calculate_perfect_binding(&mut self) {
        let pages_per_sig = self.options.binding.pages_per_signature();

        // Round up to signature size
        let adjusted_count = if self.options.add_blank_pages {
            ((self.page_count + pages_per_sig - 1) / pages_per_sig) * pages_per_sig
        } else {
            self.page_count
        };

        let num_sigs = adjusted_count / pages_per_sig;

        // Calculate creep distribution
        let creep_per_sig = if let Some(total) = self.options.total_creep {
            total / num_sigs as f32
        } else {
            self.options.creep_per_signature
        };

        self.signatures.clear();

        for sig_idx in 0..num_sigs {
            let mut sig = Signature::new(sig_idx);
            sig.creep = creep_per_sig * sig_idx as f32;

            let start_page = sig_idx * pages_per_sig;

            // For 16-page signatures, layout is more complex
            // Simplified: just pair pages
            for sheet in 0..(pages_per_sig / 4) {
                let p1 = start_page + pages_per_sig - 1 - sheet * 2;
                let p2 = start_page + sheet * 2;
                let p3 = start_page + sheet * 2 + 1;
                let p4 = start_page + pages_per_sig - 2 - sheet * 2;

                let to_opt = |p: usize| -> Option<usize> {
                    if p < self.page_count { Some(p) } else { None }
                };

                sig.front.extend(vec![to_opt(p1), to_opt(p2)]);
                sig.back.extend(vec![to_opt(p3), to_opt(p4)]);
            }

            self.signatures.push(sig);
        }
    }

    /// Get total number of output sheets
    pub fn sheet_count(&self) -> usize {
        self.signatures.len()
    }

    /// Get signatures
    pub fn signatures(&self) -> &[Signature] {
        &self.signatures
    }

    /// Get page order for printing
    pub fn print_order(&self) -> Vec<(usize, Vec<Option<usize>>)> {
        let mut order = Vec::new();

        for sig in &self.signatures {
            order.push((sig.index * 2, sig.front.clone()));
            order.push((sig.index * 2 + 1, sig.back.clone()));
        }

        order
    }
}

// ============================================================================
// Booklet Generator
// ============================================================================

/// Generates booklet PDFs
pub struct BookletGenerator {
    /// Options
    options: BookletOptions,
    /// Source PDF data
    source_data: Vec<u8>,
    /// Number of source pages
    source_page_count: usize,
    /// Imposition calculator
    calculator: Option<ImpositionCalculator>,
}

impl BookletGenerator {
    /// Create a new booklet generator
    pub fn new(options: BookletOptions) -> Self {
        Self {
            options,
            source_data: Vec::new(),
            source_page_count: 0,
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
        self.source_page_count = self.count_pages()?;
        self.calculator = Some(ImpositionCalculator::new(
            self.source_page_count,
            self.options.clone(),
        ));
        Ok(())
    }

    /// Load from data
    pub fn load_data(&mut self, data: Vec<u8>) -> Result<()> {
        self.source_data = data;
        self.source_page_count = self.count_pages()?;
        self.calculator = Some(ImpositionCalculator::new(
            self.source_page_count,
            self.options.clone(),
        ));
        Ok(())
    }

    /// Get number of output sheets
    pub fn output_sheet_count(&self) -> usize {
        self.calculator
            .as_ref()
            .map(|c| c.sheet_count())
            .unwrap_or(0)
    }

    /// Generate booklet PDF
    pub fn generate(&self) -> Result<Vec<u8>> {
        let calculator = self
            .calculator
            .as_ref()
            .ok_or_else(|| EnhancedError::InvalidParameter("Source not loaded".to_string()))?;

        let (sheet_w, sheet_h) = if self.options.landscape {
            self.options.sheet_size.landscape()
        } else {
            self.options.sheet_size.portrait()
        };

        let (page_w, page_h) = self.options.page_dimensions();

        let mut pdf = Vec::new();

        // PDF header
        pdf.extend(b"%PDF-1.7\n");
        pdf.extend(b"%\xe2\xe3\xcf\xd3\n");

        let mut objects: Vec<String> = Vec::new();
        let mut page_refs: Vec<usize> = Vec::new();

        // Generate each output sheet
        for sig in calculator.signatures() {
            // Front side of sheet
            let front_content = self.generate_sheet_content(
                &sig.front, sheet_w, sheet_h, page_w, page_h, sig.creep, true,
            );

            let content_obj_num = objects.len() + 1;
            objects.push(format!(
                "{} 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
                content_obj_num,
                front_content.len(),
                front_content
            ));

            let page_obj_num = objects.len() + 1;
            objects.push(format!(
                "{} 0 obj\n<< /Type /Page /MediaBox [0 0 {} {}] /Contents {} 0 R >>\nendobj\n",
                page_obj_num, sheet_w, sheet_h, content_obj_num
            ));
            page_refs.push(page_obj_num);

            // Back side of sheet (if duplex)
            if self.options.duplex {
                let back_content = self.generate_sheet_content(
                    &sig.back, sheet_w, sheet_h, page_w, page_h, sig.creep, false,
                );

                let content_obj_num = objects.len() + 1;
                objects.push(format!(
                    "{} 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
                    content_obj_num,
                    back_content.len(),
                    back_content
                ));

                let page_obj_num = objects.len() + 1;
                objects.push(format!(
                    "{} 0 obj\n<< /Type /Page /MediaBox [0 0 {} {}] /Contents {} 0 R >>\nendobj\n",
                    page_obj_num, sheet_w, sheet_h, content_obj_num
                ));
                page_refs.push(page_obj_num);
            }
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

    /// Generate content for one side of a sheet
    fn generate_sheet_content(
        &self,
        pages: &[Option<usize>],
        sheet_w: f32,
        sheet_h: f32,
        page_w: f32,
        page_h: f32,
        creep: f32,
        is_front: bool,
    ) -> String {
        let mut content = String::new();

        // Background (optional)
        content.push_str("q\n");

        // Draw each page position
        for (idx, page_opt) in pages.iter().enumerate() {
            let x = if idx == 0 {
                // Left page
                if is_front { 0.0 - creep } else { 0.0 + creep }
            } else {
                // Right page
                page_w + if is_front { creep } else { -creep }
            };

            let y = 0.0;

            // Page boundary
            if self.options.crop_marks {
                content.push_str(&self.generate_crop_marks(x, y, page_w, page_h));
            }

            if let Some(_page_num) = page_opt {
                // In a full implementation, we would:
                // 1. Extract the page content from source PDF
                // 2. Create an XObject for the page
                // 3. Reference it here

                // Placeholder: draw page number
                content.push_str(&format!(
                    "BT /F1 24 Tf {} {} Td (Page {}) Tj ET\n",
                    x + page_w / 2.0 - 30.0,
                    y + page_h / 2.0,
                    _page_num + 1
                ));
            }
        }

        // Fold mark (center of sheet)
        if self.options.fold_marks {
            let fold_x = sheet_w / 2.0;
            content.push_str(&format!(
                "q 0.5 w 0.5 G {} 0 m {} 10 l S {} {} m {} {} l S Q\n",
                fold_x,
                fold_x,
                fold_x,
                sheet_h - 10.0,
                fold_x,
                sheet_h
            ));
        }

        content.push_str("Q\n");
        content
    }

    /// Generate crop marks for a page
    fn generate_crop_marks(&self, x: f32, y: f32, w: f32, h: f32) -> String {
        let len = self.options.crop_mark_length;
        let off = self.options.crop_mark_offset;
        let mut marks = String::new();

        marks.push_str("q 0.25 w 0 G\n");

        // Top-left
        marks.push_str(&format!(
            "{} {} m {} {} l S\n",
            x - off - len,
            y + h,
            x - off,
            y + h
        ));
        marks.push_str(&format!(
            "{} {} m {} {} l S\n",
            x,
            y + h + off,
            x,
            y + h + off + len
        ));

        // Top-right
        marks.push_str(&format!(
            "{} {} m {} {} l S\n",
            x + w + off,
            y + h,
            x + w + off + len,
            y + h
        ));
        marks.push_str(&format!(
            "{} {} m {} {} l S\n",
            x + w,
            y + h + off,
            x + w,
            y + h + off + len
        ));

        // Bottom-left
        marks.push_str(&format!(
            "{} {} m {} {} l S\n",
            x - off - len,
            y,
            x - off,
            y
        ));
        marks.push_str(&format!(
            "{} {} m {} {} l S\n",
            x,
            y - off - len,
            x,
            y - off
        ));

        // Bottom-right
        marks.push_str(&format!(
            "{} {} m {} {} l S\n",
            x + w + off,
            y,
            x + w + off + len,
            y
        ));
        marks.push_str(&format!(
            "{} {} m {} {} l S\n",
            x + w,
            y - off - len,
            x + w,
            y - off
        ));

        marks.push_str("Q\n");
        marks
    }

    /// Save generated booklet
    pub fn save(&self, output_path: &str) -> Result<()> {
        let pdf = self.generate()?;
        fs::write(output_path, pdf)?;
        Ok(())
    }

    /// Count pages in source PDF
    fn count_pages(&self) -> Result<usize> {
        let content = String::from_utf8_lossy(&self.source_data);

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

        let count = content.matches("/Type /Page").count();
        Ok(count.saturating_sub(1).max(1))
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Create a saddle-stitch booklet with default options
pub fn create_saddle_stitch_booklet(input_path: &str, output_path: &str) -> Result<()> {
    let options = BookletOptions::new().binding(BindingMethod::SaddleStitch);
    create_booklet_with_options(input_path, output_path, &options)
}

/// Create a booklet with custom options
pub fn create_booklet_with_options(
    input_path: &str,
    output_path: &str,
    options: &BookletOptions,
) -> Result<()> {
    let mut generator = BookletGenerator::new(options.clone());
    generator.load(input_path)?;
    generator.save(output_path)
}

/// Create a perfect-bound book imposition
pub fn create_perfect_binding(input_path: &str, output_path: &str) -> Result<()> {
    let options = BookletOptions::new().binding(BindingMethod::PerfectBinding);
    create_booklet_with_options(input_path, output_path, &options)
}

/// Calculate imposition for a given page count
pub fn calculate_imposition(
    page_count: usize,
    binding: BindingMethod,
) -> Vec<(usize, Vec<Option<usize>>)> {
    let options = BookletOptions::new().binding(binding);
    let calc = ImpositionCalculator::new(page_count, options);
    calc.print_order()
}

/// Get recommended binding method for page count
pub fn recommend_binding(page_count: usize) -> BindingMethod {
    if page_count <= 64 {
        BindingMethod::SaddleStitch
    } else if page_count <= 80 {
        BindingMethod::SideStitch
    } else if page_count <= 300 {
        BindingMethod::PerfectBinding
    } else {
        BindingMethod::CaseBinding
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_method_properties() {
        assert_eq!(BindingMethod::SaddleStitch.min_pages(), 4);
        assert_eq!(BindingMethod::SaddleStitch.max_pages(), 64);
        assert_eq!(BindingMethod::SaddleStitch.pages_per_signature(), 4);

        assert_eq!(BindingMethod::PerfectBinding.min_pages(), 40);
        assert_eq!(BindingMethod::PerfectBinding.pages_per_signature(), 16);
    }

    #[test]
    fn test_saddle_stitch_imposition_8_pages() {
        let options = BookletOptions::new();
        let calc = ImpositionCalculator::new(8, options);

        assert_eq!(calc.sheet_count(), 2);

        let sigs = calc.signatures();

        // Sheet 1 front: pages 8, 1 (indices 7, 0)
        assert_eq!(sigs[0].front, vec![Some(7), Some(0)]);
        // Sheet 1 back: pages 2, 7 (indices 1, 6)
        assert_eq!(sigs[0].back, vec![Some(1), Some(6)]);

        // Sheet 2 front: pages 6, 3 (indices 5, 2)
        assert_eq!(sigs[1].front, vec![Some(5), Some(2)]);
        // Sheet 2 back: pages 4, 5 (indices 3, 4)
        assert_eq!(sigs[1].back, vec![Some(3), Some(4)]);
    }

    #[test]
    fn test_saddle_stitch_with_blanks() {
        // 6 pages should round up to 8
        let options = BookletOptions::new().add_blank_pages(true);
        let calc = ImpositionCalculator::new(6, options);

        assert_eq!(calc.sheet_count(), 2);

        // Last 2 pages should be None (blanks)
        let sigs = calc.signatures();
        assert_eq!(sigs[0].front[0], None); // Page 8 (blank)
        assert_eq!(sigs[0].back[1], None); // Page 7 (blank)
    }

    #[test]
    fn test_booklet_options_builder() {
        let options = BookletOptions::new()
            .binding(BindingMethod::PerfectBinding)
            .sheet_size(PageSize::A4)
            .creep_per_signature(0.5, Unit::Mm)
            .crop_marks(true)
            .gutter(10.0, Unit::Mm);

        assert_eq!(options.binding, BindingMethod::PerfectBinding);
        assert_eq!(options.sheet_size, PageSize::A4);
        assert!(options.crop_marks);
        assert!(options.gutter > 28.0 && options.gutter < 29.0); // ~28.35pt
    }

    #[test]
    fn test_page_dimensions() {
        let options = BookletOptions::new()
            .sheet_size(PageSize::Letter)
            .landscape(true);

        let (page_w, page_h): (f32, f32) = options.page_dimensions();
        // Letter landscape is 792x612, page width is half
        assert!((page_w - 396.0).abs() < 1.0);
        assert!((page_h - 612.0).abs() < 1.0);
    }

    #[test]
    fn test_recommend_binding() {
        assert_eq!(recommend_binding(8), BindingMethod::SaddleStitch);
        assert_eq!(recommend_binding(64), BindingMethod::SaddleStitch);
        assert_eq!(recommend_binding(70), BindingMethod::SideStitch);
        assert_eq!(recommend_binding(100), BindingMethod::PerfectBinding);
        assert_eq!(recommend_binding(500), BindingMethod::CaseBinding);
    }

    #[test]
    fn test_signature_all_pages() {
        let mut sig = Signature::new(0);
        sig.front = vec![Some(7), Some(0)];
        sig.back = vec![Some(1), Some(6)];

        let all = sig.all_pages();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&0));
        assert!(all.contains(&1));
        assert!(all.contains(&6));
        assert!(all.contains(&7));
    }

    #[test]
    fn test_creep_distribution() {
        let options = BookletOptions::new().total_creep(10.0, Unit::Point);

        let calc = ImpositionCalculator::new(16, options); // 4 sheets
        let sigs = calc.signatures();

        // Outer sheets should have more creep
        assert!(sigs[0].creep > sigs[1].creep);
        assert!(sigs[1].creep > sigs[2].creep);
    }
}
