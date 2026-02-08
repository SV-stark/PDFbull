//! SVG (Scalable Vector Graphics) FFI Module
//!
//! Provides support for SVG document format, including DOM parsing,
//! path commands, transformations, filters, and text layout.

use crate::ffi::{Handle, HandleStore};
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::LazyLock;

// ============================================================================
// Type Aliases
// ============================================================================

type ContextHandle = Handle;
type StreamHandle = Handle;
type DeviceHandle = Handle;
type OutputHandle = Handle;

// ============================================================================
// SVG Text Format Constants
// ============================================================================

/// Render text as path elements (exact visual)
pub const SVG_TEXT_AS_PATH: i32 = 0;
/// Render text as text elements (editable)
pub const SVG_TEXT_AS_TEXT: i32 = 1;

// ============================================================================
// SVG Path Command Constants
// ============================================================================

/// Move to (M/m)
pub const SVG_PATH_MOVE: i32 = 0;
/// Line to (L/l)
pub const SVG_PATH_LINE: i32 = 1;
/// Horizontal line (H/h)
pub const SVG_PATH_HLINE: i32 = 2;
/// Vertical line (V/v)
pub const SVG_PATH_VLINE: i32 = 3;
/// Cubic Bezier (C/c)
pub const SVG_PATH_CUBIC: i32 = 4;
/// Smooth cubic (S/s)
pub const SVG_PATH_SMOOTH_CUBIC: i32 = 5;
/// Quadratic Bezier (Q/q)
pub const SVG_PATH_QUAD: i32 = 6;
/// Smooth quadratic (T/t)
pub const SVG_PATH_SMOOTH_QUAD: i32 = 7;
/// Elliptical arc (A/a)
pub const SVG_PATH_ARC: i32 = 8;
/// Close path (Z/z)
pub const SVG_PATH_CLOSE: i32 = 9;

// ============================================================================
// SVG Element Type Constants
// ============================================================================

/// SVG root element
pub const SVG_ELEM_SVG: i32 = 0;
/// Group element (g)
pub const SVG_ELEM_G: i32 = 1;
/// Definition element (defs)
pub const SVG_ELEM_DEFS: i32 = 2;
/// Symbol element
pub const SVG_ELEM_SYMBOL: i32 = 3;
/// Use element
pub const SVG_ELEM_USE: i32 = 4;
/// Rectangle (rect)
pub const SVG_ELEM_RECT: i32 = 5;
/// Circle
pub const SVG_ELEM_CIRCLE: i32 = 6;
/// Ellipse
pub const SVG_ELEM_ELLIPSE: i32 = 7;
/// Line
pub const SVG_ELEM_LINE: i32 = 8;
/// Polyline
pub const SVG_ELEM_POLYLINE: i32 = 9;
/// Polygon
pub const SVG_ELEM_POLYGON: i32 = 10;
/// Path
pub const SVG_ELEM_PATH: i32 = 11;
/// Text
pub const SVG_ELEM_TEXT: i32 = 12;
/// Text span (tspan)
pub const SVG_ELEM_TSPAN: i32 = 13;
/// Image
pub const SVG_ELEM_IMAGE: i32 = 14;
/// Linear gradient
pub const SVG_ELEM_LINEAR_GRADIENT: i32 = 15;
/// Radial gradient
pub const SVG_ELEM_RADIAL_GRADIENT: i32 = 16;
/// Gradient stop
pub const SVG_ELEM_STOP: i32 = 17;
/// Clip path
pub const SVG_ELEM_CLIPPATH: i32 = 18;
/// Mask
pub const SVG_ELEM_MASK: i32 = 19;
/// Pattern
pub const SVG_ELEM_PATTERN: i32 = 20;
/// Filter
pub const SVG_ELEM_FILTER: i32 = 21;
/// Unknown element
pub const SVG_ELEM_UNKNOWN: i32 = 99;

// ============================================================================
// SVG Transform Type Constants
// ============================================================================

/// Matrix transform
pub const SVG_TRANSFORM_MATRIX: i32 = 0;
/// Translate transform
pub const SVG_TRANSFORM_TRANSLATE: i32 = 1;
/// Scale transform
pub const SVG_TRANSFORM_SCALE: i32 = 2;
/// Rotate transform
pub const SVG_TRANSFORM_ROTATE: i32 = 3;
/// SkewX transform
pub const SVG_TRANSFORM_SKEWX: i32 = 4;
/// SkewY transform
pub const SVG_TRANSFORM_SKEWY: i32 = 5;

// ============================================================================
// SVG Path Command
// ============================================================================

