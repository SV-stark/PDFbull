//! FFI bindings for pdf_interpret (Content Stream Processor)
//!
//! Provides PDF content stream processing capabilities:
//! - Processor base with operator callbacks
//! - Run processor (rendering)
//! - Buffer processor (collect to buffer)
//! - Output processor (write to stream)
//! - Sanitize filter (clean content)
//! - Color filter (recolor content)
//! - Content stream processing

use crate::ffi::ffi_safety::{cstr_to_str, raw_to_slice, write_out};
use crate::ffi::{Handle, HandleStore};
use crate::fitz::geometry::{Matrix, Rect};
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_void};
use std::ptr;
use std::sync::LazyLock;

// ============================================================================
// Types and Enums
// ============================================================================

/// PDF processor requirements flags
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessorRequirements {
    #[default]
    None = 0,
    /// Processor requires decoded images
    RequiresDecodedImages = 1,
}

/// Cull types for content filtering
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CullType {
    #[default]
    PathDrop = 0,
    PathFill = 1,
    PathStroke = 2,
    PathFillStroke = 3,
    ClipPathDrop = 4,
    ClipPathFill = 5,
    ClipPathStroke = 6,
    ClipPathFillStroke = 7,
    Glyph = 8,
    Image = 9,
    Shading = 10,
}

impl From<i32> for CullType {
    fn from(value: i32) -> Self {
        match value {
            0 => CullType::PathDrop,
            1 => CullType::PathFill,
            2 => CullType::PathStroke,
            3 => CullType::PathFillStroke,
            4 => CullType::ClipPathDrop,
            5 => CullType::ClipPathFill,
            6 => CullType::ClipPathStroke,
            7 => CullType::ClipPathFillStroke,
            8 => CullType::Glyph,
            9 => CullType::Image,
            10 => CullType::Shading,
            _ => CullType::PathDrop,
        }
    }
}

/// Processor type
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessorType {
    #[default]
    Base = 0,
    Run = 1,
    Buffer = 2,
    Output = 3,
    Sanitize = 4,
    Color = 5,
    Vectorize = 6,
}

// ============================================================================
// Graphics State
// ============================================================================

/// Graphics state for PDF processing
#[derive(Debug, Clone, Default)]
pub struct PdfGstate {
    /// Current transformation matrix
    pub ctm: Matrix,
    /// Line width
    pub line_width: f32,
    /// Line join style (0=miter, 1=round, 2=bevel)
    pub line_join: i32,
    /// Line cap style (0=butt, 1=round, 2=square)
    pub line_cap: i32,
    /// Miter limit
    pub miter_limit: f32,
    /// Flatness
    pub flatness: f32,
    /// Dash array
    pub dash_array: Vec<f32>,
    /// Dash phase
    pub dash_phase: f32,
    /// Rendering intent
    pub rendering_intent: String,
    /// Blend mode
    pub blend_mode: String,
    /// Stroke alpha
    pub stroke_alpha: f32,
    /// Fill alpha
    pub fill_alpha: f32,
    /// Stroke color
    pub stroke_color: Vec<f32>,
    /// Fill color
    pub fill_color: Vec<f32>,
    /// Stroke colorspace name
    pub stroke_cs: String,
    /// Fill colorspace name
    pub fill_cs: String,
    /// Overprint stroke
    pub op_stroke: bool,
    /// Overprint fill
    pub op_fill: bool,
    /// Overprint mode
    pub opm: i32,
}

impl PdfGstate {
    pub fn new() -> Self {
        Self {
            ctm: Matrix::IDENTITY,
            line_width: 1.0,
            line_join: 0,
            line_cap: 0,
            miter_limit: 10.0,
            flatness: 1.0,
            dash_array: Vec::new(),
            dash_phase: 0.0,
            rendering_intent: "RelativeColorimetric".to_string(),
            blend_mode: "Normal".to_string(),
            stroke_alpha: 1.0,
            fill_alpha: 1.0,
            stroke_color: vec![0.0],
            fill_color: vec![0.0],
            stroke_cs: "DeviceGray".to_string(),
            fill_cs: "DeviceGray".to_string(),
            op_stroke: false,
            op_fill: false,
            opm: 0,
        }
    }
}

// ============================================================================
// Path State
// ============================================================================

/// Path element types
#[derive(Debug, Clone)]
pub enum PathElement {
    MoveTo {
        x: f32,
        y: f32,
    },
    LineTo {
        x: f32,
        y: f32,
    },
    CurveTo {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x3: f32,
        y3: f32,
    },
    CurveV {
        x2: f32,
        y2: f32,
        x3: f32,
        y3: f32,
    },
    CurveY {
        x1: f32,
        y1: f32,
        x3: f32,
        y3: f32,
    },
    ClosePath,
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
}

/// Current path being constructed
#[derive(Debug, Clone, Default)]
pub struct PathState {
    pub elements: Vec<PathElement>,
}

// ============================================================================
// Text State
// ============================================================================

/// Text state for PDF processing
#[derive(Debug, Clone, Default)]
pub struct PdfTextState {
    /// Character spacing
    pub char_space: f32,
    /// Word spacing
    pub word_space: f32,
    /// Horizontal scaling (percentage)
    pub scale: f32,
    /// Leading
    pub leading: f32,
    /// Font name
    pub font_name: String,
    /// Font handle
    pub font: Handle,
    /// Font size
    pub size: f32,
    /// Text rendering mode (0-7)
    pub render: i32,
    /// Text rise
    pub rise: f32,
}

impl PdfTextState {
    pub fn new() -> Self {
        Self {
            char_space: 0.0,
            word_space: 0.0,
            scale: 100.0,
            leading: 0.0,
            font_name: String::new(),
            font: 0,
            size: 12.0,
            render: 0,
            rise: 0.0,
        }
    }
}

/// Text object state
#[derive(Debug, Clone, Default)]
pub struct PdfTextObjectState {
    /// Text line matrix
    pub tlm: Matrix,
    /// Text matrix
    pub tm: Matrix,
    /// Text mode
    pub text_mode: i32,
    /// Accumulated text
    pub text_content: Vec<TextSpan>,
    /// Text bounding box
    pub text_bbox: Rect,
}

/// A span of text with positioning
#[derive(Debug, Clone)]
pub struct TextSpan {
    pub text: String,
    pub matrix: Matrix,
    pub font_name: String,
    pub font_size: f32,
}

// ============================================================================
// Resource Stack
// ============================================================================

/// Resource stack for content stream interpretation
#[derive(Debug, Clone, Default)]
pub struct ResourceStack {
    pub resources: Vec<Handle>,
}

// ============================================================================
// Processor Structure
// ============================================================================

