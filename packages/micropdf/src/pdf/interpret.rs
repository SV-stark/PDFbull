//! PDF Content Stream Interpreter
//!
//! This module implements the PDF content stream interpreter, which parses
//! PDF operators from a content stream and calls the appropriate device methods.
//!
//! The interpreter maintains a graphics state stack and handles:
//! - Path construction and painting
//! - Text positioning and rendering
//! - Color and colorspace operations
//! - Graphics state save/restore
//! - Transformations
//! - Clipping
//! - Images
//! - Form XObjects
//! - Transparency groups

use crate::fitz::colorspace::Colorspace;
use crate::fitz::device::Device;
use crate::fitz::geometry::{Matrix, Point};
use crate::fitz::path::{LineCap, LineJoin, Path};
use crate::pdf::lexer::{LexBuf, Lexer, Token};
use crate::pdf::object::{Dict, Object};

/// PDF graphics state
#[derive(Debug, Clone)]
pub struct GraphicsState {
    /// Current transformation matrix
    pub ctm: Matrix,

    /// Current line width
    pub line_width: f32,

    /// Current line cap style (0=butt, 1=round, 2=square)
    pub line_cap: i32,

    /// Current line join style (0=miter, 1=round, 2=bevel)
    pub line_join: i32,

    /// Current miter limit
    pub miter_limit: f32,

    /// Current dash pattern
    pub dash_pattern: Vec<f32>,
    pub dash_phase: f32,

    /// Current fill colorspace
    pub fill_colorspace: Colorspace,

    /// Current fill color components
    pub fill_color: Vec<f32>,

    /// Current stroke colorspace
    pub stroke_colorspace: Colorspace,

    /// Current stroke color components
    pub stroke_color: Vec<f32>,

    /// Current fill alpha
    pub fill_alpha: f32,

    /// Current stroke alpha
    pub stroke_alpha: f32,

    /// Current blend mode
    pub blend_mode: String,

    /// Current font and size
    pub font: Option<String>,
    pub font_size: f32,

    /// Current text matrix
    pub text_matrix: Matrix,

    /// Current text line matrix
    pub text_line_matrix: Matrix,

    /// Current character spacing
    pub char_spacing: f32,

    /// Current word spacing
    pub word_spacing: f32,

    /// Current horizontal scaling
    pub horizontal_scaling: f32,

    /// Current text leading
    pub leading: f32,

    /// Current text rendering mode
    pub text_render_mode: i32,

    /// Current text rise
    pub text_rise: f32,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: Matrix::IDENTITY,
            line_width: 1.0,
            line_cap: 0,
            line_join: 0,
            miter_limit: 10.0,
            dash_pattern: Vec::new(),
            dash_phase: 0.0,
            fill_colorspace: Colorspace::device_gray(),
            fill_color: vec![0.0],
            stroke_colorspace: Colorspace::device_gray(),
            stroke_color: vec![0.0],
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            blend_mode: "Normal".to_string(),
            font: None,
            font_size: 0.0,
            text_matrix: Matrix::IDENTITY,
            text_line_matrix: Matrix::IDENTITY,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            leading: 0.0,
            text_render_mode: 0,
            text_rise: 0.0,
        }
    }
}

/// PDF content stream interpreter
pub struct Interpreter {
    /// Graphics state stack
    state_stack: Vec<GraphicsState>,

    /// Current path being constructed
    current_path: Option<Path>,

    /// Current point in path
    current_point: Option<Point>,

    /// Resource dictionary
    resources: Option<Dict>,
}

impl Interpreter {
    /// Create a new interpreter
    pub fn new() -> Self {
        Self {
            state_stack: vec![GraphicsState::default()],
            current_path: None,
            current_point: None,
            resources: None,
        }
    }

    /// Set the resource dictionary
    pub fn set_resources(&mut self, resources: Dict) {
        self.resources = Some(resources);
    }

    /// Get the current graphics state
    fn state(&self) -> &GraphicsState {
        self.state_stack.last().unwrap()
    }

    /// Get the current graphics state (mutable)
    fn state_mut(&mut self) -> &mut GraphicsState {
        self.state_stack.last_mut().unwrap()
    }

    /// Push a new graphics state (q operator)
    fn push_state(&mut self) {
        let current = self.state().clone();
        self.state_stack.push(current);
    }

    /// Pop a graphics state (Q operator)
    fn pop_state(&mut self) {
        if self.state_stack.len() > 1 {
            self.state_stack.pop();
        }
    }