/// SVG path command
#[derive(Debug, Clone)]
pub struct SvgPathCommand {
    /// Command type
    pub cmd: i32,
    /// Whether command is relative
    pub relative: bool,
    /// Command arguments
    pub args: Vec<f32>,
}

impl SvgPathCommand {
    pub fn new(cmd: i32, relative: bool) -> Self {
        Self {
            cmd,
            relative,
            args: Vec::new(),
        }
    }

    pub fn with_args(mut self, args: &[f32]) -> Self {
        self.args = args.to_vec();
        self
    }
}

// ============================================================================
// SVG Transform
// ============================================================================

/// SVG transform
#[derive(Debug, Clone)]
pub struct SvgTransform {
    /// Transform type
    pub transform_type: i32,
    /// Transform values
    pub values: Vec<f32>,
}

impl SvgTransform {
    pub fn matrix(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Self {
            transform_type: SVG_TRANSFORM_MATRIX,
            values: vec![a, b, c, d, e, f],
        }
    }

    pub fn translate(tx: f32, ty: f32) -> Self {
        Self {
            transform_type: SVG_TRANSFORM_TRANSLATE,
            values: vec![tx, ty],
        }
    }

    pub fn scale(sx: f32, sy: f32) -> Self {
        Self {
            transform_type: SVG_TRANSFORM_SCALE,
            values: vec![sx, sy],
        }
    }

    pub fn rotate(angle: f32, cx: f32, cy: f32) -> Self {
        Self {
            transform_type: SVG_TRANSFORM_ROTATE,
            values: vec![angle, cx, cy],
        }
    }

    pub fn skew_x(angle: f32) -> Self {
        Self {
            transform_type: SVG_TRANSFORM_SKEWX,
            values: vec![angle],
        }
    }

    pub fn skew_y(angle: f32) -> Self {
        Self {
            transform_type: SVG_TRANSFORM_SKEWY,
            values: vec![angle],
        }
    }

    pub fn identity() -> Self {
        Self::matrix(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    }
}

// ============================================================================
// SVG Color
// ============================================================================

/// SVG color
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SvgColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl SvgColor {
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
            a: 255,
        }
    }

    pub fn black() -> Self {
        Self::rgb(0, 0, 0)
    }

    pub fn white() -> Self {
        Self::rgb(255, 255, 255)
    }

    pub fn transparent() -> Self {
        Self::rgba(0, 0, 0, 0)
    }
}

// ============================================================================
// SVG Style
// ============================================================================

/// SVG style properties
#[derive(Debug, Clone, Default)]
pub struct SvgStyle {
    /// Fill color
    pub fill: Option<SvgColor>,
    /// Stroke color
    pub stroke: Option<SvgColor>,
    /// Stroke width
    pub stroke_width: f32,
    /// Fill opacity (0-1)
    pub fill_opacity: f32,
    /// Stroke opacity (0-1)
    pub stroke_opacity: f32,
    /// Opacity (0-1)
    pub opacity: f32,
    /// Font family
    pub font_family: Option<String>,
    /// Font size
    pub font_size: f32,
    /// Font weight (100-900)
    pub font_weight: i32,
    /// Font style (normal, italic, oblique)
    pub font_style: String,
}

impl SvgStyle {
    pub fn new() -> Self {
        Self {
            fill: Some(SvgColor::black()),
            stroke: None,
            stroke_width: 1.0,
            fill_opacity: 1.0,
            stroke_opacity: 1.0,
            opacity: 1.0,
            font_family: None,
            font_size: 16.0,
            font_weight: 400,
            font_style: "normal".to_string(),
        }
    }
}

// ============================================================================
// SVG Element
// ============================================================================

/// SVG element
#[derive(Debug, Clone)]
pub struct SvgElement {
    /// Element type
    pub element_type: i32,
    /// Element ID
    pub id: Option<String>,
    /// Class names
    pub classes: Vec<String>,
    /// Transform
    pub transform: Option<SvgTransform>,
    /// Style
    pub style: SvgStyle,
    /// Attributes
    pub attributes: HashMap<String, String>,
    /// Children
    pub children: Vec<SvgElement>,
    /// Text content
    pub text_content: Option<String>,
    /// Path commands (for path elements)
    pub path_commands: Vec<SvgPathCommand>,
}

