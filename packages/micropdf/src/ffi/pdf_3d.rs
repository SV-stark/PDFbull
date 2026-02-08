//! PDF 3D Annotation FFI Module
//!
//! Provides support for 3D annotations in PDF documents, including
//! U3D and PRC format streams, 3D views, and activation settings.

use crate::ffi::{Handle, HandleStore};
use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::LazyLock;

// ============================================================================
// Type Aliases
// ============================================================================

type ContextHandle = Handle;
type DocumentHandle = Handle;
type AnnotHandle = Handle;

// ============================================================================
// 3D Stream Format Constants
// ============================================================================

/// Universal 3D format
pub const PDF_3D_FORMAT_U3D: i32 = 0;
/// PRC (Product Representation Compact) format
pub const PDF_3D_FORMAT_PRC: i32 = 1;
/// Unknown format
pub const PDF_3D_FORMAT_UNKNOWN: i32 = -1;

// ============================================================================
// 3D Activation Mode Constants
// ============================================================================

/// Explicitly activated by user action
pub const PDF_3D_ACTIVATION_EXPLICIT: i32 = 0;
/// Activated when page is opened
pub const PDF_3D_ACTIVATION_PAGE_OPEN: i32 = 1;
/// Activated when page is visible
pub const PDF_3D_ACTIVATION_PAGE_VISIBLE: i32 = 2;

// ============================================================================
// 3D Deactivation Mode Constants
// ============================================================================

/// Explicitly deactivated by user action
pub const PDF_3D_DEACTIVATION_EXPLICIT: i32 = 0;
/// Deactivated when page is closed
pub const PDF_3D_DEACTIVATION_PAGE_CLOSE: i32 = 1;
/// Deactivated when page is invisible
pub const PDF_3D_DEACTIVATION_PAGE_INVISIBLE: i32 = 2;

// ============================================================================
// 3D Rendering Mode Constants
// ============================================================================

/// Solid rendering
pub const PDF_3D_RENDER_SOLID: i32 = 0;
/// Solid wireframe
pub const PDF_3D_RENDER_SOLID_WIREFRAME: i32 = 1;
/// Transparent
pub const PDF_3D_RENDER_TRANSPARENT: i32 = 2;
/// Transparent wireframe
pub const PDF_3D_RENDER_TRANSPARENT_WIREFRAME: i32 = 3;
/// Bounding box
pub const PDF_3D_RENDER_BOUNDING_BOX: i32 = 4;
/// Transparent bounding box
pub const PDF_3D_RENDER_TRANSPARENT_BBOX: i32 = 5;
/// Transparent bounding box outline
pub const PDF_3D_RENDER_TRANSPARENT_BBOX_OUTLINE: i32 = 6;
/// Wireframe
pub const PDF_3D_RENDER_WIREFRAME: i32 = 7;
/// Shaded wireframe
pub const PDF_3D_RENDER_SHADED_WIREFRAME: i32 = 8;
/// Hidden wireframe
pub const PDF_3D_RENDER_HIDDEN_WIREFRAME: i32 = 9;
/// Vertices
pub const PDF_3D_RENDER_VERTICES: i32 = 10;
/// Shaded vertices
pub const PDF_3D_RENDER_SHADED_VERTICES: i32 = 11;
/// Illustration
pub const PDF_3D_RENDER_ILLUSTRATION: i32 = 12;
/// Solid outline
pub const PDF_3D_RENDER_SOLID_OUTLINE: i32 = 13;
/// Shaded illustration
pub const PDF_3D_RENDER_SHADED_ILLUSTRATION: i32 = 14;

// ============================================================================
// 3D Lighting Scheme Constants
// ============================================================================

/// Artwork lighting (from 3D data)
pub const PDF_3D_LIGHTING_ARTWORK: i32 = 0;
/// No lighting
pub const PDF_3D_LIGHTING_NONE: i32 = 1;
/// White lighting
pub const PDF_3D_LIGHTING_WHITE: i32 = 2;
/// Day lighting
pub const PDF_3D_LIGHTING_DAY: i32 = 3;
/// Night lighting
pub const PDF_3D_LIGHTING_NIGHT: i32 = 4;
/// Hard lighting
pub const PDF_3D_LIGHTING_HARD: i32 = 5;
/// Primary lighting
pub const PDF_3D_LIGHTING_PRIMARY: i32 = 6;
/// Blue lighting
pub const PDF_3D_LIGHTING_BLUE: i32 = 7;
/// Red lighting
pub const PDF_3D_LIGHTING_RED: i32 = 8;
/// Cube lighting
pub const PDF_3D_LIGHTING_CUBE: i32 = 9;
/// CAD lighting
pub const PDF_3D_LIGHTING_CAD: i32 = 10;
/// Headlamp lighting
pub const PDF_3D_LIGHTING_HEADLAMP: i32 = 11;