    /// Interpret a content stream and call device methods
    pub fn interpret<D: Device>(&mut self, stream: &[u8], device: &mut D) -> Result<(), String> {
        let mut lexer = Lexer::new(stream);
        let mut buf = LexBuf::new();
        let mut operands: Vec<Object> = Vec::new();

        loop {
            match lexer.lex(&mut buf) {
                Ok(Token::Eof) => break,
                Ok(Token::Keyword) => {
                    // Process the operator with accumulated operands
                    let op = buf.as_str();
                    self.process_operator(op, &operands, device)?;
                    operands.clear();
                }
                Ok(Token::Int) => {
                    operands.push(Object::Int(buf.as_int()));
                }
                Ok(Token::Real) => {
                    operands.push(Object::Real(buf.as_float()));
                }
                Ok(Token::String) => {
                    operands.push(Object::String(crate::pdf::object::PdfString::new(
                        buf.as_str().as_bytes().to_vec(),
                    )));
                }
                Ok(Token::Name) => {
                    operands.push(Object::Name(crate::pdf::object::Name::new(buf.as_str())));
                }
                Ok(Token::True) => {
                    operands.push(Object::Bool(true));
                }
                Ok(Token::False) => {
                    operands.push(Object::Bool(false));
                }
                Ok(Token::Null) => {
                    operands.push(Object::Null);
                }
                Ok(Token::OpenArray) => {
                    // Parse array
                    let array = self.parse_array(&mut lexer, &mut buf)?;
                    operands.push(Object::Array(array));
                }
                Ok(Token::OpenDict) => {
                    // Parse dictionary
                    let dict = self.parse_dict(&mut lexer, &mut buf)?;
                    operands.push(Object::Dict(dict));
                }
                Ok(_) => {
                    // Skip other tokens
                }
                Err(e) => return Err(format!("Lexer error: {}", e)),
            }
        }

        Ok(())
    }

    /// Parse an array from the token stream
    fn parse_array(&self, lexer: &mut Lexer, buf: &mut LexBuf) -> Result<Vec<Object>, String> {
        let mut array = Vec::new();

        loop {
            match lexer.lex(buf) {
                Ok(Token::CloseArray) => break,
                Ok(Token::Int) => array.push(Object::Int(buf.as_int())),
                Ok(Token::Real) => array.push(Object::Real(buf.as_float())),
                Ok(Token::Name) => {
                    array.push(Object::Name(crate::pdf::object::Name::new(buf.as_str())))
                }
                Ok(Token::String) => array.push(Object::String(
                    crate::pdf::object::PdfString::new(buf.as_str().as_bytes().to_vec()),
                )),
                Ok(Token::True) => array.push(Object::Bool(true)),
                Ok(Token::False) => array.push(Object::Bool(false)),
                Ok(Token::Null) => array.push(Object::Null),
                Ok(Token::Eof) => return Err("Unexpected end of stream in array".to_string()),
                Err(e) => return Err(format!("Error parsing array: {}", e)),
                _ => {} // Skip other tokens
            }
        }

        Ok(array)
    }

    /// Parse a dictionary from the token stream
    fn parse_dict(&self, lexer: &mut Lexer, buf: &mut LexBuf) -> Result<Dict, String> {
        let mut dict = Dict::new();

        loop {
            match lexer.lex(buf) {
                Ok(Token::CloseDict) => break,
                Ok(Token::Name) => {
                    let key = crate::pdf::object::Name::new(buf.as_str());

                    // Read value
                    match lexer.lex(buf) {
                        Ok(Token::Int) => {
                            dict.insert(key, Object::Int(buf.as_int()));
                        }
                        Ok(Token::Real) => {
                            dict.insert(key, Object::Real(buf.as_float()));
                        }
                        Ok(Token::Name) => {
                            dict.insert(
                                key,
                                Object::Name(crate::pdf::object::Name::new(buf.as_str())),
                            );
                        }
                        Ok(Token::String) => {
                            dict.insert(
                                key,
                                Object::String(crate::pdf::object::PdfString::new(
                                    buf.as_str().as_bytes().to_vec(),
                                )),
                            );
                        }
                        Ok(Token::True) => {
                            dict.insert(key, Object::Bool(true));
                        }
                        Ok(Token::False) => {
                            dict.insert(key, Object::Bool(false));
                        }
                        Ok(Token::Null) => {
                            dict.insert(key, Object::Null);
                        }
                        _ => return Err(format!("Invalid value for key '{}'", key.as_str())),
                    }
                }
                Ok(Token::Eof) => return Err("Unexpected end of stream in dictionary".to_string()),
                Err(e) => return Err(format!("Error parsing dictionary: {}", e)),
                _ => {} // Skip other tokens
            }
        }

        Ok(dict)
    }

