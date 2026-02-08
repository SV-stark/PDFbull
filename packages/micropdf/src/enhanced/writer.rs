//! PDF Writer - Create and modify PDF documents
//!
//! Complete implementation for creating new PDFs with pages and content.

use super::error::{EnhancedError, Result};
use crate::pdf::object::{Array, Dict, Name, ObjRef, Object};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Seek, Write};

/// Text element to draw on a page
#[derive(Debug, Clone)]
pub struct TextElement {
    /// Text content
    pub text: String,
    /// X position in points (from left)
    pub x: f32,
    /// Y position in points (from top - will be converted to PDF coordinates)
    pub y: f32,
    /// Bounding box height in points (for proper vertical positioning)
    pub height: f32,
    /// Font size in points
    pub font_size: f32,
    /// Font name (must be registered with add_ttf_font)
    pub font_name: String,
    /// Text color RGB (0.0-1.0)
    pub color: (f32, f32, f32),
    /// Text rendering mode (0=fill, 3=invisible for OCR)
    pub render_mode: i32,
}

impl TextElement {
    /// Create a new text element
    pub fn new(text: impl Into<String>, x: f32, y: f32, height: f32, font_size: f32) -> Self {
        Self {
            text: text.into(),
            x,
            y,
            height,
            font_size,
            font_name: "F1".to_string(),
            color: (0.0, 0.0, 0.0),
            render_mode: 0, // Fill (visible)
        }
    }

    /// Set font name
    pub fn with_font(mut self, name: impl Into<String>) -> Self {
        self.font_name = name.into();
        self
    }

    /// Set color
    pub fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color = (r, g, b);
        self
    }

    /// Set invisible (for OCR text layers)
    pub fn invisible(mut self) -> Self {
        self.render_mode = 3;
        self
    }
}

/// Image element to draw on a page
#[derive(Debug, Clone)]
pub struct ImageElement {
    /// X position in points (from left)
    pub x: f32,
    /// Y position in points (from top)
    pub y: f32,
    /// Width in points
    pub width: f32,
    /// Height in points
    pub height: f32,
    /// Image data (PNG or JPEG)
    pub data: Vec<u8>,
    /// Image format
    pub format: ImageFormat,
}

/// Supported image formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
}

impl ImageElement {
    /// Create a new image element
    pub fn new(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        data: Vec<u8>,
        format: ImageFormat,
    ) -> Self {
        Self {
            x,
            y,
            width,
            height,
            data,
            format,
        }
    }
}

/// Registered font info
#[derive(Debug, Clone)]
struct RegisteredFont {
    /// Font data (TTF file contents)
    data: Vec<u8>,
    /// Object number for font dictionary
    obj_num: Option<usize>,
    /// Glyph widths (char code -> width in 1/1000 em)
    widths: HashMap<u16, u16>,
    /// First char code
    first_char: u16,
    /// Last char code
    last_char: u16,
}

/// PDF Writer for creating new documents
pub struct PdfWriter {
    /// Objects in the PDF
    objects: Vec<Object>,
    /// Pages array
    pages: Vec<usize>, // Object numbers of page objects
    /// Next object number
    next_obj_num: usize,
    /// Registered fonts (name -> font info)
    fonts: HashMap<String, RegisteredFont>,
}

impl PdfWriter {
    /// Create a new PDF writer
    pub fn new() -> Self {
        Self {
            objects: vec![Object::Null], // Object 0 is null
            pages: Vec::new(),
            next_obj_num: 1,
            fonts: HashMap::new(),
        }
    }