// ============================================================================
// 3D Projection Type Constants
// ============================================================================

/// Perspective projection
pub const PDF_3D_PROJECTION_PERSPECTIVE: i32 = 0;
/// Orthographic projection
pub const PDF_3D_PROJECTION_ORTHOGRAPHIC: i32 = 1;

// ============================================================================
// 3D Camera Position
// ============================================================================

/// 3D camera/view position
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct Camera3D {
    /// Camera position X
    pub pos_x: f32,
    /// Camera position Y
    pub pos_y: f32,
    /// Camera position Z
    pub pos_z: f32,
    /// Camera target X
    pub target_x: f32,
    /// Camera target Y
    pub target_y: f32,
    /// Camera target Z
    pub target_z: f32,
    /// Up vector X
    pub up_x: f32,
    /// Up vector Y
    pub up_y: f32,
    /// Up vector Z
    pub up_z: f32,
    /// Field of view (degrees, for perspective)
    pub fov: f32,
    /// Projection type
    pub projection: i32,
}

impl Camera3D {
    pub fn new() -> Self {
        Self {
            pos_x: 0.0,
            pos_y: 0.0,
            pos_z: 10.0,
            target_x: 0.0,
            target_y: 0.0,
            target_z: 0.0,
            up_x: 0.0,
            up_y: 1.0,
            up_z: 0.0,
            fov: 60.0,
            projection: PDF_3D_PROJECTION_PERSPECTIVE,
        }
    }

    pub fn perspective(pos: [f32; 3], target: [f32; 3], fov: f32) -> Self {
        Self {
            pos_x: pos[0],
            pos_y: pos[1],
            pos_z: pos[2],
            target_x: target[0],
            target_y: target[1],
            target_z: target[2],
            up_x: 0.0,
            up_y: 1.0,
            up_z: 0.0,
            fov,
            projection: PDF_3D_PROJECTION_PERSPECTIVE,
        }
    }

    pub fn orthographic(pos: [f32; 3], target: [f32; 3]) -> Self {
        Self {
            pos_x: pos[0],
            pos_y: pos[1],
            pos_z: pos[2],
            target_x: target[0],
            target_y: target[1],
            target_z: target[2],
            up_x: 0.0,
            up_y: 1.0,
            up_z: 0.0,
            fov: 0.0,
            projection: PDF_3D_PROJECTION_ORTHOGRAPHIC,
        }
    }
}

// ============================================================================
// 3D View
// ============================================================================

/// A named 3D view
#[derive(Debug, Clone)]
pub struct View3D {
    /// View name
    pub name: String,
    /// External name (display name)
    pub external_name: String,
    /// Camera position
    pub camera: Camera3D,
    /// Rendering mode
    pub render_mode: i32,
    /// Lighting scheme
    pub lighting: i32,
    /// Background color (RGBA)
    pub background: [f32; 4],
}

impl Default for View3D {
    fn default() -> Self {
        Self::new()
    }
}