    /// Process a single PDF operator
    fn process_operator<D: Device>(
        &mut self,
        op: &str,
        operands: &[Object],
        device: &mut D,
    ) -> Result<(), String> {
        match op {
            // Graphics state operators
            "q" => self.op_save_state(),
            "Q" => self.op_restore_state(),
            "cm" => self.op_concat_matrix(operands)?,
            "w" => self.op_set_line_width(operands)?,
            "J" => self.op_set_line_cap(operands)?,
            "j" => self.op_set_line_join(operands)?,
            "M" => self.op_set_miter_limit(operands)?,
            "d" => self.op_set_dash(operands)?,
            "gs" => self.op_set_gstate(operands)?,

            // Path construction operators
            "m" => self.op_move_to(operands)?,
            "l" => self.op_line_to(operands)?,
            "c" => self.op_curve_to(operands)?,
            "v" => self.op_curve_to_v(operands)?,
            "y" => self.op_curve_to_y(operands)?,
            "h" => self.op_close_path(),
            "re" => self.op_rectangle(operands)?,

            // Path painting operators
            "S" => self.op_stroke(device)?,
            "s" => self.op_close_and_stroke(device)?,
            "f" | "F" => self.op_fill(device)?,
            "f*" => self.op_fill_even_odd(device)?,
            "B" => self.op_fill_and_stroke(device)?,
            "B*" => self.op_fill_and_stroke_even_odd(device)?,
            "b" => self.op_close_fill_and_stroke(device)?,
            "b*" => self.op_close_fill_and_stroke_even_odd(device)?,
            "n" => self.op_end_path(),

            // Clipping path operators
            "W" => self.op_clip(),
            "W*" => self.op_clip_even_odd(),

            // Color operators
            "CS" => self.op_set_stroke_colorspace(operands)?,
            "cs" => self.op_set_fill_colorspace(operands)?,
            "SC" | "SCN" => self.op_set_stroke_color(operands)?,
            "sc" | "scn" => self.op_set_fill_color(operands)?,
            "G" => self.op_set_stroke_gray(operands)?,
            "g" => self.op_set_fill_gray(operands)?,
            "RG" => self.op_set_stroke_rgb(operands)?,
            "rg" => self.op_set_fill_rgb(operands)?,
            "K" => self.op_set_stroke_cmyk(operands)?,
            "k" => self.op_set_fill_cmyk(operands)?,

            // Text object operators
            "BT" => self.op_begin_text(),
            "ET" => self.op_end_text(),

            // Text positioning operators
            "Td" => self.op_text_move(operands)?,
            "TD" => self.op_text_move_set_leading(operands)?,
            "Tm" => self.op_text_set_matrix(operands)?,
            "T*" => self.op_text_next_line(),

            // Text state operators
            "Tc" => self.op_set_char_spacing(operands)?,
            "Tw" => self.op_set_word_spacing(operands)?,
            "Tz" => self.op_set_horizontal_scaling(operands)?,
            "TL" => self.op_set_leading(operands)?,
            "Tf" => self.op_set_font(operands)?,
            "Tr" => self.op_set_text_render_mode(operands)?,
            "Ts" => self.op_set_text_rise(operands)?,

            // Text showing operators
            "Tj" => self.op_show_text(operands, device)?,
            "TJ" => self.op_show_text_adjusted(operands, device)?,
            "'" => self.op_show_text_next_line(operands, device)?,
            "\"" => self.op_show_text_next_line_with_spacing(operands, device)?,

            // XObject operators
            "Do" => self.op_paint_xobject(operands, device)?,

            // Image operators
            "BI" => self.op_begin_inline_image(operands)?,
            "ID" => self.op_inline_image_data(operands)?,
            "EI" => self.op_end_inline_image(device)?,

            // Marked content operators
            "MP" => self.op_marked_content_point(operands)?,
            "DP" => self.op_marked_content_point_with_props(operands)?,
            "BMC" => self.op_begin_marked_content(operands)?,
            "BDC" => self.op_begin_marked_content_with_props(operands)?,
            "EMC" => self.op_end_marked_content()?,

            // Compatibility operators
            "BX" => self.op_begin_compat(),
            "EX" => self.op_end_compat(),

            // Shading operator
            "sh" => self.op_shade(operands, device)?,

            _ => {
                // Unknown operator - log but don't fail
                // This allows forward compatibility with newer PDF versions
            }
        }

        Ok(())
    }

    // ========================================================================
    // Graphics State Operators
    // ========================================================================

    fn op_save_state(&mut self) {
        self.push_state();
    }

    fn op_restore_state(&mut self) {
        self.pop_state();
    }

    fn op_concat_matrix(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 6 {
            return Err(format!(
                "cm operator requires 6 operands, got {}",
                operands.len()
            ));
        }

        let a = get_f32(&operands[0])?;
        let b = get_f32(&operands[1])?;
        let c = get_f32(&operands[2])?;
        let d = get_f32(&operands[3])?;
        let e = get_f32(&operands[4])?;
        let f = get_f32(&operands[5])?;

        let matrix = Matrix::new(a, b, c, d, e, f);
        let state = self.state_mut();
        state.ctm = state.ctm.concat(&matrix);

        Ok(())
    }