/// PDF processor for content stream interpretation
#[derive(Default)]
pub struct PdfProcessor {
    /// Reference count
    pub refs: i32,
    /// Whether processor is closed
    pub closed: bool,
    /// Processor type
    pub proc_type: ProcessorType,
    /// Requirements
    pub requirements: ProcessorRequirements,
    /// Usage string (e.g., "View", "Print")
    pub usage: String,
    /// Hidden content handling
    pub hidden: i32,
    /// Resource stack
    pub rstack: ResourceStack,
    /// Chained processor
    pub chain: Option<Handle>,
    /// Graphics state stack
    pub gstate_stack: Vec<PdfGstate>,
    /// Current graphics state
    pub gstate: PdfGstate,
    /// Current path
    pub path: PathState,
    /// Text state
    pub text_state: PdfTextState,
    /// Text object state
    pub text_object: Option<PdfTextObjectState>,
    /// In text object (between BT and ET)
    pub in_text: bool,
    /// Output buffer (for buffer processor)
    pub output_buffer: Option<Handle>,
    /// Output stream (for output processor)
    pub output_stream: Option<Handle>,
    /// Device (for run processor)
    pub device: Option<Handle>,
    /// Document reference
    pub document: Option<Handle>,
    /// ASCII hex encode images
    pub ahx_encode: bool,
    /// Add newlines after operators
    pub newlines: bool,
    /// Collected operators (for debugging/inspection)
    pub operators: Vec<ProcessedOperator>,
    /// Struct parent
    pub struct_parent: i32,
    /// Transform
    pub transform: Matrix,
}

/// A processed operator with its arguments
#[derive(Debug, Clone)]
pub struct ProcessedOperator {
    pub name: String,
    pub args: Vec<OperatorArg>,
}

/// Operator argument types
#[derive(Debug, Clone)]
pub enum OperatorArg {
    Number(f32),
    Integer(i32),
    String(String),
    Name(String),
    Array(Vec<OperatorArg>),
    Boolean(bool),
}

// ============================================================================
// Global State
// ============================================================================

pub static PDF_PROCESSORS: LazyLock<HandleStore<PdfProcessor>> = LazyLock::new(HandleStore::new);
pub static PDF_GSTATES: LazyLock<HandleStore<PdfGstate>> = LazyLock::new(HandleStore::new);

// ============================================================================
// Processor Implementation
// ============================================================================

impl PdfProcessor {
    pub fn new(proc_type: ProcessorType) -> Self {
        Self {
            refs: 1,
            closed: false,
            proc_type,
            gstate: PdfGstate::new(),
            gstate_stack: Vec::new(),
            text_state: PdfTextState::new(),
            ..Default::default()
        }
    }

    /// Push current graphics state
    pub fn push_gstate(&mut self) {
        self.gstate_stack.push(self.gstate.clone());
    }

    /// Pop graphics state
    pub fn pop_gstate(&mut self) -> bool {
        if let Some(gstate) = self.gstate_stack.pop() {
            self.gstate = gstate;
            true
        } else {
            false
        }
    }

    /// Record an operator
    pub fn record_op(&mut self, name: &str, args: Vec<OperatorArg>) {
        self.operators.push(ProcessedOperator {
            name: name.to_string(),
            args,
        });
    }

    /// Write operator to output
    pub fn write_op(&self, _name: &str, _args: &[OperatorArg]) {
        // Would write to output_buffer or output_stream
    }
}

// ============================================================================
// FFI Functions - Processor Lifecycle
// ============================================================================

/// Create a new processor
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_processor(_ctx: Handle, size: i32) -> Handle {
    let proc_type = match size {
        1 => ProcessorType::Run,
        2 => ProcessorType::Buffer,
        3 => ProcessorType::Output,
        4 => ProcessorType::Sanitize,
        5 => ProcessorType::Color,
        6 => ProcessorType::Vectorize,
        _ => ProcessorType::Base,
    };
    let processor = PdfProcessor::new(proc_type);
    PDF_PROCESSORS.insert(processor)
}

/// Keep (increment ref count) a processor
#[unsafe(no_mangle)]
pub extern "C" fn pdf_keep_processor(_ctx: Handle, proc: Handle) -> Handle {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.refs += 1;
    }
    proc
}

/// Close a processor
#[unsafe(no_mangle)]
pub extern "C" fn pdf_close_processor(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.closed = true;

        // Close chained processor if any
        if let Some(chain) = proc_guard.chain {
            drop(proc_guard);
            pdf_close_processor(_ctx, chain);
        }
    }
}

/// Drop a processor
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_processor(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.refs -= 1;
        if proc_guard.refs <= 0 {
            // Drop chained processor
            let chain = proc_guard.chain;
            drop(proc_guard);

            if let Some(chain_handle) = chain {
                pdf_drop_processor(_ctx, chain_handle);
            }
            PDF_PROCESSORS.remove(proc);
        }
    }
}

/// Reset a processor for reuse
#[unsafe(no_mangle)]
pub extern "C" fn pdf_reset_processor(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.closed = false;
        proc_guard.gstate = PdfGstate::new();
        proc_guard.gstate_stack.clear();
        proc_guard.path = PathState::default();
        proc_guard.text_state = PdfTextState::new();
        proc_guard.text_object = None;
        proc_guard.in_text = false;
        proc_guard.operators.clear();
    }
}

// ============================================================================
// FFI Functions - Processor Factories
// ============================================================================

/// Create a run processor for rendering
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_run_processor(
    _ctx: Handle,
    doc: Handle,
    dev: Handle,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
    struct_parent: i32,
    usage: *const c_char,
) -> Handle {
    let mut processor = PdfProcessor::new(ProcessorType::Run);
    processor.document = Some(doc);
    processor.device = Some(dev);
    processor.transform = Matrix { a, b, c, d, e, f };
    processor.struct_parent = struct_parent;

    if !usage.is_null() {
        if let Ok(s) = unsafe { CStr::from_ptr(usage) }.to_str() {
            processor.usage = s.to_string();
        }
    }

    PDF_PROCESSORS.insert(processor)
}

/// Create a buffer processor
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_buffer_processor(
    _ctx: Handle,
    buffer: Handle,
    ahx_encode: i32,
    newlines: i32,
) -> Handle {
    let mut processor = PdfProcessor::new(ProcessorType::Buffer);
    processor.output_buffer = Some(buffer);
    processor.ahx_encode = ahx_encode != 0;
    processor.newlines = newlines != 0;

    PDF_PROCESSORS.insert(processor)
}