impl View3D {
    pub fn new() -> Self {
        Self {
            name: "Default".to_string(),
            external_name: "Default View".to_string(),
            camera: Camera3D::new(),
            render_mode: PDF_3D_RENDER_SOLID,
            lighting: PDF_3D_LIGHTING_DAY,
            background: [1.0, 1.0, 1.0, 1.0],
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn with_camera(mut self, camera: Camera3D) -> Self {
        self.camera = camera;
        self
    }

    pub fn with_render_mode(mut self, mode: i32) -> Self {
        self.render_mode = mode;
        self
    }
}

// ============================================================================
// 3D Annotation Data
// ============================================================================

/// 3D annotation data
#[derive(Debug, Clone)]
pub struct Annotation3D {
    /// Annotation handle
    pub annot: AnnotHandle,
    /// 3D stream format
    pub format: i32,
    /// 3D stream data
    pub data: Vec<u8>,
    /// Views
    pub views: Vec<View3D>,
    /// Default view index
    pub default_view: usize,
    /// Activation mode
    pub activation: i32,
    /// Deactivation mode
    pub deactivation: i32,
    /// Show toolbar
    pub show_toolbar: bool,
    /// Show navigation panel
    pub show_navigation: bool,
    /// Interactive
    pub interactive: bool,
}

impl Default for Annotation3D {
    fn default() -> Self {
        Self::new()
    }
}

impl Annotation3D {
    pub fn new() -> Self {
        Self {
            annot: 0,
            format: PDF_3D_FORMAT_UNKNOWN,
            data: Vec::new(),
            views: vec![View3D::new()],
            default_view: 0,
            activation: PDF_3D_ACTIVATION_EXPLICIT,
            deactivation: PDF_3D_DEACTIVATION_PAGE_CLOSE,
            show_toolbar: true,
            show_navigation: true,
            interactive: true,
        }
    }

    pub fn with_u3d_data(mut self, data: &[u8]) -> Self {
        self.format = PDF_3D_FORMAT_U3D;
        self.data = data.to_vec();
        self
    }

    pub fn with_prc_data(mut self, data: &[u8]) -> Self {
        self.format = PDF_3D_FORMAT_PRC;
        self.data = data.to_vec();
        self
    }

    pub fn add_view(&mut self, view: View3D) {
        self.views.push(view);
    }
}

// ============================================================================
// Global Handle Store
// ============================================================================

pub static ANNOTATIONS_3D: LazyLock<HandleStore<Annotation3D>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Annotation Management
// ============================================================================

/// Create a new 3D annotation data structure.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_3d_annotation(_ctx: ContextHandle) -> Handle {
    let annot = Annotation3D::new();
    ANNOTATIONS_3D.insert(annot)
}

/// Drop a 3D annotation data structure.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_3d_annotation(_ctx: ContextHandle, annot: Handle) {
    ANNOTATIONS_3D.remove(annot);
}

/// Set the 3D stream data (U3D format).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_set_u3d_data(
    _ctx: ContextHandle,
    annot: Handle,
    data: *const u8,
    len: usize,
) -> i32 {
    if data.is_null() || len == 0 {
        return 0;
    }

    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let mut a = a.lock().unwrap();
        a.format = PDF_3D_FORMAT_U3D;
        unsafe {
            a.data = std::slice::from_raw_parts(data, len).to_vec();
        }
        return 1;
    }
    0
}

/// Set the 3D stream data (PRC format).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_set_prc_data(
    _ctx: ContextHandle,
    annot: Handle,
    data: *const u8,
    len: usize,
) -> i32 {
    if data.is_null() || len == 0 {
        return 0;
    }

    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let mut a = a.lock().unwrap();
        a.format = PDF_3D_FORMAT_PRC;
        unsafe {
            a.data = std::slice::from_raw_parts(data, len).to_vec();
        }
        return 1;
    }
    0
}

/// Get the 3D stream format.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_get_format(_ctx: ContextHandle, annot: Handle) -> i32 {
    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let a = a.lock().unwrap();
        return a.format;
    }
    PDF_3D_FORMAT_UNKNOWN
}

/// Get the 3D stream data.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_get_data(
    _ctx: ContextHandle,
    annot: Handle,
    len_out: *mut usize,
) -> *const u8 {
    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let a = a.lock().unwrap();
        if !a.data.is_empty() {
            if !len_out.is_null() {
                unsafe {
                    *len_out = a.data.len();
                }
            }
            return a.data.as_ptr();
        }
    }

    if !len_out.is_null() {
        unsafe {
            *len_out = 0;
        }
    }
    ptr::null()
}

// ============================================================================
// FFI Functions - View Management
// ============================================================================

/// Add a 3D view.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_add_view(_ctx: ContextHandle, annot: Handle, name: *const c_char) -> i32 {
    if name.is_null() {
        return -1;
    }

    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let mut a = a.lock().unwrap();
        let view_name = unsafe { CStr::from_ptr(name).to_string_lossy().to_string() };
        let view = View3D::new().with_name(&view_name);
        let idx = a.views.len();
        a.views.push(view);
        return idx as i32;
    }
    -1
}

/// Get the number of views.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_view_count(_ctx: ContextHandle, annot: Handle) -> i32 {
    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let a = a.lock().unwrap();
        return a.views.len() as i32;
    }
    0
}