    fn op_set_line_width(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 1 {
            return Err("w operator requires 1 operand".to_string());
        }

        let width = get_f32(&operands[0])?;
        self.state_mut().line_width = width;

        Ok(())
    }

    fn op_set_line_cap(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 1 {
            return Err("J operator requires 1 operand".to_string());
        }

        let cap = get_i32(&operands[0])?;
        self.state_mut().line_cap = cap;

        Ok(())
    }

    fn op_set_line_join(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 1 {
            return Err("j operator requires 1 operand".to_string());
        }

        let join = get_i32(&operands[0])?;
        self.state_mut().line_join = join;

        Ok(())
    }

    fn op_set_miter_limit(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 1 {
            return Err("M operator requires 1 operand".to_string());
        }

        let limit = get_f32(&operands[0])?;
        self.state_mut().miter_limit = limit;

        Ok(())
    }

    fn op_set_dash(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 2 {
            return Err("d operator requires 2 operands".to_string());
        }

        let pattern = match &operands[0] {
            Object::Array(arr) => arr.iter().filter_map(|obj| get_f32(obj).ok()).collect(),
            _ => return Err("Invalid dash pattern".to_string()),
        };

        let phase = get_f32(&operands[1])?;

        let state = self.state_mut();
        state.dash_pattern = pattern;
        state.dash_phase = phase;

        Ok(())
    }

    fn op_set_gstate(&mut self, _operands: &[Object]) -> Result<(), String> {
        // TODO: Look up graphics state from ExtGState resource dictionary
        Ok(())
    }

    // ========================================================================
    // Path Construction Operators
    // ========================================================================

    fn op_move_to(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 2 {
            return Err("m operator requires 2 operands".to_string());
        }

        let x = get_f32(&operands[0])?;
        let y = get_f32(&operands[1])?;

        if self.current_path.is_none() {
            self.current_path = Some(Path::new());
        }

        if let Some(ref mut path) = self.current_path {
            path.move_to(Point::new(x, y));
        }

        self.current_point = Some(Point::new(x, y));

        Ok(())
    }

    fn op_line_to(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 2 {
            return Err("l operator requires 2 operands".to_string());
        }

        let x = get_f32(&operands[0])?;
        let y = get_f32(&operands[1])?;

        if let Some(ref mut path) = self.current_path {
            path.line_to(Point::new(x, y));
        }

        self.current_point = Some(Point::new(x, y));

        Ok(())
    }

    fn op_curve_to(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 6 {
            return Err("c operator requires 6 operands".to_string());
        }

        let x1 = get_f32(&operands[0])?;
        let y1 = get_f32(&operands[1])?;
        let x2 = get_f32(&operands[2])?;
        let y2 = get_f32(&operands[3])?;
        let x3 = get_f32(&operands[4])?;
        let y3 = get_f32(&operands[5])?;

        if let Some(ref mut path) = self.current_path {
            path.curve_to(Point::new(x1, y1), Point::new(x2, y2), Point::new(x3, y3));
        }

        self.current_point = Some(Point::new(x3, y3));

        Ok(())
    }

    fn op_curve_to_v(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 4 {
            return Err("v operator requires 4 operands".to_string());
        }

        let current = self
            .current_point
            .ok_or("No current point for v operator")?;

        let x2 = get_f32(&operands[0])?;
        let y2 = get_f32(&operands[1])?;
        let x3 = get_f32(&operands[2])?;
        let y3 = get_f32(&operands[3])?;

        if let Some(ref mut path) = self.current_path {
            path.curve_to(current, Point::new(x2, y2), Point::new(x3, y3));
        }

        self.current_point = Some(Point::new(x3, y3));

        Ok(())
    }

    fn op_curve_to_y(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 4 {
            return Err("y operator requires 4 operands".to_string());
        }

        let x1 = get_f32(&operands[0])?;
        let y1 = get_f32(&operands[1])?;
        let x3 = get_f32(&operands[2])?;
        let y3 = get_f32(&operands[3])?;

        if let Some(ref mut path) = self.current_path {
            path.curve_to(Point::new(x1, y1), Point::new(x3, y3), Point::new(x3, y3));
        }

        self.current_point = Some(Point::new(x3, y3));

        Ok(())
    }

    fn op_close_path(&mut self) {
        if let Some(ref mut path) = self.current_path {
            path.close();
        }
    }