    /// Register a TTF font for use in text elements
    ///
    /// # Arguments
    /// * `name` - Font name to use in TextElement (e.g., "F1")
    /// * `data` - TTF font file contents
    ///
    /// Returns Ok if the font was successfully parsed and registered.
    pub fn add_ttf_font(&mut self, name: impl Into<String>, data: Vec<u8>) -> Result<()> {
        use ttf_parser::Face;

        let font_name = name.into();

        // Parse the TTF font
        let face = Face::parse(&data, 0).map_err(|e| {
            EnhancedError::InvalidParameter(format!("Failed to parse TTF font: {:?}", e))
        })?;

        // Build character widths map
        let mut widths: HashMap<u16, u16> = HashMap::new();
        let mut first_char: u16 = 255;
        let mut last_char: u16 = 0;

        let units_per_em = face.units_per_em() as f32;

        // Map ASCII printable characters (32-126) and extended latin (128-255)
        for char_code in 32u16..=255 {
            if let Some(glyph_id) =
                face.glyph_index(char::from_u32(char_code as u32).unwrap_or(' '))
            {
                let advance = face.glyph_hor_advance(glyph_id).unwrap_or(0);
                // Convert to 1/1000 em units (PDF standard)
                let width_1000 = ((advance as f32 / units_per_em) * 1000.0) as u16;
                widths.insert(char_code, width_1000);

                if char_code < first_char {
                    first_char = char_code;
                }
                if char_code > last_char {
                    last_char = char_code;
                }
            }
        }

        if first_char > last_char {
            first_char = 32;
            last_char = 126;
        }

        self.fonts.insert(
            font_name,
            RegisteredFont {
                data,
                obj_num: None,
                widths,
                first_char,
                last_char,
            },
        );

        Ok(())
    }

    /// Check if a font is registered
    pub fn has_font(&self, name: &str) -> bool {
        self.fonts.contains_key(name)
    }

    /// Add an object and return its object number
    fn add_object(&mut self, obj: Object) -> usize {
        let obj_num = self.next_obj_num;
        self.next_obj_num += 1;
        self.objects.push(obj);
        obj_num
    }

    /// Add a blank page with specified dimensions
    pub fn add_blank_page(&mut self, width: f32, height: f32) -> Result<()> {
        if width <= 0.0 || height <= 0.0 {
            return Err(EnhancedError::InvalidParameter(format!(
                "Invalid page dimensions: {}x{}",
                width, height
            )));
        }

        if width > 14400.0 || height > 14400.0 {
            return Err(EnhancedError::InvalidParameter(format!(
                "Page dimensions too large: {}x{} (max 14400)",
                width, height
            )));
        }

        // Create page content stream (empty for blank page)
        let content_data = b"".to_vec();
        let mut content_dict = Dict::new();
        content_dict.insert(Name::new("Length"), Object::Int(content_data.len() as i64));

        let content_obj = Object::Stream {
            dict: content_dict,
            data: content_data,
        };
        let content_ref = self.add_object(content_obj);

        // Create page dictionary
        let mut page_dict = Dict::new();
        page_dict.insert(Name::new("Type"), Object::Name(Name::new("Page")));

        // MediaBox: [0 0 width height]
        let media_box = Object::Array(vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(width as f64),
            Object::Real(height as f64),
        ]);
        page_dict.insert(Name::new("MediaBox"), media_box);

        // Resources (empty for now)
        let mut resources = Dict::new();
        resources.insert(
            Name::new("ProcSet"),
            Object::Array(vec![
                Object::Name(Name::new("PDF")),
                Object::Name(Name::new("Text")),
            ]),
        );
        page_dict.insert(Name::new("Resources"), Object::Dict(resources));

        // Contents reference
        page_dict.insert(
            Name::new("Contents"),
            Object::Ref(ObjRef::new(content_ref as i32, 0)),
        );

        // We'll set Parent later when creating Pages tree
        let page_obj_num = self.add_object(Object::Dict(page_dict));
        self.pages.push(page_obj_num);