/// Get view name by index.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_get_view_name(
    _ctx: ContextHandle,
    annot: Handle,
    index: i32,
) -> *mut c_char {
    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let a = a.lock().unwrap();
        if let Some(view) = a.views.get(index as usize) {
            if let Ok(cstr) = CString::new(view.name.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Set the default view.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_set_default_view(_ctx: ContextHandle, annot: Handle, index: i32) -> i32 {
    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let mut a = a.lock().unwrap();
        if (index as usize) < a.views.len() {
            a.default_view = index as usize;
            return 1;
        }
    }
    0
}

/// Get the default view index.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_get_default_view(_ctx: ContextHandle, annot: Handle) -> i32 {
    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let a = a.lock().unwrap();
        return a.default_view as i32;
    }
    0
}

// ============================================================================
// FFI Functions - View Properties
// ============================================================================

/// Set view camera.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_set_view_camera(
    _ctx: ContextHandle,
    annot: Handle,
    view_index: i32,
    camera: *const Camera3D,
) -> i32 {
    if camera.is_null() {
        return 0;
    }

    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let mut a = a.lock().unwrap();
        if let Some(view) = a.views.get_mut(view_index as usize) {
            view.camera = unsafe { *camera };
            return 1;
        }
    }
    0
}

/// Get view camera.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_get_view_camera(
    _ctx: ContextHandle,
    annot: Handle,
    view_index: i32,
    camera_out: *mut Camera3D,
) -> i32 {
    if camera_out.is_null() {
        return 0;
    }

    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let a = a.lock().unwrap();
        if let Some(view) = a.views.get(view_index as usize) {
            unsafe {
                *camera_out = view.camera;
            }
            return 1;
        }
    }
    0
}

/// Set view render mode.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_set_view_render_mode(
    _ctx: ContextHandle,
    annot: Handle,
    view_index: i32,
    mode: i32,
) -> i32 {
    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let mut a = a.lock().unwrap();
        if let Some(view) = a.views.get_mut(view_index as usize) {
            view.render_mode = mode;
            return 1;
        }
    }
    0
}

/// Set view lighting.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_set_view_lighting(
    _ctx: ContextHandle,
    annot: Handle,
    view_index: i32,
    lighting: i32,
) -> i32 {
    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let mut a = a.lock().unwrap();
        if let Some(view) = a.views.get_mut(view_index as usize) {
            view.lighting = lighting;
            return 1;
        }
    }
    0
}

/// Set view background color.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_set_view_background(
    _ctx: ContextHandle,
    annot: Handle,
    view_index: i32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
) -> i32 {
    if let Some(ann) = ANNOTATIONS_3D.get(annot) {
        let mut ann = ann.lock().unwrap();
        if let Some(view) = ann.views.get_mut(view_index as usize) {
            view.background = [r, g, b, a];
            return 1;
        }
    }
    0
}

// ============================================================================
// FFI Functions - Activation Settings
// ============================================================================

/// Set activation mode.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_set_activation(_ctx: ContextHandle, annot: Handle, mode: i32) -> i32 {
    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let mut a = a.lock().unwrap();
        a.activation = mode;
        return 1;
    }
    0
}

/// Get activation mode.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_get_activation(_ctx: ContextHandle, annot: Handle) -> i32 {
    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let a = a.lock().unwrap();
        return a.activation;
    }
    PDF_3D_ACTIVATION_EXPLICIT
}

/// Set deactivation mode.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_set_deactivation(_ctx: ContextHandle, annot: Handle, mode: i32) -> i32 {
    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let mut a = a.lock().unwrap();
        a.deactivation = mode;
        return 1;
    }
    0
}

/// Get deactivation mode.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_get_deactivation(_ctx: ContextHandle, annot: Handle) -> i32 {
    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let a = a.lock().unwrap();
        return a.deactivation;
    }
    PDF_3D_DEACTIVATION_PAGE_CLOSE
}

// ============================================================================
// FFI Functions - UI Settings
// ============================================================================

/// Set toolbar visibility.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_set_toolbar(_ctx: ContextHandle, annot: Handle, show: i32) -> i32 {
    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let mut a = a.lock().unwrap();
        a.show_toolbar = show != 0;
        return 1;
    }
    0
}

/// Set navigation panel visibility.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_set_navigation(_ctx: ContextHandle, annot: Handle, show: i32) -> i32 {
    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let mut a = a.lock().unwrap();
        a.show_navigation = show != 0;
        return 1;
    }
    0
}

/// Set interactive mode.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_set_interactive(
    _ctx: ContextHandle,
    annot: Handle,
    interactive: i32,
) -> i32 {
    if let Some(a) = ANNOTATIONS_3D.get(annot) {
        let mut a = a.lock().unwrap();
        a.interactive = interactive != 0;
        return 1;
    }
    0
}