    fn op_rectangle(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 4 {
            return Err("re operator requires 4 operands".to_string());
        }

        let x = get_f32(&operands[0])?;
        let y = get_f32(&operands[1])?;
        let w = get_f32(&operands[2])?;
        let h = get_f32(&operands[3])?;

        if self.current_path.is_none() {
            self.current_path = Some(Path::new());
        }

        if let Some(ref mut path) = self.current_path {
            path.move_to(Point::new(x, y));
            path.line_to(Point::new(x + w, y));
            path.line_to(Point::new(x + w, y + h));
            path.line_to(Point::new(x, y + h));
            path.close();
        }

        self.current_point = Some(Point::new(x, y));

        Ok(())
    }

    // ========================================================================
    // Path Painting Operators
    // ========================================================================

    fn op_stroke<D: Device>(&mut self, device: &mut D) -> Result<(), String> {
        if let Some(path) = self.current_path.take() {
            let state = self.state();

            // Create stroke state
            let line_cap = line_cap_from_i32(state.line_cap);
            let stroke_state = crate::fitz::path::StrokeState {
                linewidth: state.line_width,
                miterlimit: state.miter_limit,
                start_cap: line_cap,
                dash_cap: line_cap,
                end_cap: line_cap,
                linejoin: line_join_from_i32(state.line_join),
                dash_phase: state.dash_phase,
                dash_pattern: state.dash_pattern.clone(),
            };

            // Call device stroke method
            device.stroke_path(
                &path,
                &stroke_state,
                &state.ctm,
                &state.stroke_colorspace,
                &state.stroke_color,
                state.stroke_alpha,
            );
        }

        self.current_point = None;
        Ok(())
    }

    fn op_close_and_stroke<D: Device>(&mut self, device: &mut D) -> Result<(), String> {
        self.op_close_path();
        self.op_stroke(device)
    }

    fn op_fill<D: Device>(&mut self, device: &mut D) -> Result<(), String> {
        if let Some(path) = self.current_path.take() {
            let state = self.state();

            // Call device fill method (non-zero winding rule)
            device.fill_path(
                &path,
                false, // even_odd = false (non-zero winding)
                &state.ctm,
                &state.fill_colorspace,
                &state.fill_color,
                state.fill_alpha,
            );
        }

        self.current_point = None;
        Ok(())
    }

    fn op_fill_even_odd<D: Device>(&mut self, device: &mut D) -> Result<(), String> {
        if let Some(path) = self.current_path.take() {
            let state = self.state();

            // Call device fill method (even-odd rule)
            device.fill_path(
                &path,
                true, // even_odd = true
                &state.ctm,
                &state.fill_colorspace,
                &state.fill_color,
                state.fill_alpha,
            );
        }

        self.current_point = None;
        Ok(())
    }

    fn op_fill_and_stroke<D: Device>(&mut self, device: &mut D) -> Result<(), String> {
        // Need to clone path since both operations consume it
        if let Some(ref path) = self.current_path {
            let state = self.state();

            // Fill
            device.fill_path(
                path,
                false,
                &state.ctm,
                &state.fill_colorspace,
                &state.fill_color,
                state.fill_alpha,
            );

            // Stroke
            let line_cap = line_cap_from_i32(state.line_cap);
            let stroke_state = crate::fitz::path::StrokeState {
                linewidth: state.line_width,
                miterlimit: state.miter_limit,
                start_cap: line_cap,
                dash_cap: line_cap,
                end_cap: line_cap,
                linejoin: line_join_from_i32(state.line_join),
                dash_phase: state.dash_phase,
                dash_pattern: state.dash_pattern.clone(),
            };

            device.stroke_path(
                path,
                &stroke_state,
                &state.ctm,
                &state.stroke_colorspace,
                &state.stroke_color,
                state.stroke_alpha,
            );
        }

        self.current_path = None;
        self.current_point = None;
        Ok(())
    }

    fn op_fill_and_stroke_even_odd<D: Device>(&mut self, device: &mut D) -> Result<(), String> {
        if let Some(ref path) = self.current_path {
            let state = self.state();

            // Fill with even-odd
            device.fill_path(
                path,
                true,
                &state.ctm,
                &state.fill_colorspace,
                &state.fill_color,
                state.fill_alpha,
            );

            // Stroke
            let line_cap = line_cap_from_i32(state.line_cap);
            let stroke_state = crate::fitz::path::StrokeState {
                linewidth: state.line_width,
                miterlimit: state.miter_limit,
                start_cap: line_cap,
                dash_cap: line_cap,
                end_cap: line_cap,
                linejoin: line_join_from_i32(state.line_join),
                dash_phase: state.dash_phase,
                dash_pattern: state.dash_pattern.clone(),
            };

            device.stroke_path(
                path,
                &stroke_state,
                &state.ctm,
                &state.stroke_colorspace,
                &state.stroke_color,
                state.stroke_alpha,
            );
        }

        self.current_path = None;
        self.current_point = None;
        Ok(())
    }

    fn op_close_fill_and_stroke<D: Device>(&mut self, device: &mut D) -> Result<(), String> {
        self.op_close_path();
        self.op_fill_and_stroke(device)
    }