/// Create an output processor
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_output_processor(
    _ctx: Handle,
    out: Handle,
    ahx_encode: i32,
    newlines: i32,
) -> Handle {
    let mut processor = PdfProcessor::new(ProcessorType::Output);
    processor.output_stream = Some(out);
    processor.ahx_encode = ahx_encode != 0;
    processor.newlines = newlines != 0;

    PDF_PROCESSORS.insert(processor)
}

/// Create a sanitize filter processor
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_sanitize_filter(
    _ctx: Handle,
    _doc: Handle,
    chain: Handle,
    struct_parents: i32,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
) -> Handle {
    let mut processor = PdfProcessor::new(ProcessorType::Sanitize);
    processor.chain = Some(chain);
    processor.struct_parent = struct_parents;
    processor.transform = Matrix { a, b, c, d, e, f };

    PDF_PROCESSORS.insert(processor)
}

/// Create a color filter processor
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_color_filter(
    _ctx: Handle,
    _doc: Handle,
    chain: Handle,
    struct_parents: i32,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
) -> Handle {
    let mut processor = PdfProcessor::new(ProcessorType::Color);
    processor.chain = Some(chain);
    processor.struct_parent = struct_parents;
    processor.transform = Matrix { a, b, c, d, e, f };

    PDF_PROCESSORS.insert(processor)
}

/// Create a vectorize filter processor
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_vectorize_filter(
    _ctx: Handle,
    _doc: Handle,
    chain: Handle,
    struct_parents: i32,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
) -> Handle {
    let mut processor = PdfProcessor::new(ProcessorType::Vectorize);
    processor.chain = Some(chain);
    processor.struct_parent = struct_parents;
    processor.transform = Matrix { a, b, c, d, e, f };

    PDF_PROCESSORS.insert(processor)
}

// ============================================================================
// FFI Functions - Resource Stack
// ============================================================================

/// Push resources onto the processor's resource stack
#[unsafe(no_mangle)]
pub extern "C" fn pdf_processor_push_resources(_ctx: Handle, proc: Handle, res: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.rstack.resources.push(res);

        // Pass to chain if any
        if let Some(chain) = proc_guard.chain {
            drop(proc_guard);
            pdf_processor_push_resources(_ctx, chain, res);
        }
    }
}

/// Pop resources from the processor's resource stack
#[unsafe(no_mangle)]
pub extern "C" fn pdf_processor_pop_resources(_ctx: Handle, proc: Handle) -> Handle {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        let res = proc_guard.rstack.resources.pop().unwrap_or(0);

        // Pass to chain if any
        if let Some(chain) = proc_guard.chain {
            drop(proc_guard);
            let _ = pdf_processor_pop_resources(_ctx, chain);
        }

        return res;
    }
    0
}

// ============================================================================
// FFI Functions - Content Processing
// ============================================================================

/// Process a content stream
#[unsafe(no_mangle)]
pub extern "C" fn pdf_process_contents(
    _ctx: Handle,
    proc: Handle,
    _doc: Handle,
    res: Handle,
    _stm: Handle,
    _out_res: *mut Handle,
) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();

        // Push resources
        proc_guard.rstack.resources.push(res);

        // Initialize graphics state
        proc_guard.push_gstate();

        // In a real implementation, we would parse the content stream
        // and call the appropriate operator functions

        // Pop graphics state
        proc_guard.pop_gstate();

        // Pop resources
        proc_guard.rstack.resources.pop();
    }

    if !_out_res.is_null() {
        unsafe { *_out_res = res };
    }
}

/// Process an annotation
#[unsafe(no_mangle)]
pub extern "C" fn pdf_process_annot(_ctx: Handle, proc: Handle, annot: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();

        // Initialize state for annotation processing
        proc_guard.push_gstate();

        // Process annotation appearance stream (would parse in real implementation)
        proc_guard.record_op("annot", vec![OperatorArg::Integer(annot as i32)]);

        proc_guard.pop_gstate();
    }
}

/// Process a glyph (for Type 3 fonts)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_process_glyph(_ctx: Handle, proc: Handle, _doc: Handle, res: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.rstack.resources.push(res);
        // Process glyph content stream
        proc_guard.rstack.resources.pop();
    }
}

/// Process raw contents without resource handling
#[unsafe(no_mangle)]
pub extern "C" fn pdf_process_raw_contents(_ctx: Handle, proc: Handle, _doc: Handle, _stm: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let _proc_guard = proc_arc.lock().unwrap();
        // Process content stream without managing resources
    }
}

/// Count q/Q balance in a content stream
#[unsafe(no_mangle)]
pub extern "C" fn pdf_count_q_balance(
    _ctx: Handle,
    _doc: Handle,
    _res: Handle,
    _stm: Handle,
    prepend: *mut i32,
    append: *mut i32,
) {
    // In a real implementation, would scan the stream
    if !prepend.is_null() {
        unsafe { *prepend = 0 };
    }
    if !append.is_null() {
        unsafe { *append = 0 };
    }
}

// ============================================================================
// FFI Functions - Graphics State Operators
// ============================================================================

/// Set line width (w operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_w(_ctx: Handle, proc: Handle, linewidth: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.line_width = linewidth;
        proc_guard.record_op("w", vec![OperatorArg::Number(linewidth)]);
    }
}

/// Set line join (j operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_j(_ctx: Handle, proc: Handle, linejoin: i32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.line_join = linejoin;
        proc_guard.record_op("j", vec![OperatorArg::Integer(linejoin)]);
    }
}

/// Set line cap (J operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_J(_ctx: Handle, proc: Handle, linecap: i32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.line_cap = linecap;
        proc_guard.record_op("J", vec![OperatorArg::Integer(linecap)]);
    }
}

/// Set miter limit (M operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_M(_ctx: Handle, proc: Handle, miterlimit: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.miter_limit = miterlimit;
        proc_guard.record_op("M", vec![OperatorArg::Number(miterlimit)]);
    }
}

/// Set dash pattern (d operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_d(
    _ctx: Handle,
    proc: Handle,
    array: *const f32,
    array_len: i32,
    phase: f32,
) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();

        let dash_array = if !array.is_null() && array_len > 0 {
            unsafe { std::slice::from_raw_parts(array, array_len as usize) }.to_vec()
        } else {
            Vec::new()
        };

        proc_guard.gstate.dash_array = dash_array.clone();
        proc_guard.gstate.dash_phase = phase;

        let args: Vec<OperatorArg> = dash_array.iter().map(|&v| OperatorArg::Number(v)).collect();
        proc_guard.record_op(
            "d",
            vec![OperatorArg::Array(args), OperatorArg::Number(phase)],
        );
    }
}