// ============================================================================
// FFI Functions - Utility
// ============================================================================

/// Free a string returned by 3D functions.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Get format name string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_format_to_string(_ctx: ContextHandle, format: i32) -> *mut c_char {
    let s = match format {
        PDF_3D_FORMAT_U3D => "U3D",
        PDF_3D_FORMAT_PRC => "PRC",
        _ => "Unknown",
    };

    if let Ok(cstr) = CString::new(s) {
        return cstr.into_raw();
    }
    ptr::null_mut()
}

/// Get render mode name string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_render_mode_to_string(_ctx: ContextHandle, mode: i32) -> *mut c_char {
    let s = match mode {
        PDF_3D_RENDER_SOLID => "Solid",
        PDF_3D_RENDER_SOLID_WIREFRAME => "SolidWireframe",
        PDF_3D_RENDER_TRANSPARENT => "Transparent",
        PDF_3D_RENDER_TRANSPARENT_WIREFRAME => "TransparentWireframe",
        PDF_3D_RENDER_BOUNDING_BOX => "BoundingBox",
        PDF_3D_RENDER_WIREFRAME => "Wireframe",
        PDF_3D_RENDER_VERTICES => "Vertices",
        PDF_3D_RENDER_ILLUSTRATION => "Illustration",
        _ => "Unknown",
    };

    if let Ok(cstr) = CString::new(s) {
        return cstr.into_raw();
    }
    ptr::null_mut()
}