    fn op_close_fill_and_stroke_even_odd<D: Device>(
        &mut self,
        device: &mut D,
    ) -> Result<(), String> {
        self.op_close_path();
        self.op_fill_and_stroke_even_odd(device)
    }

    fn op_end_path(&mut self) {
        self.current_path = None;
        self.current_point = None;
    }

    // ========================================================================
    // Clipping Path Operators
    // ========================================================================

    fn op_clip(&mut self) {
        // TODO: Implement clipping with current path (non-zero winding)
    }

    fn op_clip_even_odd(&mut self) {
        // TODO: Implement clipping with current path (even-odd rule)
    }

    // ========================================================================
    // Color Operators
    // ========================================================================

    fn op_set_stroke_colorspace(&mut self, _operands: &[Object]) -> Result<(), String> {
        // TODO: Set stroke colorspace from resource dictionary
        Ok(())
    }

    fn op_set_fill_colorspace(&mut self, _operands: &[Object]) -> Result<(), String> {
        // TODO: Set fill colorspace from resource dictionary
        Ok(())
    }

    fn op_set_stroke_color(&mut self, operands: &[Object]) -> Result<(), String> {
        let color: Vec<f32> = operands
            .iter()
            .filter_map(|obj| get_f32(obj).ok())
            .collect();

        if !color.is_empty() {
            self.state_mut().stroke_color = color;
        }

        Ok(())
    }

    fn op_set_fill_color(&mut self, operands: &[Object]) -> Result<(), String> {
        let color: Vec<f32> = operands
            .iter()
            .filter_map(|obj| get_f32(obj).ok())
            .collect();

        if !color.is_empty() {
            self.state_mut().fill_color = color;
        }

        Ok(())
    }

    fn op_set_stroke_gray(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 1 {
            return Err("G operator requires 1 operand".to_string());
        }

        let gray = get_f32(&operands[0])?;
        let state = self.state_mut();
        state.stroke_colorspace = Colorspace::device_gray();
        state.stroke_color = vec![gray];

        Ok(())
    }

    fn op_set_fill_gray(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 1 {
            return Err("g operator requires 1 operand".to_string());
        }

        let gray = get_f32(&operands[0])?;
        let state = self.state_mut();
        state.fill_colorspace = Colorspace::device_gray();
        state.fill_color = vec![gray];

        Ok(())
    }

    fn op_set_stroke_rgb(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 3 {
            return Err("RG operator requires 3 operands".to_string());
        }

        let r = get_f32(&operands[0])?;
        let g = get_f32(&operands[1])?;
        let b = get_f32(&operands[2])?;

        let state = self.state_mut();
        state.stroke_colorspace = Colorspace::device_rgb();
        state.stroke_color = vec![r, g, b];

        Ok(())
    }

    fn op_set_fill_rgb(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 3 {
            return Err("rg operator requires 3 operands".to_string());
        }

        let r = get_f32(&operands[0])?;
        let g = get_f32(&operands[1])?;
        let b = get_f32(&operands[2])?;

        let state = self.state_mut();
        state.fill_colorspace = Colorspace::device_rgb();
        state.fill_color = vec![r, g, b];

        Ok(())
    }

    fn op_set_stroke_cmyk(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 4 {
            return Err("K operator requires 4 operands".to_string());
        }

        let c = get_f32(&operands[0])?;
        let m = get_f32(&operands[1])?;
        let y = get_f32(&operands[2])?;
        let k = get_f32(&operands[3])?;

        let state = self.state_mut();
        state.stroke_colorspace = Colorspace::device_cmyk();
        state.stroke_color = vec![c, m, y, k];

        Ok(())
    }

    fn op_set_fill_cmyk(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 4 {
            return Err("k operator requires 4 operands".to_string());
        }

        let c = get_f32(&operands[0])?;
        let m = get_f32(&operands[1])?;
        let y = get_f32(&operands[2])?;
        let k = get_f32(&operands[3])?;

        let state = self.state_mut();
        state.fill_colorspace = Colorspace::device_cmyk();
        state.fill_color = vec![c, m, y, k];

        Ok(())
    }

    // ========================================================================
    // Text Object Operators
    // ========================================================================

    fn op_begin_text(&mut self) {
        let state = self.state_mut();
        state.text_matrix = Matrix::IDENTITY;
        state.text_line_matrix = Matrix::IDENTITY;
    }

    fn op_end_text(&mut self) {
        // Text object complete
    }

    // ========================================================================
    // Text Positioning Operators
    // ========================================================================

