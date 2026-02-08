//! Page implementation with content stream interpretation
//!
//! Implements PDF page rendering, content stream parsing, and Device integration.
//! Supports graphics state management, font handling, and image rendering.

use crate::fitz::colorspace::Colorspace;
use crate::fitz::device::{BlendMode, Device};
use crate::fitz::error::{Error, Result};
use crate::fitz::font::Font;
use crate::fitz::geometry::{Matrix, Point, Rect};
use crate::fitz::image::Image;
use crate::fitz::path::{LineCap, LineJoin, Path, PathElement, StrokeState};
use crate::fitz::pixmap::Pixmap;
use crate::fitz::render::Rasterizer;
use crate::fitz::text::{BidiDirection, Text, TextLanguage};
use crate::pdf::document::{Document, Page};
use crate::pdf::object::{Array, Dict, Name, ObjRef, Object};
use std::collections::HashMap;
use std::sync::Arc;

/// Graphics state - tracks current drawing parameters
#[derive(Clone)]
pub struct GraphicsState {
    /// Current transformation matrix
    pub ctm: Matrix,
    /// Clipping path
    pub clip_path: Option<Path>,
    /// Fill colorspace
    pub fill_colorspace: Colorspace,
    /// Stroke colorspace
    pub stroke_colorspace: Colorspace,
    /// Fill color
    pub fill_color: Vec<f32>,
    /// Stroke color
    pub stroke_color: Vec<f32>,
    /// Stroke state
    pub stroke_state: StrokeState,
    /// Font
    pub font: Option<Arc<Font>>,
    /// Font size
    pub font_size: f32,
    /// Text rise
    pub text_rise: f32,
    /// Text knockout
    pub text_knockout: bool,
    /// Rendering intent
    pub rendering_intent: String,
    /// Flatness tolerance
    pub flatness: f32,
    /// Overprint mode (fill)
    pub overprint_fill: bool,
    /// Overprint mode (stroke)
    pub overprint_stroke: bool,
    /// Alpha constant (fill)
    pub alpha_fill: f32,
    /// Alpha constant (stroke)
    pub alpha_stroke: f32,
    /// Blend mode
    pub blend_mode: BlendMode,
    /// Soft mask
    pub soft_mask: Option<String>,
    /// Black generation
    pub black_generation: Option<Object>,
    /// Undercolor removal
    pub undercolor_removal: Option<Object>,
    /// Transfer function
    pub transfer: Option<Object>,
    /// Halftone
    pub halftone: Option<Object>,
}