/// Get lighting scheme name string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_3d_lighting_to_string(_ctx: ContextHandle, lighting: i32) -> *mut c_char {
    let s = match lighting {
        PDF_3D_LIGHTING_ARTWORK => "Artwork",
        PDF_3D_LIGHTING_NONE => "None",
        PDF_3D_LIGHTING_WHITE => "White",
        PDF_3D_LIGHTING_DAY => "Day",
        PDF_3D_LIGHTING_NIGHT => "Night",
        PDF_3D_LIGHTING_HARD => "Hard",
        PDF_3D_LIGHTING_PRIMARY => "Primary",
        PDF_3D_LIGHTING_BLUE => "Blue",
        PDF_3D_LIGHTING_RED => "Red",
        PDF_3D_LIGHTING_CUBE => "Cube",
        PDF_3D_LIGHTING_CAD => "CAD",
        PDF_3D_LIGHTING_HEADLAMP => "Headlamp",
        _ => "Unknown",
    };

    if let Ok(cstr) = CString::new(s) {
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
    fn test_format_constants() {
        assert_eq!(PDF_3D_FORMAT_U3D, 0);
        assert_eq!(PDF_3D_FORMAT_PRC, 1);
        assert_eq!(PDF_3D_FORMAT_UNKNOWN, -1);
    }

    #[test]
    fn test_activation_constants() {
        assert_eq!(PDF_3D_ACTIVATION_EXPLICIT, 0);
        assert_eq!(PDF_3D_ACTIVATION_PAGE_OPEN, 1);
        assert_eq!(PDF_3D_ACTIVATION_PAGE_VISIBLE, 2);
    }

    #[test]
    fn test_render_mode_constants() {
        assert_eq!(PDF_3D_RENDER_SOLID, 0);
        assert_eq!(PDF_3D_RENDER_WIREFRAME, 7);
        assert_eq!(PDF_3D_RENDER_ILLUSTRATION, 12);
    }

    #[test]
    fn test_lighting_constants() {
        assert_eq!(PDF_3D_LIGHTING_ARTWORK, 0);
        assert_eq!(PDF_3D_LIGHTING_DAY, 3);
        assert_eq!(PDF_3D_LIGHTING_CAD, 10);
    }

    #[test]
    fn test_camera_3d() {
        let camera = Camera3D::new();
        assert_eq!(camera.pos_z, 10.0);
        assert_eq!(camera.fov, 60.0);
        assert_eq!(camera.projection, PDF_3D_PROJECTION_PERSPECTIVE);

        let camera = Camera3D::perspective([0.0, 0.0, 5.0], [0.0, 0.0, 0.0], 45.0);
        assert_eq!(camera.pos_z, 5.0);
        assert_eq!(camera.fov, 45.0);

        let camera = Camera3D::orthographic([0.0, 10.0, 0.0], [0.0, 0.0, 0.0]);
        assert_eq!(camera.projection, PDF_3D_PROJECTION_ORTHOGRAPHIC);
    }

    #[test]
    fn test_view_3d() {
        let view = View3D::new()
            .with_name("Test View")
            .with_render_mode(PDF_3D_RENDER_WIREFRAME);

        assert_eq!(view.name, "Test View");
        assert_eq!(view.render_mode, PDF_3D_RENDER_WIREFRAME);
    }

    #[test]
    fn test_annotation_3d() {
        let mut annot = Annotation3D::new();
        assert_eq!(annot.format, PDF_3D_FORMAT_UNKNOWN);
        assert_eq!(annot.views.len(), 1);

        annot = annot.with_u3d_data(b"fake u3d data");
        assert_eq!(annot.format, PDF_3D_FORMAT_U3D);
        assert_eq!(annot.data, b"fake u3d data");

        annot.add_view(View3D::new().with_name("Second"));
        assert_eq!(annot.views.len(), 2);
    }

    #[test]
    fn test_ffi_annotation() {
        let ctx = 0;

        let annot = pdf_new_3d_annotation(ctx);
        assert!(annot > 0);

        assert_eq!(pdf_3d_get_format(ctx, annot), PDF_3D_FORMAT_UNKNOWN);
        assert_eq!(pdf_3d_view_count(ctx, annot), 1);

        pdf_drop_3d_annotation(ctx, annot);
    }

    #[test]
    fn test_ffi_set_data() {
        let ctx = 0;
        let annot = pdf_new_3d_annotation(ctx);

        let data = b"U3D test data";
        let result = pdf_3d_set_u3d_data(ctx, annot, data.as_ptr(), data.len());
        assert_eq!(result, 1);
        assert_eq!(pdf_3d_get_format(ctx, annot), PDF_3D_FORMAT_U3D);

        let mut len: usize = 0;
        let ptr = pdf_3d_get_data(ctx, annot, &mut len);
        assert!(!ptr.is_null());
        assert_eq!(len, data.len());

        pdf_drop_3d_annotation(ctx, annot);
    }

    #[test]
    fn test_ffi_views() {
        let ctx = 0;
        let annot = pdf_new_3d_annotation(ctx);

        let name = CString::new("Top View").unwrap();
        let idx = pdf_3d_add_view(ctx, annot, name.as_ptr());
        assert!(idx >= 0);
        assert_eq!(pdf_3d_view_count(ctx, annot), 2);

        pdf_3d_set_default_view(ctx, annot, idx);
        assert_eq!(pdf_3d_get_default_view(ctx, annot), idx);

        pdf_drop_3d_annotation(ctx, annot);
    }

    #[test]
    fn test_ffi_view_properties() {
        let ctx = 0;
        let annot = pdf_new_3d_annotation(ctx);

        pdf_3d_set_view_render_mode(ctx, annot, 0, PDF_3D_RENDER_WIREFRAME);
        pdf_3d_set_view_lighting(ctx, annot, 0, PDF_3D_LIGHTING_CAD);
        pdf_3d_set_view_background(ctx, annot, 0, 0.5, 0.5, 0.5, 1.0);

        pdf_drop_3d_annotation(ctx, annot);
    }

    #[test]
    fn test_ffi_activation() {
        let ctx = 0;
        let annot = pdf_new_3d_annotation(ctx);

        pdf_3d_set_activation(ctx, annot, PDF_3D_ACTIVATION_PAGE_OPEN);
        assert_eq!(
            pdf_3d_get_activation(ctx, annot),
            PDF_3D_ACTIVATION_PAGE_OPEN
        );

        pdf_3d_set_deactivation(ctx, annot, PDF_3D_DEACTIVATION_PAGE_INVISIBLE);
        assert_eq!(
            pdf_3d_get_deactivation(ctx, annot),
            PDF_3D_DEACTIVATION_PAGE_INVISIBLE
        );

        pdf_drop_3d_annotation(ctx, annot);
    }

    #[test]
    fn test_ffi_format_string() {
        let ctx = 0;

        let s = pdf_3d_format_to_string(ctx, PDF_3D_FORMAT_U3D);
        assert!(!s.is_null());
        unsafe {
            let str = CStr::from_ptr(s).to_string_lossy();
            assert_eq!(str, "U3D");
            pdf_3d_free_string(s);
        }

        let s = pdf_3d_format_to_string(ctx, PDF_3D_FORMAT_PRC);
        assert!(!s.is_null());
        unsafe {
            let str = CStr::from_ptr(s).to_string_lossy();
            assert_eq!(str, "PRC");
            pdf_3d_free_string(s);
        }
    }
}