    fn op_text_move(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 2 {
            return Err("Td operator requires 2 operands".to_string());
        }

        let tx = get_f32(&operands[0])?;
        let ty = get_f32(&operands[1])?;

        let state = self.state_mut();
        let translate = Matrix::translate(tx, ty);
        state.text_line_matrix = state.text_line_matrix.concat(&translate);
        state.text_matrix = state.text_line_matrix;

        Ok(())
    }

    fn op_text_move_set_leading(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 2 {
            return Err("TD operator requires 2 operands".to_string());
        }

        let ty = get_f32(&operands[1])?;
        self.state_mut().leading = -ty;

        self.op_text_move(operands)
    }

    fn op_text_set_matrix(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 6 {
            return Err("Tm operator requires 6 operands".to_string());
        }

        let a = get_f32(&operands[0])?;
        let b = get_f32(&operands[1])?;
        let c = get_f32(&operands[2])?;
        let d = get_f32(&operands[3])?;
        let e = get_f32(&operands[4])?;
        let f = get_f32(&operands[5])?;

        let matrix = Matrix::new(a, b, c, d, e, f);
        let state = self.state_mut();
        state.text_matrix = matrix;
        state.text_line_matrix = matrix;

        Ok(())
    }

    fn op_text_next_line(&mut self) {
        let leading = self.state().leading;
        let _ = self.op_text_move(&[Object::Real(0.0), Object::Real(-leading as f64)]);
    }

    // ========================================================================
    // Text State Operators
    // ========================================================================

    fn op_set_char_spacing(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 1 {
            return Err("Tc operator requires 1 operand".to_string());
        }

        let spacing = get_f32(&operands[0])?;
        self.state_mut().char_spacing = spacing;

        Ok(())
    }

    fn op_set_word_spacing(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 1 {
            return Err("Tw operator requires 1 operand".to_string());
        }

        let spacing = get_f32(&operands[0])?;
        self.state_mut().word_spacing = spacing;

        Ok(())
    }

    fn op_set_horizontal_scaling(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 1 {
            return Err("Tz operator requires 1 operand".to_string());
        }

        let scaling = get_f32(&operands[0])?;
        self.state_mut().horizontal_scaling = scaling;

        Ok(())
    }

    fn op_set_leading(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 1 {
            return Err("TL operator requires 1 operand".to_string());
        }

        let leading = get_f32(&operands[0])?;
        self.state_mut().leading = leading;

        Ok(())
    }

    fn op_set_font(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 2 {
            return Err("Tf operator requires 2 operands".to_string());
        }

        let font = match &operands[0] {
            Object::Name(n) => n.as_str().to_string(),
            _ => return Err("Invalid font name".to_string()),
        };
        let size = get_f32(&operands[1])?;

        let state = self.state_mut();
        state.font = Some(font);
        state.font_size = size;

        Ok(())
    }

    fn op_set_text_render_mode(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 1 {
            return Err("Tr operator requires 1 operand".to_string());
        }

        let mode = get_i32(&operands[0])?;
        self.state_mut().text_render_mode = mode;

        Ok(())
    }

    fn op_set_text_rise(&mut self, operands: &[Object]) -> Result<(), String> {
        if operands.len() != 1 {
            return Err("Ts operator requires 1 operand".to_string());
        }

        let rise = get_f32(&operands[0])?;
        self.state_mut().text_rise = rise;

        Ok(())
    }

    // ========================================================================
    // Text Showing Operators
    // ========================================================================

    fn op_show_text<D: Device>(
        &mut self,
        operands: &[Object],
        _device: &mut D,
    ) -> Result<(), String> {
        if operands.len() != 1 {
            return Err("Tj operator requires 1 operand".to_string());
        }

        let _text_str = match &operands[0] {
            Object::String(s) => s.as_str(),
            _ => return Err("Invalid text string".to_string()),
        };

        // TODO: Full text rendering requires:
        // 1. Font loading from resources
        // 2. Glyph lookup and positioning
        // 3. Creating Text object with TextSpan
        // 4. Calling device.fill_text() or device.stroke_text()
        //
        // For now, we successfully parse the text operator but don't render.
        // This will be implemented in Phase 2 (Fonts & Text).

        Ok(())
    }

    fn op_show_text_adjusted<D: Device>(
        &mut self,
        operands: &[Object],
        device: &mut D,
    ) -> Result<(), String> {
        if operands.len() != 1 {
            return Err("TJ operator requires 1 operand".to_string());
        }

        let array = match &operands[0] {
            Object::Array(arr) => arr,
            _ => return Err("Invalid text array".to_string()),
        };

        for item in array {
            if let Object::String(s) = item {
                let _ = self.op_show_text(&[Object::String(s.clone())], device);
            } else if let Ok(_offset) = get_f32(item) {
                // TODO: Adjust text position based on offset
            }
        }

        Ok(())
    }

    fn op_show_text_next_line<D: Device>(
        &mut self,
        operands: &[Object],
        device: &mut D,
    ) -> Result<(), String> {
        self.op_text_next_line();
        self.op_show_text(operands, device)
    }