/// Set rendering intent (ri operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_ri(_ctx: Handle, proc: Handle, intent: *const c_char) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();

        let intent_str = if !intent.is_null() {
            unsafe { CStr::from_ptr(intent) }
                .to_str()
                .unwrap_or("RelativeColorimetric")
                .to_string()
        } else {
            "RelativeColorimetric".to_string()
        };

        proc_guard.gstate.rendering_intent = intent_str.clone();
        proc_guard.record_op("ri", vec![OperatorArg::Name(intent_str)]);
    }
}

/// Set flatness (i operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_i(_ctx: Handle, proc: Handle, flatness: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.flatness = flatness;
        proc_guard.record_op("i", vec![OperatorArg::Number(flatness)]);
    }
}

/// Save graphics state (q operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_q(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.push_gstate();
        proc_guard.record_op("q", vec![]);
    }
}

/// Restore graphics state (Q operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_Q(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.pop_gstate();
        proc_guard.record_op("Q", vec![]);
    }
}

/// Concatenate matrix (cm operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_cm(
    _ctx: Handle,
    proc: Handle,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();

        let new_matrix = Matrix { a, b, c, d, e, f };
        proc_guard.gstate.ctm = proc_guard.gstate.ctm.concat(&new_matrix);

        proc_guard.record_op(
            "cm",
            vec![
                OperatorArg::Number(a),
                OperatorArg::Number(b),
                OperatorArg::Number(c),
                OperatorArg::Number(d),
                OperatorArg::Number(e),
                OperatorArg::Number(f),
            ],
        );
    }
}

// ============================================================================
// FFI Functions - Extended Graphics State Operators
// ============================================================================

/// Begin extended graphics state
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_gs_begin(_ctx: Handle, proc: Handle, name: *const c_char) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        let name_str = if !name.is_null() {
            unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("")
        } else {
            ""
        };
        proc_guard.record_op("gs", vec![OperatorArg::Name(name_str.to_string())]);
    }
}

/// Set blend mode
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_gs_BM(_ctx: Handle, proc: Handle, blendmode: *const c_char) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        let mode = if !blendmode.is_null() {
            unsafe { CStr::from_ptr(blendmode) }
                .to_str()
                .unwrap_or("Normal")
                .to_string()
        } else {
            "Normal".to_string()
        };
        proc_guard.gstate.blend_mode = mode;
    }
}

/// Set fill alpha (ca)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_gs_ca(_ctx: Handle, proc: Handle, alpha: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.fill_alpha = alpha;
    }
}

/// Set stroke alpha (CA)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_gs_CA(_ctx: Handle, proc: Handle, alpha: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.stroke_alpha = alpha;
    }
}

/// End extended graphics state
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_gs_end(_ctx: Handle, _proc: Handle) {
    // End of ExtGState processing
}

/// Set overprint for fill (op)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_gs_op(_ctx: Handle, proc: Handle, b: i32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.op_fill = b != 0;
    }
}

/// Set overprint for stroke (OP)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_gs_OP(_ctx: Handle, proc: Handle, b: i32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.op_stroke = b != 0;
    }
}

/// Set overprint mode (OPM)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_gs_OPM(_ctx: Handle, proc: Handle, i: i32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.opm = i;
    }
}

// ============================================================================
// FFI Functions - Path Construction Operators
// ============================================================================

/// Move to (m operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_m(_ctx: Handle, proc: Handle, x: f32, y: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.path.elements.push(PathElement::MoveTo { x, y });
        proc_guard.record_op("m", vec![OperatorArg::Number(x), OperatorArg::Number(y)]);
    }
}

/// Line to (l operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_l(_ctx: Handle, proc: Handle, x: f32, y: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.path.elements.push(PathElement::LineTo { x, y });
        proc_guard.record_op("l", vec![OperatorArg::Number(x), OperatorArg::Number(y)]);
    }
}

/// Curve to (c operator) - cubic bezier with two control points
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_c(
    _ctx: Handle,
    proc: Handle,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    x3: f32,
    y3: f32,
) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.path.elements.push(PathElement::CurveTo {
            x1,
            y1,
            x2,
            y2,
            x3,
            y3,
        });
        proc_guard.record_op(
            "c",
            vec![
                OperatorArg::Number(x1),
                OperatorArg::Number(y1),
                OperatorArg::Number(x2),
                OperatorArg::Number(y2),
                OperatorArg::Number(x3),
                OperatorArg::Number(y3),
            ],
        );
    }
}

/// Curve to (v operator) - control point at current point
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_v(_ctx: Handle, proc: Handle, x2: f32, y2: f32, x3: f32, y3: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard
            .path
            .elements
            .push(PathElement::CurveV { x2, y2, x3, y3 });
        proc_guard.record_op(
            "v",
            vec![
                OperatorArg::Number(x2),
                OperatorArg::Number(y2),
                OperatorArg::Number(x3),
                OperatorArg::Number(y3),
            ],
        );
    }
}

/// Curve to (y operator) - control point at end point
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_y(_ctx: Handle, proc: Handle, x1: f32, y1: f32, x3: f32, y3: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard
            .path
            .elements
            .push(PathElement::CurveY { x1, y1, x3, y3 });
        proc_guard.record_op(
            "y",
            vec![
                OperatorArg::Number(x1),
                OperatorArg::Number(y1),
                OperatorArg::Number(x3),
                OperatorArg::Number(y3),
            ],
        );
    }
}

/// Close path (h operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_h(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.path.elements.push(PathElement::ClosePath);
        proc_guard.record_op("h", vec![]);
    }
}

/// Rectangle (re operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_re(_ctx: Handle, proc: Handle, x: f32, y: f32, w: f32, h: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard
            .path
            .elements
            .push(PathElement::Rect { x, y, w, h });
        proc_guard.record_op(
            "re",
            vec![
                OperatorArg::Number(x),
                OperatorArg::Number(y),
                OperatorArg::Number(w),
                OperatorArg::Number(h),
            ],
        );
    }
}

// ============================================================================
// FFI Functions - Path Painting Operators
// ============================================================================

/// Stroke path (S operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_S(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op("S", vec![]);
        proc_guard.path = PathState::default();
    }
}

/// Close and stroke (s operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_s(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.path.elements.push(PathElement::ClosePath);
        proc_guard.record_op("s", vec![]);
        proc_guard.path = PathState::default();
    }
}

/// Fill path nonzero (f operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_f(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op("f", vec![]);
        proc_guard.path = PathState::default();
    }
}

/// Fill path nonzero (F operator - same as f)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_F(_ctx: Handle, proc: Handle) {
    pdf_op_f(_ctx, proc);
}

/// Fill path even-odd (f* operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_fstar(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op("f*", vec![]);
        proc_guard.path = PathState::default();
    }
}

/// Fill and stroke nonzero (B operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_B(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op("B", vec![]);
        proc_guard.path = PathState::default();
    }
}