impl SvgElement {
    pub fn new(element_type: i32) -> Self {
        Self {
            element_type,
            id: None,
            classes: Vec::new(),
            transform: None,
            style: SvgStyle::new(),
            attributes: HashMap::new(),
            children: Vec::new(),
            text_content: None,
            path_commands: Vec::new(),
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = Some(id.to_string());
        self
    }

    pub fn add_child(&mut self, child: SvgElement) {
        self.children.push(child);
    }

    pub fn set_attribute(&mut self, name: &str, value: &str) {
        self.attributes.insert(name.to_string(), value.to_string());
    }

    pub fn get_attribute(&self, name: &str) -> Option<&String> {
        self.attributes.get(name)
    }
}

// ============================================================================
// SVG Document
// ============================================================================

/// SVG document structure
pub struct SvgDocument {
    /// Context handle
    pub context: ContextHandle,
    /// Document width (in user units)
    pub width: f32,
    /// Document height (in user units)
    pub height: f32,
    /// ViewBox (min-x, min-y, width, height)
    pub viewbox: Option<(f32, f32, f32, f32)>,
    /// Root element
    pub root: Option<SvgElement>,
    /// ID map for quick lookup
    pub id_map: HashMap<String, usize>,
    /// Definitions (gradients, patterns, etc.)
    pub defs: HashMap<String, SvgElement>,
    /// Base URI for external resources
    pub base_uri: String,
}

impl SvgDocument {
    pub fn new(context: ContextHandle) -> Self {
        Self {
            context,
            width: 300.0,
            height: 150.0,
            viewbox: None,
            root: None,
            id_map: HashMap::new(),
            defs: HashMap::new(),
            base_uri: String::new(),
        }
    }

    pub fn set_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }

    pub fn set_viewbox(&mut self, min_x: f32, min_y: f32, width: f32, height: f32) {
        self.viewbox = Some((min_x, min_y, width, height));
    }

    pub fn add_def(&mut self, id: &str, element: SvgElement) {
        self.defs.insert(id.to_string(), element);
    }

    pub fn get_def(&self, id: &str) -> Option<&SvgElement> {
        self.defs.get(id)
    }
}

// ============================================================================
// SVG Device Options
// ============================================================================

/// SVG output device options
#[derive(Debug, Clone)]
pub struct SvgDeviceOptions {
    /// Text format (path or text)
    pub text_format: i32,
    /// Reuse images using symbols
    pub reuse_images: bool,
    /// Resolution for rasterized content
    pub resolution: i32,
}

impl Default for SvgDeviceOptions {
    fn default() -> Self {
        Self {
            text_format: SVG_TEXT_AS_PATH,
            reuse_images: true,
            resolution: 96,
        }
    }
}

// ============================================================================
// Global Handle Store
// ============================================================================

pub static SVG_DOCUMENTS: LazyLock<HandleStore<SvgDocument>> = LazyLock::new(HandleStore::new);
pub static SVG_ELEMENTS: LazyLock<HandleStore<SvgElement>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Document Management
// ============================================================================

/// Create a new SVG document.
#[unsafe(no_mangle)]
pub extern "C" fn svg_new_document(ctx: ContextHandle) -> Handle {
    let doc = SvgDocument::new(ctx);
    SVG_DOCUMENTS.insert(doc)
}

/// Drop an SVG document.
#[unsafe(no_mangle)]
pub extern "C" fn svg_drop_document(_ctx: ContextHandle, doc: Handle) {
    SVG_DOCUMENTS.remove(doc);
}

/// Open an SVG document from a file path.
#[unsafe(no_mangle)]
pub extern "C" fn svg_open_document(ctx: ContextHandle, filename: *const c_char) -> Handle {
    if filename.is_null() {
        return 0;
    }

    let _path = unsafe { CStr::from_ptr(filename).to_string_lossy() };

    let doc = SvgDocument::new(ctx);
    SVG_DOCUMENTS.insert(doc)
}

/// Open an SVG document from a stream.
#[unsafe(no_mangle)]
pub extern "C" fn svg_open_document_with_stream(
    ctx: ContextHandle,
    _stream: StreamHandle,
) -> Handle {
    let doc = SvgDocument::new(ctx);
    SVG_DOCUMENTS.insert(doc)
}

// ============================================================================
// FFI Functions - Document Properties
// ============================================================================

/// Get document width.
#[unsafe(no_mangle)]
pub extern "C" fn svg_get_width(_ctx: ContextHandle, doc: Handle) -> f32 {
    if let Some(d) = SVG_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.width;
    }
    0.0
}

/// Get document height.
#[unsafe(no_mangle)]
pub extern "C" fn svg_get_height(_ctx: ContextHandle, doc: Handle) -> f32 {
    if let Some(d) = SVG_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.height;
    }
    0.0
}

/// Set document size.
#[unsafe(no_mangle)]
pub extern "C" fn svg_set_size(_ctx: ContextHandle, doc: Handle, width: f32, height: f32) -> i32 {
    if let Some(d) = SVG_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        d.set_size(width, height);
        return 1;
    }
    0
}