    fn op_show_text_next_line_with_spacing<D: Device>(
        &mut self,
        operands: &[Object],
        device: &mut D,
    ) -> Result<(), String> {
        if operands.len() != 3 {
            return Err("\" operator requires 3 operands".to_string());
        }

        self.op_set_word_spacing(&operands[0..1])?;
        self.op_set_char_spacing(&operands[1..2])?;
        self.op_show_text_next_line(&operands[2..3], device)
    }

    // ========================================================================
    // XObject Operators
    // ========================================================================

    fn op_paint_xobject<D: Device>(
        &mut self,
        _operands: &[Object],
        _device: &mut D,
    ) -> Result<(), String> {
        // TODO: Look up XObject from resources and paint it
        Ok(())
    }

    // ========================================================================
    // Image Operators
    // ========================================================================

    fn op_begin_inline_image(&mut self, _operands: &[Object]) -> Result<(), String> {
        // TODO: Begin inline image parsing
        Ok(())
    }

    fn op_inline_image_data(&mut self, _operands: &[Object]) -> Result<(), String> {
        // TODO: Parse inline image data
        Ok(())
    }

    fn op_end_inline_image<D: Device>(&mut self, _device: &mut D) -> Result<(), String> {
        // TODO: Paint inline image
        Ok(())
    }

    // ========================================================================
    // Marked Content Operators
    // ========================================================================

    fn op_marked_content_point(&mut self, _operands: &[Object]) -> Result<(), String> {
        Ok(())
    }

    fn op_marked_content_point_with_props(&mut self, _operands: &[Object]) -> Result<(), String> {
        Ok(())
    }

    fn op_begin_marked_content(&mut self, _operands: &[Object]) -> Result<(), String> {
        Ok(())
    }

    fn op_begin_marked_content_with_props(&mut self, _operands: &[Object]) -> Result<(), String> {
        Ok(())
    }

    fn op_end_marked_content(&mut self) -> Result<(), String> {
        Ok(())
    }

    // ========================================================================
    // Compatibility Operators
    // ========================================================================

    fn op_begin_compat(&mut self) {
        // Begin compatibility section
    }

    fn op_end_compat(&mut self) {
        // End compatibility section
    }

    // ========================================================================
    // Shading Operator
    // ========================================================================

    fn op_shade<D: Device>(&mut self, _operands: &[Object], _device: &mut D) -> Result<(), String> {
        // TODO: Paint shading pattern
        Ok(())
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get f32 value from Object
fn get_f32(obj: &Object) -> Result<f32, String> {
    match obj {
        Object::Int(i) => Ok(*i as f32),
        Object::Real(f) => Ok(*f as f32),
        _ => Err("Expected number".to_string()),
    }
}

/// Get i32 value from Object
fn get_i32(obj: &Object) -> Result<i32, String> {
    match obj {
        Object::Int(i) => Ok(*i as i32),
        Object::Real(f) => Ok(*f as i32),
        _ => Err("Expected integer".to_string()),
    }
}

/// Convert PDF line cap value to LineCap enum
fn line_cap_from_i32(cap: i32) -> LineCap {
    match cap {
        0 => LineCap::Butt,
        1 => LineCap::Round,
        2 => LineCap::Square,
        _ => LineCap::Butt, // Default to butt
    }
}

/// Convert PDF line join value to LineJoin enum
fn line_join_from_i32(join: i32) -> LineJoin {
    match join {
        0 => LineJoin::Miter,
        1 => LineJoin::Round,
        2 => LineJoin::Bevel,
        _ => LineJoin::Miter, // Default to miter
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpreter_creation() {
        let interp = Interpreter::new();
        assert_eq!(interp.state_stack.len(), 1);
    }

    #[test]
    fn test_state_stack() {
        let mut interp = Interpreter::new();
        interp.push_state();
        assert_eq!(interp.state_stack.len(), 2);
        interp.pop_state();
        assert_eq!(interp.state_stack.len(), 1);
    }

    #[test]
    fn test_matrix_concat() {
        let mut interp = Interpreter::new();
        let operands = vec![
            Object::Real(2.0),
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(2.0),
            Object::Real(0.0),
            Object::Real(0.0),
        ];

        interp.op_concat_matrix(&operands).unwrap();

        let state = interp.state();
        assert_eq!(state.ctm.a, 2.0);
        assert_eq!(state.ctm.d, 2.0);
    }

    #[test]
    fn test_get_f32() {
        assert_eq!(get_f32(&Object::Int(42)).unwrap(), 42.0);
        assert_eq!(get_f32(&Object::Real(3.5)).unwrap(), 3.5f32);
        assert!(get_f32(&Object::Null).is_err());
    }
}