/// Fill and stroke even-odd (B* operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_Bstar(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op("B*", vec![]);
        proc_guard.path = PathState::default();
    }
}

/// Close, fill and stroke nonzero (b operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_b(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.path.elements.push(PathElement::ClosePath);
        proc_guard.record_op("b", vec![]);
        proc_guard.path = PathState::default();
    }
}

/// Close, fill and stroke even-odd (b* operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_bstar(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.path.elements.push(PathElement::ClosePath);
        proc_guard.record_op("b*", vec![]);
        proc_guard.path = PathState::default();
    }
}

/// End path without filling or stroking (n operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_n(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op("n", vec![]);
        proc_guard.path = PathState::default();
    }
}

// ============================================================================
// FFI Functions - Clipping Operators
// ============================================================================

/// Set clipping path nonzero (W operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_W(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op("W", vec![]);
    }
}

/// Set clipping path even-odd (W* operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_Wstar(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op("W*", vec![]);
    }
}

// ============================================================================
// FFI Functions - Text Object Operators
// ============================================================================

/// Begin text object (BT operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_BT(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.in_text = true;
        proc_guard.text_object = Some(PdfTextObjectState {
            tlm: Matrix::IDENTITY,
            tm: Matrix::IDENTITY,
            text_mode: 0,
            text_content: Vec::new(),
            text_bbox: Rect::default(),
        });
        proc_guard.record_op("BT", vec![]);
    }
}

/// End text object (ET operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_ET(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.in_text = false;
        proc_guard.text_object = None;
        proc_guard.record_op("ET", vec![]);
    }
}

// ============================================================================
// FFI Functions - Text State Operators
// ============================================================================

/// Set character spacing (Tc operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_Tc(_ctx: Handle, proc: Handle, charspace: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.text_state.char_space = charspace;
        proc_guard.record_op("Tc", vec![OperatorArg::Number(charspace)]);
    }
}

/// Set word spacing (Tw operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_Tw(_ctx: Handle, proc: Handle, wordspace: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.text_state.word_space = wordspace;
        proc_guard.record_op("Tw", vec![OperatorArg::Number(wordspace)]);
    }
}

/// Set horizontal scaling (Tz operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_Tz(_ctx: Handle, proc: Handle, scale: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.text_state.scale = scale;
        proc_guard.record_op("Tz", vec![OperatorArg::Number(scale)]);
    }
}

/// Set leading (TL operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_TL(_ctx: Handle, proc: Handle, leading: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.text_state.leading = leading;
        proc_guard.record_op("TL", vec![OperatorArg::Number(leading)]);
    }
}

/// Set font and size (Tf operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_Tf(_ctx: Handle, proc: Handle, name: *const c_char, size: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();

        let font_name = if !name.is_null() {
            unsafe { CStr::from_ptr(name) }
                .to_str()
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        };

        proc_guard.text_state.font_name = font_name.clone();
        proc_guard.text_state.size = size;
        proc_guard.record_op(
            "Tf",
            vec![OperatorArg::Name(font_name), OperatorArg::Number(size)],
        );
    }
}

/// Set text rendering mode (Tr operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_Tr(_ctx: Handle, proc: Handle, render: i32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.text_state.render = render;
        proc_guard.record_op("Tr", vec![OperatorArg::Integer(render)]);
    }
}

/// Set text rise (Ts operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_Ts(_ctx: Handle, proc: Handle, rise: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.text_state.rise = rise;
        proc_guard.record_op("Ts", vec![OperatorArg::Number(rise)]);
    }
}

// ============================================================================
// FFI Functions - Text Positioning Operators
// ============================================================================

/// Move text position (Td operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_Td(_ctx: Handle, proc: Handle, tx: f32, ty: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        if let Some(ref mut tos) = proc_guard.text_object {
            let translate = Matrix {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                e: tx,
                f: ty,
            };
            tos.tlm = tos.tlm.concat(&translate);
            tos.tm = tos.tlm;
        }
        proc_guard.record_op("Td", vec![OperatorArg::Number(tx), OperatorArg::Number(ty)]);
    }
}

/// Move text position and set leading (TD operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_TD(_ctx: Handle, proc: Handle, tx: f32, ty: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.text_state.leading = -ty;
        if let Some(ref mut tos) = proc_guard.text_object {
            let translate = Matrix {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                e: tx,
                f: ty,
            };
            tos.tlm = tos.tlm.concat(&translate);
            tos.tm = tos.tlm;
        }
        proc_guard.record_op("TD", vec![OperatorArg::Number(tx), OperatorArg::Number(ty)]);
    }
}

/// Set text matrix (Tm operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_Tm(
    _ctx: Handle,
    proc: Handle,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        if let Some(ref mut tos) = proc_guard.text_object {
            tos.tlm = Matrix { a, b, c, d, e, f };
            tos.tm = tos.tlm;
        }
        proc_guard.record_op(
            "Tm",
            vec![
                OperatorArg::Number(a),
                OperatorArg::Number(b),
                OperatorArg::Number(c),
                OperatorArg::Number(d),
                OperatorArg::Number(e),
                OperatorArg::Number(f),
            ],
        );
    }
}

/// Move to next line (T* operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_Tstar(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        let leading = proc_guard.text_state.leading;
        if let Some(ref mut tos) = proc_guard.text_object {
            let translate = Matrix {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                e: 0.0,
                f: -leading,
            };
            tos.tlm = tos.tlm.concat(&translate);
            tos.tm = tos.tlm;
        }
        proc_guard.record_op("T*", vec![]);
    }
}

// ============================================================================
// FFI Functions - Text Showing Operators
// ============================================================================

/// Show text (Tj operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_Tj(_ctx: Handle, proc: Handle, str: *const c_char, len: usize) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();

        let text = if !str.is_null() {
            let bytes = unsafe { std::slice::from_raw_parts(str as *const u8, len) };
            String::from_utf8_lossy(bytes).to_string()
        } else {
            String::new()
        };

        // Extract text state values before mutable borrow
        let font_name = proc_guard.text_state.font_name.clone();
        let font_size = proc_guard.text_state.size;

        if let Some(ref mut tos) = proc_guard.text_object {
            tos.text_content.push(TextSpan {
                text: text.clone(),
                matrix: tos.tm,
                font_name,
                font_size,
            });
        }

        proc_guard.record_op("Tj", vec![OperatorArg::String(text)]);
    }
}

/// Show text with individual positioning (TJ operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_TJ(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op("TJ", vec![]);
    }
}

/// Move to next line and show text (' operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_squote(_ctx: Handle, proc: Handle, str: *const c_char, len: usize) {
    // T*
    pdf_op_Tstar(_ctx, proc);
    // Tj
    pdf_op_Tj(_ctx, proc, str, len);
}