impl GraphicsState {
    /// Create default graphics state
    pub fn new() -> Self {
        Self {
            ctm: Matrix::IDENTITY,
            clip_path: None,
            fill_colorspace: Colorspace::device_gray(),
            stroke_colorspace: Colorspace::device_gray(),
            fill_color: vec![0.0],
            stroke_color: vec![0.0],
            stroke_state: StrokeState::default(),
            font: None,
            font_size: 0.0,
            text_rise: 0.0,
            text_knockout: true,
            rendering_intent: "RelativeColorimetric".to_string(),
            flatness: 1.0,
            overprint_fill: false,
            overprint_stroke: false,
            alpha_fill: 1.0,
            alpha_stroke: 1.0,
            blend_mode: BlendMode::Normal,
            soft_mask: None,
            black_generation: None,
            undercolor_removal: None,
            transfer: None,
            halftone: None,
        }
    }
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource dictionary manager
pub struct Resources {
    /// Font dictionary: name -> Font
    fonts: HashMap<String, Arc<Font>>,
    /// XObject dictionary: name -> Image/Form
    xobjects: HashMap<String, XObject>,
    /// Colorspace dictionary: name -> Colorspace
    colorspaces: HashMap<String, Colorspace>,
    /// Pattern dictionary
    patterns: HashMap<String, Object>,
    /// ExtGState dictionary: name -> graphics state parameters
    ext_gstates: HashMap<String, Dict>,
    /// Properties (for marked content)
    properties: HashMap<String, Dict>,
}

/// XObject types
pub enum XObject {
    Image(Image),
    Form(FormXObject),
}

/// Form XObject
pub struct FormXObject {
    pub bbox: Rect,
    pub matrix: Matrix,
    pub resources: Resources,
    pub content: Vec<u8>,
}

impl Resources {
    /// Create empty resources
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            xobjects: HashMap::new(),
            colorspaces: HashMap::new(),
            patterns: HashMap::new(),
            ext_gstates: HashMap::new(),
            properties: HashMap::new(),
        }
    }

    /// Load resources from a PDF dictionary
    pub fn from_dict(dict: &Dict, doc: &Document) -> Result<Self> {
        let mut resources = Self::new();

        // Load fonts
        if let Some(Object::Dict(fonts_dict)) = dict.get(&Name::new("Font")) {
            for (name, obj) in fonts_dict {
                if let Ok(font) = Self::load_font(obj, doc) {
                    resources
                        .fonts
                        .insert(name.as_str().to_string(), Arc::new(font));
                }
            }
        }

        // Load XObjects
        if let Some(Object::Dict(xobj_dict)) = dict.get(&Name::new("XObject")) {
            for (name, obj) in xobj_dict {
                if let Ok(xobj) = Self::load_xobject(obj, doc) {
                    resources.xobjects.insert(name.as_str().to_string(), xobj);
                }
            }
        }

        // Load colorspaces
        if let Some(Object::Dict(cs_dict)) = dict.get(&Name::new("ColorSpace")) {
            for (name, obj) in cs_dict {
                if let Ok(cs) = Self::load_colorspace(obj) {
                    resources.colorspaces.insert(name.as_str().to_string(), cs);
                }
            }
        }

        // Load ExtGState
        if let Some(Object::Dict(gs_dict)) = dict.get(&Name::new("ExtGState")) {
            for (name, obj) in gs_dict {
                if let Object::Dict(gs) = obj {
                    resources
                        .ext_gstates
                        .insert(name.as_str().to_string(), gs.clone());
                }
            }
        }

        Ok(resources)
    }

    /// Load a font from PDF object
    fn load_font(obj: &Object, doc: &Document) -> Result<Font> {
        let font_dict = match obj {
            Object::Ref(r) => match doc.resolve_object_ref(*r)? {
                Object::Dict(d) => d,
                _ => return Err(Error::Generic("Font is not a dictionary".into())),
            },
            Object::Dict(d) => d.clone(),
            _ => {
                return Err(Error::Generic(
                    "Font is not a dictionary or reference".into(),
                ))
            }
        };

        let subtype = font_dict
            .get(&Name::new("Subtype"))
            .and_then(|o| o.as_name())
            .map(|n| n.as_str())
            .unwrap_or("Type1");

        let base_font = font_dict
            .get(&Name::new("BaseFont"))
            .and_then(|o| o.as_name())
            .map(|n| n.as_str().to_string())
            .unwrap_or_else(|| "Helvetica".to_string());

        let mut font = Font::with_type(
            &base_font,
            crate::fitz::font::FontType::from_string(subtype),
        );

        // Load encoding if present
        if let Some(Object::Name(enc)) = font_dict.get(&Name::new("Encoding")) {
            font.set_encoding(Some(enc.as_str().to_string()));
        }

        // Load widths if present
        if let Some(Object::Array(widths)) = font_dict.get(&Name::new("Widths")) {
            let first_char = font_dict
                .get(&Name::new("FirstChar"))
                .and_then(|o| o.as_int())
                .unwrap_or(0) as u16;

            for (i, width_obj) in widths.iter().enumerate() {
                if let Some(width) = width_obj.as_real() {
                    font.set_glyph_advance(first_char + i as u16, width as f32);
                }
            }
        }

        // Load font descriptor if present
        if let Some(Object::Ref(desc_ref)) = font_dict.get(&Name::new("FontDescriptor")) {
            if let Ok(Object::Dict(desc)) = doc.resolve_object_ref(*desc_ref) {
                // Extract flags
                if let Some(Object::Int(flags)) = desc.get(&Name::new("Flags")) {
                    font.set_flags(crate::fitz::font::FontFlags::new(*flags as u32));
                }

                // Extract embedded font data
                if let Some(Object::Ref(font_file_ref)) = desc
                    .get(&Name::new("FontFile2"))
                    .or_else(|| desc.get(&Name::new("FontFile3")))
                    .or_else(|| desc.get(&Name::new("FontFile")))
                {
                    if let Ok(Object::Stream { data, .. }) = doc.resolve_object_ref(*font_file_ref)
                    {
                        font.set_font_data(data);
                    }
                }
            }
        }

        Ok(font)
    }

    /// Load an XObject from PDF object
    fn load_xobject(obj: &Object, doc: &Document) -> Result<XObject> {
        let xobj_dict = match obj {
            Object::Ref(r) => match doc.resolve_object_ref(*r)? {
                Object::Dict(d) => d,
                Object::Stream { dict, .. } => dict,
                _ => {
                    return Err(Error::Generic(
                        "XObject is not a dictionary or stream".into(),
                    ))
                }
            },
            Object::Dict(d) => d.clone(),
            _ => {
                return Err(Error::Generic(
                    "XObject is not a dictionary or reference".into(),
                ))
            }
        };

        let subtype = xobj_dict
            .get(&Name::new("Subtype"))
            .and_then(|o| o.as_name())
            .map(|n| n.as_str())
            .unwrap_or("Image");

        match subtype {
            "Image" => {
                let width = xobj_dict
                    .get(&Name::new("Width"))
                    .and_then(|o| o.as_int())
                    .unwrap_or(0) as i32;
                let height = xobj_dict
                    .get(&Name::new("Height"))
                    .and_then(|o| o.as_int())
                    .unwrap_or(0) as i32;

                // Load colorspace if specified
                let colorspace = xobj_dict
                    .get(&Name::new("ColorSpace"))
                    .and_then(|o| Self::load_colorspace(o).ok());

                // Load image data
                let image = if let Object::Ref(r) = obj {
                    if let Ok(Object::Stream { dict: _, data }) = doc.resolve_object_ref(*r) {
                        Image::from_raw(
                            width,
                            height,
                            8,
                            colorspace.unwrap_or(Colorspace::device_rgb()),
                            data,
                        )?
                    } else {
                        Image::new(width, height, None)
                    }
                } else {
                    Image::new(width, height, None)
                };

                Ok(XObject::Image(image))
            }
            "Form" => {
                let bbox = xobj_dict
                    .get(&Name::new("BBox"))
                    .and_then(|o| o.as_array())
                    .map(|a| Self::array_to_rect(a))
                    .unwrap_or(Rect::UNIT);

                let matrix = xobj_dict
                    .get(&Name::new("Matrix"))
                    .and_then(|o| o.as_array())
                    .map(|a| Self::array_to_matrix(a))
                    .unwrap_or(Matrix::IDENTITY);

                let resources =
                    if let Some(Object::Dict(res_dict)) = xobj_dict.get(&Name::new("Resources")) {
                        Resources::from_dict(res_dict, doc)?
                    } else {
                        Resources::new()
                    };

                let content = if let Object::Ref(r) = obj {
                    if let Ok(Object::Stream { data, .. }) = doc.resolve_object_ref(*r) {
                        data
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                Ok(XObject::Form(FormXObject {
                    bbox,
                    matrix,
                    resources,
                    content,
                }))
            }
            _ => Err(Error::Generic(format!(
                "Unknown XObject subtype: {}",
                subtype
            ))),
        }
    }

    /// Load a colorspace from PDF object
    fn load_colorspace(obj: &Object) -> Result<Colorspace> {
        match obj {
            Object::Name(name) => {
                match name.as_str() {
                    "DeviceGray" | "G" => Ok(Colorspace::device_gray()),
                    "DeviceRGB" | "RGB" => Ok(Colorspace::device_rgb()),
                    "DeviceCMYK" | "CMYK" => Ok(Colorspace::device_cmyk()),
                    _ => Ok(Colorspace::device_rgb()), // Default to RGB
                }
            }
            Object::Array(arr) => {
                if arr.is_empty() {
                    return Ok(Colorspace::device_rgb());
                }
                // Handle complex colorspaces
                if let Some(Object::Name(name)) = arr.first() {
                    match name.as_str() {
                        "ICCBased" => Ok(Colorspace::device_rgb()),
                        "Indexed" => Ok(Colorspace::device_rgb()),
                        "Separation" => Ok(Colorspace::device_cmyk()),
                        _ => Ok(Colorspace::device_rgb()),
                    }
                } else {
                    Ok(Colorspace::device_rgb())
                }
            }
            _ => Ok(Colorspace::device_rgb()),
        }
    }

    /// Convert array to rectangle
    fn array_to_rect(arr: &Array) -> Rect {
        if arr.len() >= 4 {
            Rect::new(
                arr[0].as_real().unwrap_or(0.0) as f32,
                arr[1].as_real().unwrap_or(0.0) as f32,
                arr[2].as_real().unwrap_or(0.0) as f32,
                arr[3].as_real().unwrap_or(0.0) as f32,
            )
        } else {
            Rect::EMPTY
        }
    }

    /// Convert array to matrix
    fn array_to_matrix(arr: &Array) -> Matrix {
        if arr.len() >= 6 {
            Matrix::new(
                arr[0].as_real().unwrap_or(1.0) as f32,
                arr[1].as_real().unwrap_or(0.0) as f32,
                arr[2].as_real().unwrap_or(0.0) as f32,
                arr[3].as_real().unwrap_or(1.0) as f32,
                arr[4].as_real().unwrap_or(0.0) as f32,
                arr[5].as_real().unwrap_or(0.0) as f32,
            )
        } else {
            Matrix::IDENTITY
        }
    }

    /// Get a font by name
    pub fn get_font(&self, name: &str) -> Option<Arc<Font>> {
        self.fonts.get(name).cloned()
    }

    /// Get an XObject by name
    pub fn get_xobject(&self, name: &str) -> Option<&XObject> {
        self.xobjects.get(name)
    }

    /// Get a colorspace by name
    pub fn get_colorspace(&self, name: &str) -> Option<&Colorspace> {
        self.colorspaces.get(name)
    }

    /// Get ExtGState by name
    pub fn get_ext_gstate(&self, name: &str) -> Option<&Dict> {
        self.ext_gstates.get(name)
    }
}

impl Default for Resources {
    fn default() -> Self {
        Self::new()
    }
}

/// Content stream interpreter
pub struct ContentInterpreter<'a, 'b> {
    /// Document reference
    doc: &'a Document,
    /// Page reference
    page: &'a Page,
    /// Device to render to
    device: &'b mut dyn Device,
    /// Graphics state stack
    gs_stack: Vec<GraphicsState>,
    /// Current graphics state
    gs: GraphicsState,
    /// Resources
    resources: Resources,
    /// Text object (for TJ operator)
    text: Option<Text>,
    /// Text matrix
    text_matrix: Matrix,
    /// Text line matrix
    text_line_matrix: Matrix,
}

impl<'a, 'b> ContentInterpreter<'a, 'b> {
    /// Create a new content interpreter
    pub fn new(doc: &'a Document, page: &'a Page, device: &'b mut dyn Device) -> Result<Self> {
        let resources = Resources::from_dict(&page.resources(), doc)?;

        Ok(Self {
            doc,
            page,
            device,
            gs_stack: Vec::new(),
            gs: GraphicsState::new(),
            resources,
            text: None,
            text_matrix: Matrix::IDENTITY,
            text_line_matrix: Matrix::IDENTITY,
        })
    }

    /// Run the interpreter on the page's content stream
    pub fn run(&mut self) -> Result<()> {
        let contents = self.page.contents();

        for content_ref in contents {
            let content_data = match self.doc.resolve_object_ref(content_ref)? {
                Object::Stream { data, .. } => data,
                _ => continue,
            };

            self.interpret_stream(&content_data)?;
        }

        Ok(())
    }

    /// Interpret a content stream
    fn interpret_stream(&mut self, data: &[u8]) -> Result<()> {
        use std::io::BufRead;

        let reader = std::io::Cursor::new(data);
        let mut operands: Vec<Object> = Vec::new();

        for line in reader.split(b'\n') {
            let line = line?;
            let line_str = String::from_utf8_lossy(&line);

            // Simple tokenization (for production, use proper lexer)
            let tokens: Vec<&str> = line_str.split_whitespace().collect();

            for token in tokens {
                // Try to parse as number
                if let Ok(num) = token.parse::<i64>() {
                    operands.push(Object::Int(num));
                } else if let Ok(num) = token.parse::<f64>() {
                    operands.push(Object::Real(num));
                } else if token.starts_with('/') {
                    // Name
                    operands.push(Object::Name(Name::new(&token[1..])));
                } else if token.starts_with('(') && token.ends_with(')') {
                    // String literal
                    let s = &token[1..token.len() - 1];
                    operands.push(Object::String(crate::pdf::object::PdfString::new(
                        s.as_bytes().to_vec(),
                    )));
                } else if token.starts_with('[') {
                    // Start of array - collect until ]
                    operands.push(Object::Array(Array::new()));
                } else {
                    // Operator
                    self.execute_operator(token, &mut operands)?;
                }
            }
        }

        Ok(())
    }

    /// Execute a PDF operator
    fn execute_operator(&mut self, op: &str, operands: &mut Vec<Object>) -> Result<()> {
        match op {
            // Graphics state
            "q" => {
                // Save graphics state
                self.gs_stack.push(self.gs.clone());
            }
            "Q" => {
                // Restore graphics state
                if let Some(gs) = self.gs_stack.pop() {
                    self.gs = gs;
                }
            }
            "cm" => {
                // Concatenate matrix
                if operands.len() >= 6 {
                    let a = operands[operands.len() - 6].as_real().unwrap_or(1.0) as f32;
                    let b = operands[operands.len() - 5].as_real().unwrap_or(0.0) as f32;
                    let c = operands[operands.len() - 4].as_real().unwrap_or(0.0) as f32;
                    let d = operands[operands.len() - 3].as_real().unwrap_or(1.0) as f32;
                    let e = operands[operands.len() - 2].as_real().unwrap_or(0.0) as f32;
                    let f = operands[operands.len() - 1].as_real().unwrap_or(0.0) as f32;
                    let m = Matrix::new(a, b, c, d, e, f);
                    self.gs.ctm = self.gs.ctm.concat(&m);
                    operands.truncate(operands.len() - 6);
                }
            }
            "w" => {
                // Set line width
                if let Some(obj) = operands.pop() {
                    self.gs.stroke_state.linewidth = obj.as_real().unwrap_or(1.0) as f32;
                }
            }
            "J" => {
                // Set line cap
                if let Some(Object::Int(cap)) = operands.pop() {
                    self.gs.stroke_state.start_cap = match cap {
                        0 => LineCap::Butt,
                        1 => LineCap::Round,
                        2 => LineCap::Square,
                        _ => LineCap::Butt,
                    };
                    self.gs.stroke_state.end_cap = self.gs.stroke_state.start_cap;
                }
            }
            "j" => {
                // Set line join
                if let Some(Object::Int(join)) = operands.pop() {
                    self.gs.stroke_state.linejoin = match join {
                        0 => LineJoin::Miter,
                        1 => LineJoin::Round,
                        2 => LineJoin::Bevel,
                        _ => LineJoin::Miter,
                    };
                }
            }
            "M" => {
                // Set miter limit
                if let Some(obj) = operands.pop() {
                    self.gs.stroke_state.miterlimit = obj.as_real().unwrap_or(10.0) as f32;
                }
            }
            "d" => {
                // Set dash pattern
                if operands.len() >= 2 {
                    if let Some(Object::Array(arr)) = operands.get(operands.len() - 2) {
                        let pattern: Vec<f32> = arr
                            .iter()
                            .filter_map(|o| o.as_real().map(|v| v as f32))
                            .collect();
                        self.gs.stroke_state.dash_pattern = pattern;
                    }
                    if let Some(Object::Int(phase)) = operands.pop() {
                        self.gs.stroke_state.dash_phase = phase as f32;
                    }
                    operands.pop(); // Remove the array
                }
            }
            "gs" => {
                // Set graphics state from ExtGState
                if let Some(Object::Name(name)) = operands.pop() {
                    if let Some(gs_dict) = self.resources.get_ext_gstate(name.as_str()) {
                        self.apply_ext_gstate(gs_dict)?;
                    }
                }
            }

            // Colors
            "cs" | "CS" => {
                // Set colorspace (fill or stroke)
                if let Some(Object::Name(name)) = operands.pop() {
                    if let Some(cs) = self.resources.get_colorspace(name.as_str()) {
                        if op == "cs" {
                            self.gs.fill_colorspace = cs.clone();
                        } else {
                            self.gs.stroke_colorspace = cs.clone();
                        }
                    }
                }
            }
            "sc" | "scn" => {
                // Set color (fill)
                let n = self.gs.fill_colorspace.n() as usize;
                let mut color = Vec::new();
                for _ in 0..n {
                    if let Some(obj) = operands.pop() {
                        color.push(obj.as_real().unwrap_or(0.0) as f32);
                    }
                }
                color.reverse();
                self.gs.fill_color = color;
            }
            "SC" | "SCN" => {
                // Set color (stroke)
                let n = self.gs.stroke_colorspace.n() as usize;
                let mut color = Vec::new();
                for _ in 0..n {
                    if let Some(obj) = operands.pop() {
                        color.push(obj.as_real().unwrap_or(0.0) as f32);
                    }
                }
                color.reverse();
                self.gs.stroke_color = color;
            }
            "g" | "G" => {
                // Set gray color
                if let Some(obj) = operands.pop() {
                    let gray = obj.as_real().unwrap_or(0.0) as f32;
                    if op == "g" {
                        self.gs.fill_colorspace = Colorspace::device_gray();
                        self.gs.fill_color = vec![gray];
                    } else {
                        self.gs.stroke_colorspace = Colorspace::device_gray();
                        self.gs.stroke_color = vec![gray];
                    }
                }
            }
            "rg" | "RG" => {
                // Set RGB color
                if operands.len() >= 3 {
                    let b = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let g = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let r = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    if op == "rg" {
                        self.gs.fill_colorspace = Colorspace::device_rgb();
                        self.gs.fill_color = vec![r, g, b];
                    } else {
                        self.gs.stroke_colorspace = Colorspace::device_rgb();
                        self.gs.stroke_color = vec![r, g, b];
                    }
                }
            }
            "k" | "K" => {
                // Set CMYK color
                if operands.len() >= 4 {
                    let k = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let y = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let m = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let c = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    if op == "k" {
                        self.gs.fill_colorspace = Colorspace::device_cmyk();
                        self.gs.fill_color = vec![c, m, y, k];
                    } else {
                        self.gs.stroke_colorspace = Colorspace::device_cmyk();
                        self.gs.stroke_color = vec![c, m, y, k];
                    }
                }
            }

            // Path construction
            "m" => {
                // Move to
                if operands.len() >= 2 {
                    let y = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let x = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    // Path construction would go here
                }
            }
            "l" => {
                // Line to
                if operands.len() >= 2 {
                    let _y = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let _x = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                }
            }
            "c" => {
                // Curve to
                if operands.len() >= 6 {
                    for _ in 0..6 {
                        operands.pop();
                    }
                }
            }
            "h" => {
                // Close path
            }
            "re" => {
                // Rectangle
                if operands.len() >= 4 {
                    let _h = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let _w = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let _y = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let _x = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                }
            }

            // Path painting
            "S" => {
                // Stroke path
            }
            "s" => {
                // Close and stroke path
            }
            "f" | "F" | "f*" => {
                // Fill path (even-odd or non-zero winding)
                let even_odd = op == "f*";
            }
            "B" | "B*" => {
                // Fill and stroke path
            }
            "b" | "b*" => {
                // Close, fill and stroke path
            }
            "n" => {
                // End path (no-op)
            }

            // Text operations
            "BT" => {
                // Begin text
                self.text = Some(Text::new());
                self.text_matrix = Matrix::IDENTITY;
                self.text_line_matrix = Matrix::IDENTITY;
            }
            "ET" => {
                // End text
                if let Some(text) = self.text.take() {
                    self.device.fill_text(
                        &text,
                        &self.gs.ctm.concat(&self.text_matrix),
                        &self.gs.fill_colorspace,
                        &self.gs.fill_color,
                        self.gs.alpha_fill,
                    );
                }
            }
            "Tm" => {
                // Set text matrix
                if operands.len() >= 6 {
                    let f = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let e = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let d = operands.pop().unwrap().as_real().unwrap_or(1.0) as f32;
                    let c = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let b = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let a = operands.pop().unwrap().as_real().unwrap_or(1.0) as f32;
                    self.text_matrix = Matrix::new(a, b, c, d, e, f);
                    self.text_line_matrix = self.text_matrix;
                }
            }
            "Td" => {
                // Move text position
                if operands.len() >= 2 {
                    let ty = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let tx = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let m = Matrix::translate(tx, ty);
                    self.text_line_matrix = self.text_line_matrix.concat(&m);
                    self.text_matrix = self.text_line_matrix;
                }
            }
            "TD" => {
                // Move text position and set leading
                if operands.len() >= 2 {
                    let ty = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    let tx = operands.pop().unwrap().as_real().unwrap_or(0.0) as f32;
                    // Set leading to -ty
                    let m = Matrix::translate(tx, ty);
                    self.text_line_matrix = self.text_line_matrix.concat(&m);
                    self.text_matrix = self.text_line_matrix;
                }
            }
            "T*" => {
                // Move to next line
                let m = Matrix::translate(0.0, -12.0); // Default leading
                self.text_line_matrix = self.text_line_matrix.concat(&m);
                self.text_matrix = self.text_line_matrix;
            }
            "Tf" => {
                // Set font
                if operands.len() >= 2 {
                    if let Some(Object::Real(size)) | Some(Object::Int(size)) = operands.pop() {
                        self.gs.font_size = size as f32;
                    }
                    if let Some(Object::Name(name)) = operands.pop() {
                        if let Some(font) = self.resources.get_font(name.as_str()) {
                            self.gs.font = Some(font);
                        }
                    }
                }
            }
            "Tj" => {
                // Show text
                if let Some(Object::String(s)) = operands.pop() {
                    if let Some(ref mut text) = self.text {
                        if let Some(ref font) = self.gs.font {
                            let text_matrix = self.text_matrix;
                            text.show_string(
                                Arc::clone(font),
                                text_matrix,
                                s.as_str().unwrap_or(""),
                                false, // wmode
                                0,     // bidi_level
                                BidiDirection::Ltr,
                                TextLanguage::Unset,
                            );
                        }
                    }
                }
            }
            "TJ" => {
                // Show text with positioning
                if let Some(Object::Array(arr)) = operands.pop() {
                    if let Some(ref mut text) = self.text {
                        if let Some(ref font) = self.gs.font {
                            for item in arr {
                                match item {
                                    Object::String(s) => {
                                        let text_matrix = self.text_matrix;
                                        text.show_string(
                                            Arc::clone(font),
                                            text_matrix,
                                            s.as_str().unwrap_or(""),
                                            false,
                                            0,
                                            BidiDirection::Ltr,
                                            TextLanguage::Unset,
                                        );
                                    }
                                    Object::Real(adj) | Object::Int(adj) => {
                                        // Apply adjustment (in thousandths of text unit)
                                        let _adjustment = *adj as f32 * self.gs.font_size / 1000.0;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }

            // XObjects
            "Do" => {
                // Draw XObject
                if let Some(Object::Name(name)) = operands.pop() {
                    if let Some(xobj) = self.resources.get_xobject(name.as_str()) {
                        match xobj {
                            XObject::Image(image) => {
                                self.device
                                    .fill_image(image, &self.gs.ctm, self.gs.alpha_fill);
                            }
                            XObject::Form(form) => {
                                // Save state
                                self.gs_stack.push(self.gs.clone());

                                // Apply form matrix
                                self.gs.ctm = self.gs.ctm.concat(&form.matrix);

                                // Use form resources (falling back to current)
                                let old_resources =
                                    std::mem::replace(&mut self.resources, form.resources.clone());

                                // Interpret form content
                                let _ = self.interpret_stream(&form.content);

                                // Restore resources
                                self.resources = old_resources;

                                // Restore state
                                if let Some(gs) = self.gs_stack.pop() {
                                    self.gs = gs;
                                }
                            }
                        }
                    }
                }
            }

            // Clipping
            "W" | "W*" => {
                // Set clipping path (non-zero or even-odd)
                let _even_odd = op == "W*";
            }

            // Transparency
            "gs" => {
                // Set graphics state (handled above)
            }

            _ => {
                // Unknown operator - ignore
            }
        }

        Ok(())
    }

    /// Apply ExtGState dictionary
    fn apply_ext_gstate(&mut self, dict: &Dict) -> Result<()> {
        if let Some(Object::Real(lw)) = dict.get(&Name::new("LW")) {
            self.gs.stroke_state.linewidth = *lw as f32;
        }

        if let Some(Object::Int(lc)) = dict.get(&Name::new("LC")) {
            self.gs.stroke_state.start_cap = match *lc {
                0 => LineCap::Butt,
                1 => LineCap::Round,
                2 => LineCap::Square,
                _ => LineCap::Butt,
            };
            self.gs.stroke_state.end_cap = self.gs.stroke_state.start_cap;
        }

        if let Some(Object::Int(lj)) = dict.get(&Name::new("LJ")) {
            self.gs.stroke_state.linejoin = match *lj {
                0 => LineJoin::Miter,
                1 => LineJoin::Round,
                2 => LineJoin::Bevel,
                _ => LineJoin::Miter,
            };
        }

        if let Some(Object::Real(ca)) = dict.get(&Name::new("CA")) {
            self.gs.alpha_stroke = *ca as f32;
        }

        if let Some(Object::Real(ca)) = dict.get(&Name::new("ca")) {
            self.gs.alpha_fill = *ca as f32;
        }

        if let Some(Object::Name(bm)) = dict.get(&Name::new("BM")) {
            self.gs.blend_mode = BlendMode::from_name(bm.as_str()).unwrap_or(BlendMode::Normal);
        }

        Ok(())
    }
}

/// Render a page to a pixmap
pub fn render_page_to_pixmap(
    doc: &Document,
    page_num: i32,
    width: i32,
    height: i32,
) -> Result<Pixmap> {
    let page = doc.get_page(page_num)?;

    // Create output pixmap
    let mut pixmap = Pixmap::new(Some(Colorspace::device_rgb()), width, height, false)?;

    // Clear to white
    pixmap.clear(255);

    // Create draw device
    let mut draw_device = DrawDevice::new(&mut pixmap);

    // Run interpreter
    let mut interpreter = ContentInterpreter::new(doc, &page, &mut draw_device)?;
    interpreter.run()?;

    Ok(pixmap)
}

/// Draw device - renders to a pixmap
pub struct DrawDevice<'a> {
    pixmap: &'a mut Pixmap,
    clip_stack: Vec<Rect>,
}

impl<'a> DrawDevice<'a> {
    /// Create a new draw device
    pub fn new(pixmap: &'a mut Pixmap) -> Self {
        Self {
            pixmap,
            clip_stack: Vec::new(),
        }
    }
}

impl<'a> Device for DrawDevice<'a> {
    fn fill_path(
        &mut self,
        path: &Path,
        even_odd: bool,
        ctm: &Matrix,
        colorspace: &Colorspace,
        color: &[f32],
        alpha: f32,
    ) {
        let rasterizer = Rasterizer::new(
            self.pixmap.width(),
            self.pixmap.height(),
            Rect::new(
                0.0,
                0.0,
                self.pixmap.width() as f32,
                self.pixmap.height() as f32,
            ),
        );
        rasterizer.fill_path(path, even_odd, ctm, colorspace, color, alpha, self.pixmap);
    }

    fn stroke_path(
        &mut self,
        path: &Path,
        stroke: &StrokeState,
        ctm: &Matrix,
        colorspace: &Colorspace,
        color: &[f32],
        alpha: f32,
    ) {
        let rasterizer = Rasterizer::new(
            self.pixmap.width(),
            self.pixmap.height(),
            Rect::new(
                0.0,
                0.0,
                self.pixmap.width() as f32,
                self.pixmap.height() as f32,
            ),
        );
        rasterizer.stroke_path(path, stroke, ctm, colorspace, color, alpha, self.pixmap);
    }

    fn fill_text(
        &mut self,
        text: &Text,
        ctm: &Matrix,
        colorspace: &Colorspace,
        color: &[f32],
        alpha: f32,
    ) {
        // Render each text span
        let rasterizer = Rasterizer::new(
            self.pixmap.width(),
            self.pixmap.height(),
            Rect::new(
                0.0,
                0.0,
                self.pixmap.width() as f32,
                self.pixmap.height() as f32,
            ),
        );

        for span in text.spans() {
            for item in span.items() {
                // Get glyph outline if available
                if let Ok(glyph_path) = span.font.get_glyph_outline(item.gid as u16) {
                    if !glyph_path.is_empty() {
                        // Transform glyph path by text matrix
                        let mut transformed_path = glyph_path.clone_path();

                        // Apply text transformation
                        // Scale by font size, position at item location
                        let scale_matrix = Matrix::new(
                            span.size() * item.advance,
                            0.0,
                            0.0,
                            span.size(),
                            item.x,
                            item.y,
                        );
                        let final_matrix = ctm.concat(&span.trm).concat(&scale_matrix);

                        transformed_path.transform(|p| final_matrix.transform_point(p));

                        // Fill the glyph path
                        rasterizer.fill_path(
                            &transformed_path,
                            false, // even_odd
                            &Matrix::IDENTITY,
                            colorspace,
                            color,
                            alpha,
                            self.pixmap,
                        );
                    }
                }

                // Fallback: draw a simple rectangle for the glyph if outline not available
                // This provides basic visibility even without font parsing
                if span.font.get_glyph_outline(item.gid as u16).is_err() {
                    let mut fallback_path = Path::new();
                    let font_size = span.size();
                    let char_width = font_size * 0.5; // Approximate width

                    let x = item.x;
                    let y = item.y - font_size * 0.2; // Descender offset

                    fallback_path.rect_coords(x, y, x + char_width, y + font_size);

                    // Transform to device space
                    let mut transformed_path = fallback_path.clone_path();
                    transformed_path.transform(|p| ctm.concat(&span.trm).transform_point(p));

                    rasterizer.fill_path(
                        &transformed_path,
                        false,
                        &Matrix::IDENTITY,
                        colorspace,
                        color,
                        alpha,
                        self.pixmap,
                    );
                }
            }
        }
    }

    fn fill_image(&mut self, image: &Image, ctm: &Matrix, alpha: f32) {
        // Get or create the image pixmap
        let mut image_copy = image.clone();
        let img_pixmap = match image_copy.to_pixmap() {
            Ok(p) => p,
            Err(_) => return,
        };

        let img_width = img_pixmap.width() as f32;
        let img_height = img_pixmap.height() as f32;

        // Calculate the transformed image corners
        let p00 = ctm.transform_point(crate::fitz::geometry::Point::new(0.0, 0.0));
        let p10 = ctm.transform_point(crate::fitz::geometry::Point::new(1.0, 0.0));
        let p01 = ctm.transform_point(crate::fitz::geometry::Point::new(0.0, 1.0));
        let p11 = ctm.transform_point(crate::fitz::geometry::Point::new(1.0, 1.0));

        // For now, use a simple axis-aligned approximation
        // Full implementation would do proper perspective-correct texture mapping
        let min_x = p00.x.min(p10.x).min(p01.x).min(p11.x).max(0.0) as i32;
        let max_x = p00
            .x
            .max(p10.x)
            .max(p01.x)
            .max(p11.x)
            .min(self.pixmap.width() as f32) as i32;
        let min_y = p00.y.min(p10.y).min(p01.y).min(p11.y).max(0.0) as i32;
        let max_y = p00
            .y
            .max(p10.y)
            .max(p01.y)
            .max(p11.y)
            .min(self.pixmap.height() as f32) as i32;

        // Simple bilinear scaling
        for y in min_y..max_y {
            for x in min_x..max_x {
                // Map destination pixel back to source
                let src_x = ((x as f32 - p00.x) / (p10.x - p00.x) * img_width) as i32;
                let src_y = ((y as f32 - p00.y) / (p01.y - p00.y) * img_height) as i32;

                if src_x >= 0
                    && src_x < img_pixmap.width()
                    && src_y >= 0
                    && src_y < img_pixmap.height()
                {
                    // Sample the source image
                    let src_idx =
                        ((src_y * img_pixmap.width() + src_x) * img_pixmap.n() as i32) as usize;
                    let src_samples = img_pixmap.samples();

                    if src_idx + img_pixmap.n() as usize <= src_samples.len() {
                        // Composite with alpha
                        let src_alpha = if img_pixmap.has_alpha() {
                            src_samples[src_idx + img_pixmap.n() as usize - 1] as f32 / 255.0
                        } else {
                            1.0
                        } * alpha;

                        // Get source color
                        let src_r = src_samples[src_idx] as f32;
                        let src_g = src_samples[src_idx + 1] as f32;
                        let src_b = src_samples[src_idx + 2] as f32;

                        // Write to destination (compositing)
                        let dst_idx =
                            ((y * self.pixmap.width() + x) * self.pixmap.n() as i32) as usize;
                        let dst_samples = self.pixmap.samples_mut();

                        if dst_idx + 3 <= dst_samples.len() {
                            let dst_r = dst_samples[dst_idx] as f32;
                            let dst_g = dst_samples[dst_idx + 1] as f32;
                            let dst_b = dst_samples[dst_idx + 2] as f32;

                            // Simple over compositing
                            let out_alpha = src_alpha + (1.0 - src_alpha) * (dst_r / 255.0);
                            dst_samples[dst_idx] =
                                (src_r * src_alpha + dst_r * (1.0 - src_alpha)) as u8;
                            dst_samples[dst_idx + 1] =
                                (src_g * src_alpha + dst_g * (1.0 - src_alpha)) as u8;
                            dst_samples[dst_idx + 2] =
                                (src_b * src_alpha + dst_b * (1.0 - src_alpha)) as u8;
                        }
                    }
                }
            }
        }
    }

    // Other Device trait methods can use default implementations or simple stubs
    fn clip_path(&mut self, _path: &Path, _even_odd: bool, _ctm: &Matrix, _scissor: Rect) {}
    fn clip_stroke_path(
        &mut self,
        _path: &Path,
        _stroke: &StrokeState,
        _ctm: &Matrix,
        _scissor: Rect,
    ) {
    }
    fn stroke_text(
        &mut self,
        _text: &Text,
        _stroke: &StrokeState,
        _ctm: &Matrix,
        _colorspace: &Colorspace,
        _color: &[f32],
        _alpha: f32,
    ) {
    }
    fn clip_text(&mut self, _text: &Text, _ctm: &Matrix, _scissor: Rect) {}
    fn clip_stroke_text(
        &mut self,
        _text: &Text,
        _stroke: &StrokeState,
        _ctm: &Matrix,
        _scissor: Rect,
    ) {
    }
    fn ignore_text(&mut self, _text: &Text, _ctm: &Matrix) {}
    fn fill_image_mask(
        &mut self,
        _image: &Image,
        _ctm: &Matrix,
        _colorspace: &Colorspace,
        _color: &[f32],
        _alpha: f32,
    ) {
    }
    fn clip_image_mask(&mut self, _image: &Image, _ctm: &Matrix, _scissor: Rect) {}
    fn pop_clip(&mut self) {
        self.clip_stack.pop();
    }
    fn begin_mask(
        &mut self,
        _area: Rect,
        _luminosity: bool,
        _colorspace: &Colorspace,
        _color: &[f32],
    ) {
    }
    fn end_mask(&mut self) {}
    fn begin_group(
        &mut self,
        _area: Rect,
        _colorspace: Option<&Colorspace>,
        _isolated: bool,
        _knockout: bool,
        _blendmode: BlendMode,
        _alpha: f32,
    ) {
    }
    fn end_group(&mut self) {}
    fn begin_tile(
        &mut self,
        _area: Rect,
        _view: Rect,
        _xstep: f32,
        _ystep: f32,
        _ctm: &Matrix,
    ) -> i32 {
        0
    }
    fn end_tile(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graphics_state_default() {
        let gs = GraphicsState::new();
        assert_eq!(gs.ctm, Matrix::IDENTITY);
        assert_eq!(gs.alpha_fill, 1.0);
        assert_eq!(gs.alpha_stroke, 1.0);
    }

    #[test]
    fn test_resources_new() {
        let res = Resources::new();
        assert!(res.fonts.is_empty());
        assert!(res.xobjects.is_empty());
    }
}