        Ok(())
    }

    /// Add a page with content
    pub fn add_page_with_content(&mut self, width: f32, height: f32, content: &str) -> Result<()> {
        if width <= 0.0 || height <= 0.0 {
            return Err(EnhancedError::InvalidParameter(format!(
                "Invalid page dimensions: {}x{}",
                width, height
            )));
        }

        // Create content stream
        let content_data = content.as_bytes().to_vec();
        let mut content_dict = Dict::new();
        content_dict.insert(Name::new("Length"), Object::Int(content_data.len() as i64));

        let content_obj = Object::Stream {
            dict: content_dict,
            data: content_data,
        };
        let content_ref = self.add_object(content_obj);

        // Create page dictionary
        let mut page_dict = Dict::new();
        page_dict.insert(Name::new("Type"), Object::Name(Name::new("Page")));

        let media_box = Object::Array(vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(width as f64),
            Object::Real(height as f64),
        ]);
        page_dict.insert(Name::new("MediaBox"), media_box);

        let mut resources = Dict::new();
        resources.insert(
            Name::new("ProcSet"),
            Object::Array(vec![
                Object::Name(Name::new("PDF")),
                Object::Name(Name::new("Text")),
            ]),
        );
        page_dict.insert(Name::new("Resources"), Object::Dict(resources));
        page_dict.insert(
            Name::new("Contents"),
            Object::Ref(ObjRef::new(content_ref as i32, 0)),
        );

        let page_obj_num = self.add_object(Object::Dict(page_dict));
        self.pages.push(page_obj_num);

        Ok(())
    }

    /// Get number of pages
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Add a page with transparent highlight rectangles
    ///
    /// Creates a page with filled rectangles that support transparency.
    /// Each rectangle is defined as (x, y, w, h, r, g, b, alpha) where:
    /// - x, y: position from bottom-left corner in points
    /// - w, h: width and height in points  
    /// - r, g, b: color components (0.0-1.0)
    /// - alpha: transparency (0.0=transparent, 1.0=opaque)
    pub fn add_highlight_page(
        &mut self,
        width: f32,
        height: f32,
        highlights: &[(f32, f32, f32, f32, f32, f32, f32, f32)], // x, y, w, h, r, g, b, alpha
    ) -> Result<()> {
        if width <= 0.0 || height <= 0.0 {
            return Err(EnhancedError::InvalidParameter(format!(
                "Invalid page dimensions: {}x{}",
                width, height
            )));
        }

        // Create ExtGState dictionary for transparency if needed
        let needs_transparency = highlights.iter().any(|(_, _, _, _, _, _, _, a)| *a < 1.0);

        // Generate content stream for highlights
        let mut content = String::new();

        for (x, y, w, h, r, g, b, alpha) in highlights {
            // Transform Y coordinate: PDF uses bottom-left origin, but input is top-left
            let pdf_y = height - y - h;

            // Set graphics state for transparency if needed
            if *alpha < 1.0 {
                content.push_str(&format!("/GS{} gs\n", (alpha * 100.0) as i32));
            }

            // Set fill color (RGB)
            content.push_str(&format!("{:.3} {:.3} {:.3} rg\n", r, g, b));

            // Draw filled rectangle: x y w h re f
            content.push_str(&format!("{:.2} {:.2} {:.2} {:.2} re f\n", x, pdf_y, w, h));
        }

        // Create content stream
        let content_data = content.as_bytes().to_vec();
        let mut content_dict = Dict::new();
        content_dict.insert(Name::new("Length"), Object::Int(content_data.len() as i64));

        let content_obj = Object::Stream {
            dict: content_dict,
            data: content_data,
        };
        let content_ref = self.add_object(content_obj);

        // Create page dictionary
        let mut page_dict = Dict::new();
        page_dict.insert(Name::new("Type"), Object::Name(Name::new("Page")));

        let media_box = Object::Array(vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(width as f64),
            Object::Real(height as f64),
        ]);
        page_dict.insert(Name::new("MediaBox"), media_box);

        // Build resources dictionary
        let mut resources = Dict::new();
        resources.insert(
            Name::new("ProcSet"),
            Object::Array(vec![Object::Name(Name::new("PDF"))]),
        );

        // Add ExtGState resources for transparency if needed
        if needs_transparency {
            let mut ext_g_state = Dict::new();

            // Create graphics states for different alpha values
            for (_, _, _, _, _, _, _, alpha) in highlights {
                if *alpha < 1.0 {
                    let gs_name = format!("GS{}", (alpha * 100.0) as i32);
                    let mut gs_dict = Dict::new();
                    gs_dict.insert(Name::new("Type"), Object::Name(Name::new("ExtGState")));
                    gs_dict.insert(Name::new("ca"), Object::Real(*alpha as f64)); // Fill alpha
                    gs_dict.insert(Name::new("CA"), Object::Real(*alpha as f64)); // Stroke alpha
                    ext_g_state.insert(Name::new(&gs_name), Object::Dict(gs_dict));
                }
            }

            resources.insert(Name::new("ExtGState"), Object::Dict(ext_g_state));
        }

        page_dict.insert(Name::new("Resources"), Object::Dict(resources));
        page_dict.insert(
            Name::new("Contents"),
            Object::Ref(ObjRef::new(content_ref as i32, 0)),
        );

        let page_obj_num = self.add_object(Object::Dict(page_dict));
        self.pages.push(page_obj_num);

        Ok(())
    }

    /// Add a page with text elements and optional background image
    ///
    /// This is designed for creating OCR text overlay PDFs where:
    /// - Text is positioned at specific coordinates (from Textract or OCR)
    /// - An image is placed behind the text as background
    /// - Text can be invisible (render_mode=3) for searchable PDFs
    ///
    /// # Arguments
    /// * `width` - Page width in points
    /// * `height` - Page height in points
    /// * `texts` - Text elements to draw
    /// * `image` - Optional background image (drawn first, behind text)
    pub fn add_text_overlay_page(
        &mut self,
        width: f32,
        height: f32,
        texts: &[TextElement],
        image: Option<&ImageElement>,
    ) -> Result<()> {
        if width <= 0.0 || height <= 0.0 {
            return Err(EnhancedError::InvalidParameter(format!(
                "Invalid page dimensions: {}x{}",
                width, height
            )));
        }

        // Collect unique fonts used in this page
        let font_names: Vec<String> = texts
            .iter()
            .map(|t| t.font_name.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        // Build font resources
        let mut font_resources = Dict::new();
        for font_name in &font_names {
            // Clone font info to avoid borrow checker issues
            let font_info_opt = self.fonts.get(font_name).cloned();

            if let Some(font_info) = font_info_opt {
                // Create font descriptor
                let mut font_desc = Dict::new();
                font_desc.insert(Name::new("Type"), Object::Name(Name::new("FontDescriptor")));
                font_desc.insert(
                    Name::new("FontName"),
                    Object::Name(Name::new(&format!("{}+{}", "AAAAAA", font_name))),
                );
                font_desc.insert(Name::new("Flags"), Object::Int(32)); // Nonsymbolic
                font_desc.insert(
                    Name::new("FontBBox"),
                    Object::Array(vec![
                        Object::Int(-500),
                        Object::Int(-300),
                        Object::Int(1500),
                        Object::Int(1000),
                    ]),
                );
                font_desc.insert(Name::new("ItalicAngle"), Object::Int(0));
                font_desc.insert(Name::new("Ascent"), Object::Int(1000));
                font_desc.insert(Name::new("Descent"), Object::Int(-200));
                font_desc.insert(Name::new("CapHeight"), Object::Int(700));
                font_desc.insert(Name::new("StemV"), Object::Int(80));

                // Embed font data as stream
                let mut font_file_dict = Dict::new();
                font_file_dict.insert(
                    Name::new("Length1"),
                    Object::Int(font_info.data.len() as i64),
                );
                font_file_dict.insert(
                    Name::new("Length"),
                    Object::Int(font_info.data.len() as i64),
                );
                let font_file_obj = Object::Stream {
                    dict: font_file_dict,
                    data: font_info.data.clone(),
                };
                let font_file_ref = self.add_object(font_file_obj);
                font_desc.insert(
                    Name::new("FontFile2"),
                    Object::Ref(ObjRef::new(font_file_ref as i32, 0)),
                );

                let font_desc_ref = self.add_object(Object::Dict(font_desc));

                // Build widths array
                let mut widths_array = Vec::new();
                for i in font_info.first_char..=font_info.last_char {
                    let w = font_info.widths.get(&i).copied().unwrap_or(500);
                    widths_array.push(Object::Int(w as i64));
                }

                // Create font dictionary
                let mut font_dict = Dict::new();
                font_dict.insert(Name::new("Type"), Object::Name(Name::new("Font")));
                font_dict.insert(Name::new("Subtype"), Object::Name(Name::new("TrueType")));
                font_dict.insert(
                    Name::new("BaseFont"),
                    Object::Name(Name::new(&format!("AAAAAA+{}", font_name))),
                );
                font_dict.insert(
                    Name::new("FirstChar"),
                    Object::Int(font_info.first_char as i64),
                );
                font_dict.insert(
                    Name::new("LastChar"),
                    Object::Int(font_info.last_char as i64),
                );
                font_dict.insert(Name::new("Widths"), Object::Array(widths_array));
                font_dict.insert(
                    Name::new("FontDescriptor"),
                    Object::Ref(ObjRef::new(font_desc_ref as i32, 0)),
                );
                font_dict.insert(
                    Name::new("Encoding"),
                    Object::Name(Name::new("WinAnsiEncoding")),
                );

                let font_ref = self.add_object(Object::Dict(font_dict));
                font_resources.insert(
                    Name::new(font_name),
                    Object::Ref(ObjRef::new(font_ref as i32, 0)),
                );
            } else {
                // Use standard Helvetica as fallback
                let mut font_dict = Dict::new();
                font_dict.insert(Name::new("Type"), Object::Name(Name::new("Font")));
                font_dict.insert(Name::new("Subtype"), Object::Name(Name::new("Type1")));
                font_dict.insert(Name::new("BaseFont"), Object::Name(Name::new("Helvetica")));
                font_dict.insert(
                    Name::new("Encoding"),
                    Object::Name(Name::new("WinAnsiEncoding")),
                );

                let font_ref = self.add_object(Object::Dict(font_dict));
                font_resources.insert(
                    Name::new(font_name),
                    Object::Ref(ObjRef::new(font_ref as i32, 0)),
                );
            }
        }

        // Build image XObject if present
        let mut xobject_resources = Dict::new();
        let image_ref = if let Some(img) = image {
            let img_ref = self.add_image_xobject(img)?;
            xobject_resources.insert(
                Name::new("Im1"),
                Object::Ref(ObjRef::new(img_ref as i32, 0)),
            );
            Some(img_ref)
        } else {
            None
        };

        // Generate content stream
        let mut content = String::new();

        // Draw image first (background)
        if let (Some(_img_ref), Some(img)) = (image_ref, image) {
            // Save graphics state
            content.push_str("q\n");
            // Transform matrix: scale and position the image
            // cm operator: a b c d e f
            // For an image: width 0 0 height x y cm
            let pdf_y = height - img.y - img.height;
            content.push_str(&format!(
                "{:.2} 0 0 {:.2} {:.2} {:.2} cm\n",
                img.width, img.height, img.x, pdf_y
            ));
            // Draw image
            content.push_str("/Im1 Do\n");
            // Restore graphics state
            content.push_str("Q\n");
        }

        // Draw text elements
        if !texts.is_empty() {
            content.push_str("BT\n"); // Begin text

            for text in texts {
                // Set font and size
                content.push_str(&format!("/{} {} Tf\n", text.font_name, text.font_size));

                // Set text render mode if not default
                if text.render_mode != 0 {
                    content.push_str(&format!("{} Tr\n", text.render_mode));
                }

                // Set color
                content.push_str(&format!(
                    "{:.3} {:.3} {:.3} rg\n",
                    text.color.0, text.color.1, text.color.2
                ));

                // Position text (convert from top-left to bottom-left coordinates)
                // Use bounding box height for positioning to match highlight rectangles
                // The text baseline is positioned at the bottom of the bounding box
                let pdf_y = height - text.y - text.height;
                content.push_str(&format!("{:.2} {:.2} Td\n", text.x, pdf_y));

                // Show text (escape special characters)
                let escaped = Self::escape_pdf_string(&text.text);
                content.push_str(&format!("({}) Tj\n", escaped));

                // Reset text matrix for next element
                content.push_str(&format!("{:.2} {:.2} Td\n", -text.x, -pdf_y));
            }

            content.push_str("ET\n"); // End text
        }

        // Create content stream
        let content_data = content.as_bytes().to_vec();
        let mut content_dict = Dict::new();
        content_dict.insert(Name::new("Length"), Object::Int(content_data.len() as i64));

        let content_obj = Object::Stream {
            dict: content_dict,
            data: content_data,
        };
        let content_ref = self.add_object(content_obj);

        // Create page dictionary
        let mut page_dict = Dict::new();
        page_dict.insert(Name::new("Type"), Object::Name(Name::new("Page")));

        let media_box = Object::Array(vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(width as f64),
            Object::Real(height as f64),
        ]);
        page_dict.insert(Name::new("MediaBox"), media_box);

        // Build resources dictionary
        let mut resources = Dict::new();
        resources.insert(
            Name::new("ProcSet"),
            Object::Array(vec![
                Object::Name(Name::new("PDF")),
                Object::Name(Name::new("Text")),
                Object::Name(Name::new("ImageC")),
            ]),
        );

        if !font_resources.is_empty() {
            resources.insert(Name::new("Font"), Object::Dict(font_resources));
        }

        if !xobject_resources.is_empty() {
            resources.insert(Name::new("XObject"), Object::Dict(xobject_resources));
        }

        page_dict.insert(Name::new("Resources"), Object::Dict(resources));
        page_dict.insert(
            Name::new("Contents"),
            Object::Ref(ObjRef::new(content_ref as i32, 0)),
        );

        let page_obj_num = self.add_object(Object::Dict(page_dict));
        self.pages.push(page_obj_num);

        Ok(())
    }

    /// Add an image XObject and return its object number
    fn add_image_xobject(&mut self, img: &ImageElement) -> Result<usize> {
        // Decode the image to get dimensions and raw data
        let cursor = std::io::Cursor::new(&img.data);
        let reader = image::ImageReader::new(cursor)
            .with_guessed_format()
            .map_err(|e| EnhancedError::Generic(format!("Failed to read image: {}", e)))?;

        let decoded = reader
            .decode()
            .map_err(|e| EnhancedError::Generic(format!("Failed to decode image: {}", e)))?;

        let rgb_image = decoded.to_rgb8();
        let (img_width, img_height) = rgb_image.dimensions();

        // Create image XObject
        let mut img_dict = Dict::new();
        img_dict.insert(Name::new("Type"), Object::Name(Name::new("XObject")));
        img_dict.insert(Name::new("Subtype"), Object::Name(Name::new("Image")));
        img_dict.insert(Name::new("Width"), Object::Int(img_width as i64));
        img_dict.insert(Name::new("Height"), Object::Int(img_height as i64));
        img_dict.insert(
            Name::new("ColorSpace"),
            Object::Name(Name::new("DeviceRGB")),
        );
        img_dict.insert(Name::new("BitsPerComponent"), Object::Int(8));

        // Use DCTDecode (JPEG) for smaller file size, or raw for PNG
        let (data, filter) = match img.format {
            ImageFormat::Jpeg => {
                // For JPEG, we can use the original data directly
                (img.data.clone(), Some("DCTDecode"))
            }
            ImageFormat::Png => {
                // For PNG, convert to raw RGB and optionally compress with FlateDecode
                let raw_data: Vec<u8> = rgb_image.into_raw();

                // Compress with zlib/deflate
                use flate2::Compression;
                use flate2::write::ZlibEncoder;
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&raw_data)?;
                let compressed = encoder.finish()?;

                (compressed, Some("FlateDecode"))
            }
        };

        img_dict.insert(Name::new("Length"), Object::Int(data.len() as i64));
        if let Some(f) = filter {
            img_dict.insert(Name::new("Filter"), Object::Name(Name::new(f)));
        }

        let img_obj = Object::Stream {
            dict: img_dict,
            data,
        };

        Ok(self.add_object(img_obj))
    }

    /// Escape special characters in PDF string
    fn escape_pdf_string(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '(' | ')' | '\\' => {
                    result.push('\\');
                    result.push(c);
                }
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                c if c.is_ascii() && (32..=126).contains(&(c as u8)) => {
                    result.push(c);
                }
                c => {
                    // For non-ASCII, encode as octal
                    let bytes = c.to_string();
                    for b in bytes.bytes() {
                        result.push_str(&format!("\\{:03o}", b));
                    }
                }
            }
        }
        result
    }

    /// Save the PDF to a file
    pub fn save(&self, path: &str) -> Result<()> {
        if self.pages.is_empty() {
            return Err(EnhancedError::InvalidParameter(
                "Cannot save PDF with no pages".into(),
            ));
        }

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Write PDF header
        writer.write_all(b"%PDF-1.4\n")?;
        writer.write_all(b"%\xE2\xE3\xCF\xD3\n")?; // Binary comment

        // Track object offsets for xref
        let mut offsets = vec![0usize; self.objects.len()];

        // Create Pages tree
        let pages_kids: Array = self
            .pages
            .iter()
            .map(|&obj_num| Object::Ref(ObjRef::new(obj_num as i32, 0)))
            .collect();

        let mut pages_dict = Dict::new();
        pages_dict.insert(Name::new("Type"), Object::Name(Name::new("Pages")));
        pages_dict.insert(Name::new("Kids"), Object::Array(pages_kids));
        pages_dict.insert(Name::new("Count"), Object::Int(self.pages.len() as i64));

        let pages_obj_num = self.objects.len();
        let pages_ref = ObjRef::new(pages_obj_num as i32, 0);

        // Create Catalog
        let mut catalog_dict = Dict::new();
        catalog_dict.insert(Name::new("Type"), Object::Name(Name::new("Catalog")));
        catalog_dict.insert(Name::new("Pages"), Object::Ref(pages_ref));

        let catalog_obj_num = pages_obj_num + 1;

        // Write objects (skip object 0)
        for (idx, offset) in offsets.iter_mut().enumerate().skip(1) {
            *offset = writer.stream_position().map(|p| p as usize)?;

            // Add Parent reference to page objects
            let obj = if self.pages.contains(&idx) {
                if let Object::Dict(ref mut dict) = self.objects[idx].clone() {
                    let mut page_dict = dict.clone();
                    page_dict.insert(Name::new("Parent"), Object::Ref(pages_ref));
                    Object::Dict(page_dict)
                } else {
                    self.objects[idx].clone()
                }
            } else {
                self.objects[idx].clone()
            };

            self.write_indirect_object(&mut writer, idx, 0, &obj)?;
        }

        // Write Pages object
        let pages_offset = writer.stream_position().map(|p| p as usize)?;
        self.write_indirect_object(&mut writer, pages_obj_num, 0, &Object::Dict(pages_dict))?;

        // Write Catalog object
        let catalog_offset = writer.stream_position().map(|p| p as usize)?;
        self.write_indirect_object(&mut writer, catalog_obj_num, 0, &Object::Dict(catalog_dict))?;

        // Write xref table
        let xref_offset = writer.stream_position().map(|p| p as usize)?;
        writer.write_all(b"xref\n")?;
        writer.write_all(format!("0 {}\n", catalog_obj_num + 1).as_bytes())?;

        // Object 0 (free)
        writer.write_all(b"0000000000 65535 f \n")?;

        // Regular objects
        for offset in offsets.iter().skip(1) {
            writer.write_all(format!("{:010} 00000 n \n", offset).as_bytes())?;
        }

        // Pages and Catalog
        writer.write_all(format!("{:010} 00000 n \n", pages_offset).as_bytes())?;
        writer.write_all(format!("{:010} 00000 n \n", catalog_offset).as_bytes())?;

        // Write trailer
        writer.write_all(b"trailer\n")?;
        writer.write_all(b"<<\n")?;
        writer.write_all(format!("/Size {}\n", catalog_obj_num + 1).as_bytes())?;
        writer.write_all(format!("/Root {} 0 R\n", catalog_obj_num).as_bytes())?;
        writer.write_all(b">>\n")?;
        writer.write_all(b"startxref\n")?;
        writer.write_all(format!("{}\n", xref_offset).as_bytes())?;
        writer.write_all(b"%%EOF\n")?;

        writer.flush()?;
        Ok(())
    }

    /// Write an indirect object
    fn write_indirect_object<W: Write>(
        &self,
        writer: &mut W,
        obj_num: usize,
        generation: usize,
        obj: &Object,
    ) -> Result<()> {
        writer.write_all(format!("{} {} obj\n", obj_num, generation).as_bytes())?;
        self.write_object(writer, obj)?;
        writer.write_all(b"\nendobj\n")?;
        Ok(())
    }

    /// Write a PDF object
    #[allow(clippy::only_used_in_recursion)]
    fn write_object<W: Write>(&self, writer: &mut W, obj: &Object) -> Result<()> {
        match obj {
            Object::Null => writer.write_all(b"null")?,
            Object::Bool(b) => writer.write_all(if *b { b"true" } else { b"false" })?,
            Object::Int(i) => writer.write_all(i.to_string().as_bytes())?,
            Object::Real(r) => {
                let s = format!("{:.6}", r)
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string();
                writer.write_all(s.as_bytes())?;
            }
            Object::String(s) => {
                writer.write_all(b"(")?;
                for &byte in s.as_bytes() {
                    match byte {
                        b'(' | b')' | b'\\' => {
                            writer.write_all(b"\\")?;
                            writer.write_all(&[byte])?;
                        }
                        b'\n' => writer.write_all(b"\\n")?,
                        b'\r' => writer.write_all(b"\\r")?,
                        b'\t' => writer.write_all(b"\\t")?,
                        _ if (32..=126).contains(&byte) => writer.write_all(&[byte])?,
                        _ => writer.write_all(format!("\\{:03o}", byte).as_bytes())?,
                    }
                }
                writer.write_all(b")")?;
            }
            Object::Name(n) => writer.write_all(format!("/{}", n.as_str()).as_bytes())?,
            Object::Array(arr) => {
                writer.write_all(b"[")?;
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        writer.write_all(b" ")?;
                    }
                    self.write_object(writer, item)?;
                }
                writer.write_all(b"]")?;
            }
            Object::Dict(dict) => {
                writer.write_all(b"<<\n")?;
                for (key, value) in dict.iter() {
                    writer.write_all(format!("/{} ", key.as_str()).as_bytes())?;
                    self.write_object(writer, value)?;
                    writer.write_all(b"\n")?;
                }
                writer.write_all(b">>")?;
            }
            Object::Stream { dict, data } => {
                writer.write_all(b"<<\n")?;
                for (key, value) in dict.iter() {
                    writer.write_all(format!("/{} ", key.as_str()).as_bytes())?;
                    self.write_object(writer, value)?;
                    writer.write_all(b"\n")?;
                }
                writer.write_all(b">>\nstream\n")?;
                writer.write_all(data)?;
                writer.write_all(b"\nendstream")?;
            }
            Object::Ref(r) => {
                writer.write_all(format!("{} {} R", r.num, r.generation).as_bytes())?
            }
        }
        Ok(())
    }
}