/// Set spacing, move to next line and show text (" operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_dquote(
    _ctx: Handle,
    proc: Handle,
    aw: f32,
    ac: f32,
    str: *const c_char,
    len: usize,
) {
    // Tw
    pdf_op_Tw(_ctx, proc, aw);
    // Tc
    pdf_op_Tc(_ctx, proc, ac);
    // '
    pdf_op_squote(_ctx, proc, str, len);
}

// ============================================================================
// FFI Functions - Type 3 Font Operators
// ============================================================================

/// Type 3 font glyph width (d0 operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_d0(_ctx: Handle, proc: Handle, wx: f32, wy: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op("d0", vec![OperatorArg::Number(wx), OperatorArg::Number(wy)]);
    }
}

/// Type 3 font glyph width and bounding box (d1 operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_d1(
    _ctx: Handle,
    proc: Handle,
    wx: f32,
    wy: f32,
    llx: f32,
    lly: f32,
    urx: f32,
    ury: f32,
) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op(
            "d1",
            vec![
                OperatorArg::Number(wx),
                OperatorArg::Number(wy),
                OperatorArg::Number(llx),
                OperatorArg::Number(lly),
                OperatorArg::Number(urx),
                OperatorArg::Number(ury),
            ],
        );
    }
}

// ============================================================================
// FFI Functions - Color Operators
// ============================================================================

/// Set stroke colorspace (CS operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_CS(_ctx: Handle, proc: Handle, name: *const c_char) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        let cs_name = if !name.is_null() {
            unsafe { CStr::from_ptr(name) }
                .to_str()
                .unwrap_or("DeviceGray")
                .to_string()
        } else {
            "DeviceGray".to_string()
        };
        proc_guard.gstate.stroke_cs = cs_name.clone();
        proc_guard.record_op("CS", vec![OperatorArg::Name(cs_name)]);
    }
}

/// Set fill colorspace (cs operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_cs(_ctx: Handle, proc: Handle, name: *const c_char) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        let cs_name = if !name.is_null() {
            unsafe { CStr::from_ptr(name) }
                .to_str()
                .unwrap_or("DeviceGray")
                .to_string()
        } else {
            "DeviceGray".to_string()
        };
        proc_guard.gstate.fill_cs = cs_name.clone();
        proc_guard.record_op("cs", vec![OperatorArg::Name(cs_name)]);
    }
}

/// Set stroke color (SC operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_SC_color(_ctx: Handle, proc: Handle, n: i32, color: *const f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        let colors = if !color.is_null() && n > 0 {
            unsafe { std::slice::from_raw_parts(color, n as usize) }.to_vec()
        } else {
            vec![0.0]
        };
        proc_guard.gstate.stroke_color = colors.clone();
        let args: Vec<OperatorArg> = colors.iter().map(|&c| OperatorArg::Number(c)).collect();
        proc_guard.record_op("SC", args);
    }
}

/// Set fill color (sc operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_sc_color(_ctx: Handle, proc: Handle, n: i32, color: *const f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        let colors = if !color.is_null() && n > 0 {
            unsafe { std::slice::from_raw_parts(color, n as usize) }.to_vec()
        } else {
            vec![0.0]
        };
        proc_guard.gstate.fill_color = colors.clone();
        let args: Vec<OperatorArg> = colors.iter().map(|&c| OperatorArg::Number(c)).collect();
        proc_guard.record_op("sc", args);
    }
}

/// Set stroke gray (G operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_G(_ctx: Handle, proc: Handle, g: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.stroke_cs = "DeviceGray".to_string();
        proc_guard.gstate.stroke_color = vec![g];
        proc_guard.record_op("G", vec![OperatorArg::Number(g)]);
    }
}

/// Set fill gray (g operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_g(_ctx: Handle, proc: Handle, g: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.fill_cs = "DeviceGray".to_string();
        proc_guard.gstate.fill_color = vec![g];
        proc_guard.record_op("g", vec![OperatorArg::Number(g)]);
    }
}

/// Set stroke RGB (RG operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_RG(_ctx: Handle, proc: Handle, r: f32, g: f32, b: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.stroke_cs = "DeviceRGB".to_string();
        proc_guard.gstate.stroke_color = vec![r, g, b];
        proc_guard.record_op(
            "RG",
            vec![
                OperatorArg::Number(r),
                OperatorArg::Number(g),
                OperatorArg::Number(b),
            ],
        );
    }
}

/// Set fill RGB (rg operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_rg(_ctx: Handle, proc: Handle, r: f32, g: f32, b: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.fill_cs = "DeviceRGB".to_string();
        proc_guard.gstate.fill_color = vec![r, g, b];
        proc_guard.record_op(
            "rg",
            vec![
                OperatorArg::Number(r),
                OperatorArg::Number(g),
                OperatorArg::Number(b),
            ],
        );
    }
}

/// Set stroke CMYK (K operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_K(_ctx: Handle, proc: Handle, c: f32, m: f32, y: f32, k: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.stroke_cs = "DeviceCMYK".to_string();
        proc_guard.gstate.stroke_color = vec![c, m, y, k];
        proc_guard.record_op(
            "K",
            vec![
                OperatorArg::Number(c),
                OperatorArg::Number(m),
                OperatorArg::Number(y),
                OperatorArg::Number(k),
            ],
        );
    }
}

/// Set fill CMYK (k operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_k(_ctx: Handle, proc: Handle, c: f32, m: f32, y: f32, k: f32) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.fill_cs = "DeviceCMYK".to_string();
        proc_guard.gstate.fill_color = vec![c, m, y, k];
        proc_guard.record_op(
            "k",
            vec![
                OperatorArg::Number(c),
                OperatorArg::Number(m),
                OperatorArg::Number(y),
                OperatorArg::Number(k),
            ],
        );
    }
}

// ============================================================================
// FFI Functions - XObject/Image/Shading Operators
// ============================================================================

/// Inline image (BI operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_BI(_ctx: Handle, proc: Handle, image: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op("BI", vec![OperatorArg::Integer(image as i32)]);
    }
}

/// Shading (sh operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_sh(_ctx: Handle, proc: Handle, name: *const c_char) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        let shade_name = if !name.is_null() {
            unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("")
        } else {
            ""
        };
        proc_guard.record_op("sh", vec![OperatorArg::Name(shade_name.to_string())]);
    }
}

/// XObject image (Do operator for images)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_Do_image(_ctx: Handle, proc: Handle, name: *const c_char, image: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        let img_name = if !name.is_null() {
            unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("")
        } else {
            ""
        };
        proc_guard.record_op(
            "Do",
            vec![
                OperatorArg::Name(img_name.to_string()),
                OperatorArg::Integer(image as i32),
            ],
        );
    }
}