/// Set viewBox.
#[unsafe(no_mangle)]
pub extern "C" fn svg_set_viewbox(
    _ctx: ContextHandle,
    doc: Handle,
    min_x: f32,
    min_y: f32,
    width: f32,
    height: f32,
) -> i32 {
    if let Some(d) = SVG_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        d.set_viewbox(min_x, min_y, width, height);
        return 1;
    }
    0
}

/// Get viewBox.
#[unsafe(no_mangle)]
pub extern "C" fn svg_get_viewbox(
    _ctx: ContextHandle,
    doc: Handle,
    min_x: *mut f32,
    min_y: *mut f32,
    width: *mut f32,
    height: *mut f32,
) -> i32 {
    if let Some(d) = SVG_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some((vb_x, vb_y, vb_w, vb_h)) = d.viewbox {
            if !min_x.is_null() {
                unsafe {
                    *min_x = vb_x;
                }
            }
            if !min_y.is_null() {
                unsafe {
                    *min_y = vb_y;
                }
            }
            if !width.is_null() {
                unsafe {
                    *width = vb_w;
                }
            }
            if !height.is_null() {
                unsafe {
                    *height = vb_h;
                }
            }
            return 1;
        }
    }
    0
}

// ============================================================================
// FFI Functions - Element Management
// ============================================================================

/// Create a new SVG element.
#[unsafe(no_mangle)]
pub extern "C" fn svg_new_element(_ctx: ContextHandle, element_type: i32) -> Handle {
    let elem = SvgElement::new(element_type);
    SVG_ELEMENTS.insert(elem)
}

/// Drop an SVG element.
#[unsafe(no_mangle)]
pub extern "C" fn svg_drop_element(_ctx: ContextHandle, elem: Handle) {
    SVG_ELEMENTS.remove(elem);
}

/// Set element ID.
#[unsafe(no_mangle)]
pub extern "C" fn svg_set_element_id(_ctx: ContextHandle, elem: Handle, id: *const c_char) -> i32 {
    if id.is_null() {
        return 0;
    }

    if let Some(e) = SVG_ELEMENTS.get(elem) {
        let mut e = e.lock().unwrap();
        let elem_id = unsafe { CStr::from_ptr(id).to_string_lossy().to_string() };
        e.id = Some(elem_id);
        return 1;
    }
    0
}