impl Default for PdfWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_writer_new() {
        let writer = PdfWriter::new();
        assert_eq!(writer.page_count(), 0);
        assert_eq!(writer.next_obj_num, 1);
    }

    #[test]
    fn test_add_blank_page() {
        let mut writer = PdfWriter::new();
        assert!(writer.add_blank_page(612.0, 792.0).is_ok());
        assert_eq!(writer.page_count(), 1);
    }

    #[test]
    fn test_add_blank_page_invalid_dimensions() {
        let mut writer = PdfWriter::new();
        assert!(writer.add_blank_page(0.0, 792.0).is_err());
        assert!(writer.add_blank_page(612.0, 0.0).is_err());
        assert!(writer.add_blank_page(-100.0, 792.0).is_err());
    }

    #[test]
    fn test_add_blank_page_too_large() {
        let mut writer = PdfWriter::new();
        assert!(writer.add_blank_page(20000.0, 792.0).is_err());
    }

    #[test]
    fn test_add_multiple_pages() {
        let mut writer = PdfWriter::new();
        writer.add_blank_page(612.0, 792.0).unwrap();
        writer.add_blank_page(612.0, 792.0).unwrap();
        writer.add_blank_page(612.0, 792.0).unwrap();
        assert_eq!(writer.page_count(), 3);
    }

    #[test]
    fn test_add_page_with_content() {
        let mut writer = PdfWriter::new();
        let content = "BT /F1 12 Tf 100 700 Td (Hello World) Tj ET";
        assert!(writer.add_page_with_content(612.0, 792.0, content).is_ok());
        assert_eq!(writer.page_count(), 1);
    }

    #[test]
    fn test_save_no_pages() {
        let writer = PdfWriter::new();
        let temp = NamedTempFile::new().unwrap();
        let result = writer.save(temp.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_save_with_pages() -> Result<()> {
        let mut writer = PdfWriter::new();
        writer.add_blank_page(612.0, 792.0)?;

        let temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        writer.save(temp.path().to_str().unwrap())?;

        // Verify file was created and starts with %PDF
        let data = std::fs::read(temp.path())?;
        assert!(data.starts_with(b"%PDF-1.4"));
        assert!(data.ends_with(b"%%EOF\n"));

        Ok(())
    }

    #[test]
    fn test_save_multiple_pages() -> Result<()> {
        let mut writer = PdfWriter::new();
        writer.add_blank_page(612.0, 792.0)?;
        writer.add_blank_page(612.0, 792.0)?;
        writer.add_blank_page(612.0, 792.0)?;

        let temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        writer.save(temp.path().to_str().unwrap())?;

        let data = std::fs::read(temp.path())?;
        assert!(data.starts_with(b"%PDF-1.4"));

        // Check that xref and trailer are present
        let content = String::from_utf8_lossy(&data);
        assert!(content.contains("xref"));
        assert!(content.contains("trailer"));
        assert!(content.contains("/Type /Catalog"));
        assert!(content.contains("/Type /Pages"));
        assert!(content.contains("/Count 3"));

        Ok(())
    }

    #[test]
    fn test_save_with_content() -> Result<()> {
        let mut writer = PdfWriter::new();
        let content = "BT /F1 12 Tf 100 700 Td (Test) Tj ET";
        writer.add_page_with_content(612.0, 792.0, content)?;

        let temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        writer.save(temp.path().to_str().unwrap())?;

        let data = std::fs::read(temp.path())?;
        let content_str = String::from_utf8_lossy(&data);
        assert!(content_str.contains("Test"));

        Ok(())
    }
}