/// XObject form (Do operator for forms)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_Do_form(_ctx: Handle, proc: Handle, name: *const c_char, form: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        let form_name = if !name.is_null() {
            unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("")
        } else {
            ""
        };
        proc_guard.record_op(
            "Do",
            vec![
                OperatorArg::Name(form_name.to_string()),
                OperatorArg::Integer(form as i32),
            ],
        );
    }
}

// ============================================================================
// FFI Functions - Marked Content Operators
// ============================================================================

/// Marked content point (MP operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_MP(_ctx: Handle, proc: Handle, tag: *const c_char) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        let tag_str = if !tag.is_null() {
            unsafe { CStr::from_ptr(tag) }.to_str().unwrap_or("")
        } else {
            ""
        };
        proc_guard.record_op("MP", vec![OperatorArg::Name(tag_str.to_string())]);
    }
}

/// Marked content point with properties (DP operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_DP(_ctx: Handle, proc: Handle, tag: *const c_char) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        let tag_str = if !tag.is_null() {
            unsafe { CStr::from_ptr(tag) }.to_str().unwrap_or("")
        } else {
            ""
        };
        proc_guard.record_op("DP", vec![OperatorArg::Name(tag_str.to_string())]);
    }
}

/// Begin marked content (BMC operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_BMC(_ctx: Handle, proc: Handle, tag: *const c_char) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        let tag_str = if !tag.is_null() {
            unsafe { CStr::from_ptr(tag) }.to_str().unwrap_or("")
        } else {
            ""
        };
        proc_guard.record_op("BMC", vec![OperatorArg::Name(tag_str.to_string())]);
    }
}

/// Begin marked content with properties (BDC operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_BDC(_ctx: Handle, proc: Handle, tag: *const c_char) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        let tag_str = if !tag.is_null() {
            unsafe { CStr::from_ptr(tag) }.to_str().unwrap_or("")
        } else {
            ""
        };
        proc_guard.record_op("BDC", vec![OperatorArg::Name(tag_str.to_string())]);
    }
}

/// End marked content (EMC operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_EMC(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op("EMC", vec![]);
    }
}

// ============================================================================
// FFI Functions - Compatibility Operators
// ============================================================================

/// Begin compatibility section (BX operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_BX(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op("BX", vec![]);
    }
}

/// End compatibility section (EX operator)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_EX(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op("EX", vec![]);
    }
}

// ============================================================================
// FFI Functions - End Operators
// ============================================================================

/// End of data (before finalize)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_EOD(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op("EOD", vec![]);
    }
}

/// End of stream (finalize)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_op_END(_ctx: Handle, proc: Handle) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let mut proc_guard = proc_arc.lock().unwrap();
        proc_guard.record_op("END", vec![]);
    }
}

// ============================================================================
// FFI Functions - Utility
// ============================================================================

/// Get processor type
#[unsafe(no_mangle)]
pub extern "C" fn pdf_processor_get_type(_ctx: Handle, proc: Handle) -> i32 {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let proc_guard = proc_arc.lock().unwrap();
        proc_guard.proc_type as i32
    } else {
        0
    }
}

/// Get number of operators processed
#[unsafe(no_mangle)]
pub extern "C" fn pdf_processor_get_operator_count(_ctx: Handle, proc: Handle) -> i32 {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let proc_guard = proc_arc.lock().unwrap();
        proc_guard.operators.len() as i32
    } else {
        0
    }
}

/// Get gstate stack depth
#[unsafe(no_mangle)]
pub extern "C" fn pdf_processor_get_gstate_depth(_ctx: Handle, proc: Handle) -> i32 {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate_stack.len() as i32
    } else {
        0
    }
}

/// Check if processor is in text object
#[unsafe(no_mangle)]
pub extern "C" fn pdf_processor_in_text(_ctx: Handle, proc: Handle) -> i32 {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let proc_guard = proc_arc.lock().unwrap();
        if proc_guard.in_text { 1 } else { 0 }
    } else {
        0
    }
}

/// Get current line width
#[unsafe(no_mangle)]
pub extern "C" fn pdf_processor_get_line_width(_ctx: Handle, proc: Handle) -> f32 {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let proc_guard = proc_arc.lock().unwrap();
        proc_guard.gstate.line_width
    } else {
        1.0
    }
}