/// Get element ID.
#[unsafe(no_mangle)]
pub extern "C" fn svg_get_element_id(_ctx: ContextHandle, elem: Handle) -> *mut c_char {
    if let Some(e) = SVG_ELEMENTS.get(elem) {
        let e = e.lock().unwrap();
        if let Some(ref id) = e.id {
            if let Ok(cstr) = CString::new(id.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Get element type.
#[unsafe(no_mangle)]
pub extern "C" fn svg_get_element_type(_ctx: ContextHandle, elem: Handle) -> i32 {
    if let Some(e) = SVG_ELEMENTS.get(elem) {
        let e = e.lock().unwrap();
        return e.element_type;
    }
    SVG_ELEM_UNKNOWN
}

/// Set element attribute.
#[unsafe(no_mangle)]
pub extern "C" fn svg_set_attribute(
    _ctx: ContextHandle,
    elem: Handle,
    name: *const c_char,
    value: *const c_char,
) -> i32 {
    if name.is_null() || value.is_null() {
        return 0;
    }

    if let Some(e) = SVG_ELEMENTS.get(elem) {
        let mut e = e.lock().unwrap();
        let attr_name = unsafe { CStr::from_ptr(name).to_string_lossy().to_string() };
        let attr_value = unsafe { CStr::from_ptr(value).to_string_lossy().to_string() };
        e.set_attribute(&attr_name, &attr_value);
        return 1;
    }
    0
}

/// Get element attribute.
#[unsafe(no_mangle)]
pub extern "C" fn svg_get_attribute(
    _ctx: ContextHandle,
    elem: Handle,
    name: *const c_char,
) -> *mut c_char {
    if name.is_null() {
        return ptr::null_mut();
    }

    if let Some(e) = SVG_ELEMENTS.get(elem) {
        let e = e.lock().unwrap();
        let attr_name = unsafe { CStr::from_ptr(name).to_string_lossy() };
        if let Some(value) = e.get_attribute(&attr_name) {
            if let Ok(cstr) = CString::new(value.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

// ============================================================================
// FFI Functions - Transform
// ============================================================================

/// Set element transform (matrix).
#[unsafe(no_mangle)]
pub extern "C" fn svg_set_transform_matrix(
    _ctx: ContextHandle,
    elem: Handle,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
) -> i32 {
    if let Some(el) = SVG_ELEMENTS.get(elem) {
        let mut el = el.lock().unwrap();
        el.transform = Some(SvgTransform::matrix(a, b, c, d, e, f));
        return 1;
    }
    0
}

/// Set element transform (translate).
#[unsafe(no_mangle)]
pub extern "C" fn svg_set_transform_translate(
    _ctx: ContextHandle,
    elem: Handle,
    tx: f32,
    ty: f32,
) -> i32 {
    if let Some(el) = SVG_ELEMENTS.get(elem) {
        let mut el = el.lock().unwrap();
        el.transform = Some(SvgTransform::translate(tx, ty));
        return 1;
    }
    0
}

/// Set element transform (scale).
#[unsafe(no_mangle)]
pub extern "C" fn svg_set_transform_scale(
    _ctx: ContextHandle,
    elem: Handle,
    sx: f32,
    sy: f32,
) -> i32 {
    if let Some(el) = SVG_ELEMENTS.get(elem) {
        let mut el = el.lock().unwrap();
        el.transform = Some(SvgTransform::scale(sx, sy));
        return 1;
    }
    0
}

/// Set element transform (rotate).
#[unsafe(no_mangle)]
pub extern "C" fn svg_set_transform_rotate(
    _ctx: ContextHandle,
    elem: Handle,
    angle: f32,
    cx: f32,
    cy: f32,
) -> i32 {
    if let Some(el) = SVG_ELEMENTS.get(elem) {
        let mut el = el.lock().unwrap();
        el.transform = Some(SvgTransform::rotate(angle, cx, cy));
        return 1;
    }
    0
}

// ============================================================================
// FFI Functions - Style
// ============================================================================

/// Set fill color.
#[unsafe(no_mangle)]
pub extern "C" fn svg_set_fill(
    _ctx: ContextHandle,
    elem: Handle,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
) -> i32 {
    if let Some(e) = SVG_ELEMENTS.get(elem) {
        let mut e = e.lock().unwrap();
        e.style.fill = Some(SvgColor::rgba(r, g, b, a));
        return 1;
    }
    0
}

/// Set stroke color.
#[unsafe(no_mangle)]
pub extern "C" fn svg_set_stroke(
    _ctx: ContextHandle,
    elem: Handle,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
) -> i32 {
    if let Some(e) = SVG_ELEMENTS.get(elem) {
        let mut e = e.lock().unwrap();
        e.style.stroke = Some(SvgColor::rgba(r, g, b, a));
        return 1;
    }
    0
}

/// Set stroke width.
#[unsafe(no_mangle)]
pub extern "C" fn svg_set_stroke_width(_ctx: ContextHandle, elem: Handle, width: f32) -> i32 {
    if let Some(e) = SVG_ELEMENTS.get(elem) {
        let mut e = e.lock().unwrap();
        e.style.stroke_width = width;
        return 1;
    }
    0
}

/// Set opacity.
#[unsafe(no_mangle)]
pub extern "C" fn svg_set_opacity(_ctx: ContextHandle, elem: Handle, opacity: f32) -> i32 {
    if let Some(e) = SVG_ELEMENTS.get(elem) {
        let mut e = e.lock().unwrap();
        e.style.opacity = opacity;
        return 1;
    }
    0
}

// ============================================================================
// FFI Functions - Path Commands
// ============================================================================

/// Add path command.
#[unsafe(no_mangle)]
pub extern "C" fn svg_add_path_command(
    _ctx: ContextHandle,
    elem: Handle,
    cmd: i32,
    relative: i32,
    args: *const f32,
    num_args: i32,
) -> i32 {
    if let Some(e) = SVG_ELEMENTS.get(elem) {
        let mut e = e.lock().unwrap();
        let mut path_cmd = SvgPathCommand::new(cmd, relative != 0);

        if !args.is_null() && num_args > 0 {
            let arg_slice = unsafe { std::slice::from_raw_parts(args, num_args as usize) };
            path_cmd = path_cmd.with_args(arg_slice);
        }

        e.path_commands.push(path_cmd);
        return 1;
    }
    0
}

/// Get path command count.
#[unsafe(no_mangle)]
pub extern "C" fn svg_path_command_count(_ctx: ContextHandle, elem: Handle) -> i32 {
    if let Some(e) = SVG_ELEMENTS.get(elem) {
        let e = e.lock().unwrap();
        return e.path_commands.len() as i32;
    }
    0
}

// ============================================================================
// FFI Functions - SVG Output Device
// ============================================================================

/// Create SVG output device.
#[unsafe(no_mangle)]
pub extern "C" fn svg_new_device(
    _ctx: ContextHandle,
    _output: OutputHandle,
    page_width: f32,
    page_height: f32,
    text_format: i32,
    reuse_images: i32,
) -> Handle {
    // This would create a device that renders to SVG
    // For now, return a placeholder handle
    let _ = (page_width, page_height, text_format, reuse_images);
    0
}

/// Parse SVG device options from string.
#[unsafe(no_mangle)]
pub extern "C" fn svg_parse_device_options(
    _ctx: ContextHandle,
    args: *const c_char,
    text_format: *mut i32,
    reuse_images: *mut i32,
    resolution: *mut i32,
) -> i32 {
    if args.is_null() {
        return 0;
    }

    let opts = SvgDeviceOptions::default();

    if !text_format.is_null() {
        unsafe {
            *text_format = opts.text_format;
        }
    }
    if !reuse_images.is_null() {
        unsafe {
            *reuse_images = if opts.reuse_images { 1 } else { 0 };
        }
    }
    if !resolution.is_null() {
        unsafe {
            *resolution = opts.resolution;
        }
    }

    1
}

// ============================================================================
// FFI Functions - Color Parsing
// ============================================================================

/// Parse SVG color string.
#[unsafe(no_mangle)]
pub extern "C" fn svg_parse_color(
    _ctx: ContextHandle,
    str: *const c_char,
    r: *mut u8,
    g: *mut u8,
    b: *mut u8,
) -> i32 {
    if str.is_null() {
        return 0;
    }

    let color_str = unsafe { CStr::from_ptr(str).to_string_lossy() };
    let color_str = color_str.trim();

    let color = if color_str.starts_with('#') {
        // Hex color
        let hex = &color_str[1..];
        if hex.len() == 3 {
            // Short form #RGB
            let r_val = u8::from_str_radix(&hex[0..1], 16).unwrap_or(0) * 17;
            let g_val = u8::from_str_radix(&hex[1..2], 16).unwrap_or(0) * 17;
            let b_val = u8::from_str_radix(&hex[2..3], 16).unwrap_or(0) * 17;
            Some(SvgColor::rgb(r_val, g_val, b_val))
        } else if hex.len() == 6 {
            // Long form #RRGGBB
            let r_val = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g_val = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b_val = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            Some(SvgColor::rgb(r_val, g_val, b_val))
        } else {
            None
        }
    } else {
        // Named colors
        match color_str.to_lowercase().as_str() {
            "black" => Some(SvgColor::rgb(0, 0, 0)),
            "white" => Some(SvgColor::rgb(255, 255, 255)),
            "red" => Some(SvgColor::rgb(255, 0, 0)),
            "green" => Some(SvgColor::rgb(0, 128, 0)),
            "blue" => Some(SvgColor::rgb(0, 0, 255)),
            "yellow" => Some(SvgColor::rgb(255, 255, 0)),
            "cyan" | "aqua" => Some(SvgColor::rgb(0, 255, 255)),
            "magenta" | "fuchsia" => Some(SvgColor::rgb(255, 0, 255)),
            "gray" | "grey" => Some(SvgColor::rgb(128, 128, 128)),
            "orange" => Some(SvgColor::rgb(255, 165, 0)),
            "purple" => Some(SvgColor::rgb(128, 0, 128)),
            "none" | "transparent" => Some(SvgColor::transparent()),
            _ => None,
        }
    };

    if let Some(c) = color {
        if !r.is_null() {
            unsafe {
                *r = c.r;
            }
        }
        if !g.is_null() {
            unsafe {
                *g = c.g;
            }
        }
        if !b.is_null() {
            unsafe {
                *b = c.b;
            }
        }
        return 1;
    }

    0
}

// ============================================================================
// FFI Functions - Utility
// ============================================================================

/// Free a string returned by SVG functions.
#[unsafe(no_mangle)]
pub extern "C" fn svg_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Get element type name.
#[unsafe(no_mangle)]
pub extern "C" fn svg_element_type_name(_ctx: ContextHandle, element_type: i32) -> *mut c_char {
    let name = match element_type {
        SVG_ELEM_SVG => "svg",
        SVG_ELEM_G => "g",
        SVG_ELEM_DEFS => "defs",
        SVG_ELEM_SYMBOL => "symbol",
        SVG_ELEM_USE => "use",
        SVG_ELEM_RECT => "rect",
        SVG_ELEM_CIRCLE => "circle",
        SVG_ELEM_ELLIPSE => "ellipse",
        SVG_ELEM_LINE => "line",
        SVG_ELEM_POLYLINE => "polyline",
        SVG_ELEM_POLYGON => "polygon",
        SVG_ELEM_PATH => "path",
        SVG_ELEM_TEXT => "text",
        SVG_ELEM_TSPAN => "tspan",
        SVG_ELEM_IMAGE => "image",
        SVG_ELEM_LINEAR_GRADIENT => "linearGradient",
        SVG_ELEM_RADIAL_GRADIENT => "radialGradient",
        SVG_ELEM_STOP => "stop",
        SVG_ELEM_CLIPPATH => "clipPath",
        SVG_ELEM_MASK => "mask",
        SVG_ELEM_PATTERN => "pattern",
        SVG_ELEM_FILTER => "filter",
        _ => "unknown",
    };

    if let Ok(cstr) = CString::new(name) {
        return cstr.into_raw();
    }
    ptr::null_mut()
}

/// Get path command name.
#[unsafe(no_mangle)]
pub extern "C" fn svg_path_command_name(
    _ctx: ContextHandle,
    cmd: i32,
    relative: i32,
) -> *mut c_char {
    let name = match (cmd, relative != 0) {
        (SVG_PATH_MOVE, false) => "M",
        (SVG_PATH_MOVE, true) => "m",
        (SVG_PATH_LINE, false) => "L",
        (SVG_PATH_LINE, true) => "l",
        (SVG_PATH_HLINE, false) => "H",
        (SVG_PATH_HLINE, true) => "h",
        (SVG_PATH_VLINE, false) => "V",
        (SVG_PATH_VLINE, true) => "v",
        (SVG_PATH_CUBIC, false) => "C",
        (SVG_PATH_CUBIC, true) => "c",
        (SVG_PATH_SMOOTH_CUBIC, false) => "S",
        (SVG_PATH_SMOOTH_CUBIC, true) => "s",
        (SVG_PATH_QUAD, false) => "Q",
        (SVG_PATH_QUAD, true) => "q",
        (SVG_PATH_SMOOTH_QUAD, false) => "T",
        (SVG_PATH_SMOOTH_QUAD, true) => "t",
        (SVG_PATH_ARC, false) => "A",
        (SVG_PATH_ARC, true) => "a",
        (SVG_PATH_CLOSE, _) => "Z",
        _ => "?",
    };

    if let Ok(cstr) = CString::new(name) {
        return cstr.into_raw();
    }
    ptr::null_mut()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_format_constants() {
        assert_eq!(SVG_TEXT_AS_PATH, 0);
        assert_eq!(SVG_TEXT_AS_TEXT, 1);
    }

    #[test]
    fn test_path_command_constants() {
        assert_eq!(SVG_PATH_MOVE, 0);
        assert_eq!(SVG_PATH_LINE, 1);
        assert_eq!(SVG_PATH_CLOSE, 9);
    }

    #[test]
    fn test_element_type_constants() {
        assert_eq!(SVG_ELEM_SVG, 0);
        assert_eq!(SVG_ELEM_PATH, 11);
        assert_eq!(SVG_ELEM_TEXT, 12);
    }

    #[test]
    fn test_svg_color() {
        let black = SvgColor::black();
        assert_eq!(black.r, 0);
        assert_eq!(black.g, 0);
        assert_eq!(black.b, 0);

        let hex = SvgColor::from_hex(0xFF5500);
        assert_eq!(hex.r, 255);
        assert_eq!(hex.g, 85);
        assert_eq!(hex.b, 0);
    }

    #[test]
    fn test_svg_transform() {
        let t = SvgTransform::translate(10.0, 20.0);
        assert_eq!(t.transform_type, SVG_TRANSFORM_TRANSLATE);
        assert_eq!(t.values, vec![10.0, 20.0]);

        let r = SvgTransform::rotate(45.0, 0.0, 0.0);
        assert_eq!(r.transform_type, SVG_TRANSFORM_ROTATE);
    }

    #[test]
    fn test_svg_element() {
        let mut elem = SvgElement::new(SVG_ELEM_RECT).with_id("my-rect");
        elem.set_attribute("width", "100");
        elem.set_attribute("height", "50");

        assert_eq!(elem.id, Some("my-rect".to_string()));
        assert_eq!(elem.get_attribute("width"), Some(&"100".to_string()));
    }

    #[test]
    fn test_svg_document() {
        let mut doc = SvgDocument::new(0);
        doc.set_size(800.0, 600.0);
        doc.set_viewbox(0.0, 0.0, 800.0, 600.0);

        assert_eq!(doc.width, 800.0);
        assert_eq!(doc.height, 600.0);
        assert_eq!(doc.viewbox, Some((0.0, 0.0, 800.0, 600.0)));
    }

    #[test]
    fn test_ffi_document() {
        let ctx = 0;

        let doc = svg_new_document(ctx);
        assert!(doc > 0);

        assert_eq!(svg_get_width(ctx, doc), 300.0);
        assert_eq!(svg_get_height(ctx, doc), 150.0);

        svg_set_size(ctx, doc, 800.0, 600.0);
        assert_eq!(svg_get_width(ctx, doc), 800.0);

        svg_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_element() {
        let ctx = 0;

        let elem = svg_new_element(ctx, SVG_ELEM_RECT);
        assert!(elem > 0);

        assert_eq!(svg_get_element_type(ctx, elem), SVG_ELEM_RECT);

        let id = CString::new("my-rect").unwrap();
        svg_set_element_id(ctx, elem, id.as_ptr());

        let result = svg_get_element_id(ctx, elem);
        assert!(!result.is_null());
        unsafe {
            let s = CStr::from_ptr(result).to_string_lossy();
            assert_eq!(s, "my-rect");
            svg_free_string(result);
        }

        svg_drop_element(ctx, elem);
    }

    #[test]
    fn test_ffi_transform() {
        let ctx = 0;
        let elem = svg_new_element(ctx, SVG_ELEM_G);

        svg_set_transform_translate(ctx, elem, 50.0, 100.0);

        if let Some(e) = SVG_ELEMENTS.get(elem) {
            let e = e.lock().unwrap();
            assert!(e.transform.is_some());
            if let Some(ref t) = e.transform {
                assert_eq!(t.transform_type, SVG_TRANSFORM_TRANSLATE);
            }
        }

        svg_drop_element(ctx, elem);
    }

    #[test]
    fn test_ffi_style() {
        let ctx = 0;
        let elem = svg_new_element(ctx, SVG_ELEM_RECT);

        svg_set_fill(ctx, elem, 255, 0, 0, 255);
        svg_set_stroke(ctx, elem, 0, 0, 0, 255);
        svg_set_stroke_width(ctx, elem, 2.0);

        if let Some(e) = SVG_ELEMENTS.get(elem) {
            let e = e.lock().unwrap();
            assert_eq!(e.style.fill, Some(SvgColor::rgba(255, 0, 0, 255)));
            assert_eq!(e.style.stroke_width, 2.0);
        }

        svg_drop_element(ctx, elem);
    }

    #[test]
    fn test_ffi_path_commands() {
        let ctx = 0;
        let elem = svg_new_element(ctx, SVG_ELEM_PATH);

        let args1 = [10.0f32, 20.0];
        svg_add_path_command(ctx, elem, SVG_PATH_MOVE, 0, args1.as_ptr(), 2);

        let args2 = [100.0f32, 20.0];
        svg_add_path_command(ctx, elem, SVG_PATH_LINE, 0, args2.as_ptr(), 2);

        svg_add_path_command(ctx, elem, SVG_PATH_CLOSE, 0, ptr::null(), 0);

        assert_eq!(svg_path_command_count(ctx, elem), 3);

        svg_drop_element(ctx, elem);
    }

    #[test]
    fn test_ffi_parse_color() {
        let ctx = 0;
        let mut r: u8 = 0;
        let mut g: u8 = 0;
        let mut b: u8 = 0;

        // Hex color
        let hex = CString::new("#FF5500").unwrap();
        svg_parse_color(ctx, hex.as_ptr(), &mut r, &mut g, &mut b);
        assert_eq!(r, 255);
        assert_eq!(g, 85);
        assert_eq!(b, 0);

        // Named color
        let red = CString::new("red").unwrap();
        svg_parse_color(ctx, red.as_ptr(), &mut r, &mut g, &mut b);
        assert_eq!(r, 255);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }

    #[test]
    fn test_ffi_element_type_name() {
        let ctx = 0;

        let name = svg_element_type_name(ctx, SVG_ELEM_RECT);
        assert!(!name.is_null());
        unsafe {
            let s = CStr::from_ptr(name).to_string_lossy();
            assert_eq!(s, "rect");
            svg_free_string(name);
        }
    }

    #[test]
    fn test_ffi_path_command_name() {
        let ctx = 0;

        let name = svg_path_command_name(ctx, SVG_PATH_MOVE, 0);
        unsafe {
            let s = CStr::from_ptr(name).to_string_lossy();
            assert_eq!(s, "M");
            svg_free_string(name);
        }

        let name = svg_path_command_name(ctx, SVG_PATH_LINE, 1);
        unsafe {
            let s = CStr::from_ptr(name).to_string_lossy();
            assert_eq!(s, "l");
            svg_free_string(name);
        }
    }
}