/// Get current CTM
#[unsafe(no_mangle)]
pub extern "C" fn pdf_processor_get_ctm(
    _ctx: Handle,
    proc: Handle,
    a: *mut f32,
    b: *mut f32,
    c: *mut f32,
    d: *mut f32,
    e: *mut f32,
    f: *mut f32,
) {
    if let Some(proc_arc) = PDF_PROCESSORS.get(proc) {
        let proc_guard = proc_arc.lock().unwrap();
        let ctm = &proc_guard.gstate.ctm;
        unsafe {
            if !a.is_null() {
                *a = ctm.a;
            }
            if !b.is_null() {
                *b = ctm.b;
            }
            if !c.is_null() {
                *c = ctm.c;
            }
            if !d.is_null() {
                *d = ctm.d;
            }
            if !e.is_null() {
                *e = ctm.e;
            }
            if !f.is_null() {
                *f = ctm.f;
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_creation() {
        let ctx = 1;
        let proc = pdf_new_processor(ctx, 0);
        assert!(proc > 0);
        pdf_drop_processor(ctx, proc);
    }

    #[test]
    fn test_run_processor() {
        let ctx = 1;
        let proc = pdf_new_run_processor(ctx, 0, 0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0, ptr::null());
        assert!(proc > 0);

        assert_eq!(pdf_processor_get_type(ctx, proc), ProcessorType::Run as i32);

        pdf_drop_processor(ctx, proc);
    }

    #[test]
    fn test_buffer_processor() {
        let ctx = 1;
        let proc = pdf_new_buffer_processor(ctx, 0, 0, 1);
        assert!(proc > 0);
        assert_eq!(
            pdf_processor_get_type(ctx, proc),
            ProcessorType::Buffer as i32
        );
        pdf_drop_processor(ctx, proc);
    }

    #[test]
    fn test_gstate_stack() {
        let ctx = 1;
        let proc = pdf_new_processor(ctx, 0);

        // Initial depth should be 0
        assert_eq!(pdf_processor_get_gstate_depth(ctx, proc), 0);

        // Push state
        pdf_op_q(ctx, proc);
        assert_eq!(pdf_processor_get_gstate_depth(ctx, proc), 1);

        // Push again
        pdf_op_q(ctx, proc);
        assert_eq!(pdf_processor_get_gstate_depth(ctx, proc), 2);

        // Pop
        pdf_op_Q(ctx, proc);
        assert_eq!(pdf_processor_get_gstate_depth(ctx, proc), 1);

        pdf_drop_processor(ctx, proc);
    }

    #[test]
    fn test_graphics_state_ops() {
        let ctx = 1;
        let proc = pdf_new_processor(ctx, 0);

        pdf_op_w(ctx, proc, 2.5);
        assert!((pdf_processor_get_line_width(ctx, proc) - 2.5).abs() < 0.001);

        pdf_op_j(ctx, proc, 1);
        pdf_op_J(ctx, proc, 2);
        pdf_op_M(ctx, proc, 5.0);
        pdf_op_i(ctx, proc, 0.5);

        // Check operator count
        assert_eq!(pdf_processor_get_operator_count(ctx, proc), 5);

        pdf_drop_processor(ctx, proc);
    }

    #[test]
    fn test_path_ops() {
        let ctx = 1;
        let proc = pdf_new_processor(ctx, 0);

        pdf_op_m(ctx, proc, 10.0, 20.0);
        pdf_op_l(ctx, proc, 100.0, 200.0);
        pdf_op_c(ctx, proc, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0);
        pdf_op_h(ctx, proc);
        pdf_op_S(ctx, proc);

        assert_eq!(pdf_processor_get_operator_count(ctx, proc), 5);

        pdf_drop_processor(ctx, proc);
    }

    #[test]
    fn test_text_ops() {
        let ctx = 1;
        let proc = pdf_new_processor(ctx, 0);

        pdf_op_BT(ctx, proc);
        assert_eq!(pdf_processor_in_text(ctx, proc), 1);

        let font = CString::new("F1").unwrap();
        pdf_op_Tf(ctx, proc, font.as_ptr(), 12.0);
        pdf_op_Td(ctx, proc, 100.0, 700.0);

        let text = CString::new("Hello").unwrap();
        pdf_op_Tj(ctx, proc, text.as_ptr(), 5);

        pdf_op_ET(ctx, proc);
        assert_eq!(pdf_processor_in_text(ctx, proc), 0);

        pdf_drop_processor(ctx, proc);
    }

    #[test]
    fn test_color_ops() {
        let ctx = 1;
        let proc = pdf_new_processor(ctx, 0);

        pdf_op_g(ctx, proc, 0.5);
        pdf_op_G(ctx, proc, 0.7);
        pdf_op_rg(ctx, proc, 1.0, 0.0, 0.0);
        pdf_op_RG(ctx, proc, 0.0, 1.0, 0.0);
        pdf_op_k(ctx, proc, 0.0, 1.0, 1.0, 0.0);
        pdf_op_K(ctx, proc, 1.0, 0.0, 0.0, 0.0);

        assert_eq!(pdf_processor_get_operator_count(ctx, proc), 6);

        pdf_drop_processor(ctx, proc);
    }

    #[test]
    fn test_ctm() {
        let ctx = 1;
        let proc = pdf_new_processor(ctx, 0);

        // Initial CTM should be identity
        let mut a: f32 = 0.0;
        let mut b: f32 = 0.0;
        let mut c: f32 = 0.0;
        let mut d: f32 = 0.0;
        let mut e: f32 = 0.0;
        let mut f: f32 = 0.0;
        pdf_processor_get_ctm(ctx, proc, &mut a, &mut b, &mut c, &mut d, &mut e, &mut f);

        assert!((a - 1.0).abs() < 0.001);
        assert!((d - 1.0).abs() < 0.001);

        // Apply translation
        pdf_op_cm(ctx, proc, 1.0, 0.0, 0.0, 1.0, 100.0, 200.0);

        pdf_processor_get_ctm(ctx, proc, &mut a, &mut b, &mut c, &mut d, &mut e, &mut f);
        assert!((e - 100.0).abs() < 0.001);
        assert!((f - 200.0).abs() < 0.001);

        pdf_drop_processor(ctx, proc);
    }

    #[test]
    fn test_marked_content() {
        let ctx = 1;
        let proc = pdf_new_processor(ctx, 0);

        let tag = CString::new("Span").unwrap();
        pdf_op_BMC(ctx, proc, tag.as_ptr());
        pdf_op_EMC(ctx, proc);

        assert_eq!(pdf_processor_get_operator_count(ctx, proc), 2);

        pdf_drop_processor(ctx, proc);
    }

    #[test]
    fn test_sanitize_filter() {
        let ctx = 1;

        // Create a buffer processor as the chain
        let buffer_proc = pdf_new_buffer_processor(ctx, 0, 0, 1);

        // Create sanitize filter
        let sanitize =
            pdf_new_sanitize_filter(ctx, 0, buffer_proc, 0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        assert!(sanitize > 0);
        assert_eq!(
            pdf_processor_get_type(ctx, sanitize),
            ProcessorType::Sanitize as i32
        );

        pdf_drop_processor(ctx, sanitize);
    }

    #[test]
    fn test_resource_stack() {
        let ctx = 1;
        let proc = pdf_new_processor(ctx, 0);

        pdf_processor_push_resources(ctx, proc, 100);
        pdf_processor_push_resources(ctx, proc, 200);

        let popped1 = pdf_processor_pop_resources(ctx, proc);
        assert_eq!(popped1, 200);

        let popped2 = pdf_processor_pop_resources(ctx, proc);
        assert_eq!(popped2, 100);

        pdf_drop_processor(ctx, proc);
    }

    #[test]
    fn test_processor_close_reset() {
        let ctx = 1;
        let proc = pdf_new_processor(ctx, 0);

        // Add some state
        pdf_op_q(ctx, proc);
        pdf_op_w(ctx, proc, 5.0);

        // Close
        pdf_close_processor(ctx, proc);

        // Reset
        pdf_reset_processor(ctx, proc);

        // State should be reset
        assert_eq!(pdf_processor_get_gstate_depth(ctx, proc), 0);
        assert!((pdf_processor_get_line_width(ctx, proc) - 1.0).abs() < 0.001);

        pdf_drop_processor(ctx, proc);
    }

    #[test]
    fn test_keep_drop() {
        let ctx = 1;
        let proc = pdf_new_processor(ctx, 0);

        // Keep should increment ref count
        let kept = pdf_keep_processor(ctx, proc);
        assert_eq!(kept, proc);

        // First drop decrements
        pdf_drop_processor(ctx, proc);

        // Processor should still exist
        assert_eq!(
            pdf_processor_get_type(ctx, proc),
            ProcessorType::Base as i32
        );

        // Second drop should actually free
        pdf_drop_processor(ctx, proc);
    }
}
