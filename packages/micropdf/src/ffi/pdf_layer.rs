//! PDF Layer (Optional Content Groups) FFI Module
//!
//! Provides support for PDF Optional Content Groups (OCG) which allow
//! layers of content to be selectively shown or hidden.

use crate::ffi::{Handle, HandleStore};
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::{Arc, LazyLock, Mutex};

// ============================================================================
// Type Aliases
// ============================================================================

type ContextHandle = Handle;
type DocumentHandle = Handle;

// ============================================================================
// Layer Config UI Types
// ============================================================================

/// UI element type for layer configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub enum LayerConfigUiType {
    #[default]
    Label = 0,
    Checkbox = 1,
    Radiobox = 2,
}

impl LayerConfigUiType {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => LayerConfigUiType::Label,
            1 => LayerConfigUiType::Checkbox,
            2 => LayerConfigUiType::Radiobox,
            _ => LayerConfigUiType::Label,
        }
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            LayerConfigUiType::Label => "label",
            LayerConfigUiType::Checkbox => "checkbox",
            LayerConfigUiType::Radiobox => "radiobox",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "label" => LayerConfigUiType::Label,
            "checkbox" => LayerConfigUiType::Checkbox,
            "radiobox" => LayerConfigUiType::Radiobox,
            _ => LayerConfigUiType::Label,
        }
    }
}

// ============================================================================
// Layer Configuration Structures
// ============================================================================

/// Layer configuration info
#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct LayerConfig {
    /// Configuration name
    pub name: Option<String>,
    /// Configuration creator
    pub creator: Option<String>,
}

/// Layer UI element info
#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct LayerConfigUi {
    /// Display text
    pub text: Option<String>,
    /// Nesting depth in UI
    pub depth: i32,
    /// UI element type
    pub ui_type: LayerConfigUiType,
    /// Whether selected/enabled
    pub selected: bool,
    /// Whether locked (cannot be changed)
    pub locked: bool,
}

// ============================================================================
// Layer Structure
// ============================================================================

/// Individual layer (OCG)
#[derive(Debug, Clone)]
pub struct Layer {
    /// Layer name
    pub name: String,
    /// Whether enabled
    pub enabled: bool,
    /// Layer index
    pub index: i32,
}

// ============================================================================
// OCG Descriptor
// ============================================================================

/// Optional Content Group descriptor
#[derive(Debug)]
pub struct OcgDescriptor {
    /// All layers in the document
    pub layers: Vec<Layer>,
    /// Layer configurations
    pub configs: Vec<LayerConfig>,
    /// Currently selected config
    pub current_config: i32,
    /// UI elements for current config
    pub ui_elements: Vec<LayerConfigUi>,
    /// Whether changes have been made
    pub modified: bool,
}

impl Default for OcgDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

impl OcgDescriptor {
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            configs: vec![LayerConfig {
                name: Some("Default".to_string()),
                creator: None,
            }],
            current_config: 0,
            ui_elements: Vec::new(),
            modified: false,
        }
    }

    /// Add a layer
    pub fn add_layer(&mut self, name: &str, enabled: bool) -> i32 {
        let index = self.layers.len() as i32;
        self.layers.push(Layer {
            name: name.to_string(),
            enabled,
            index,
        });
        index
    }

    /// Get layer count
    pub fn layer_count(&self) -> i32 {
        self.layers.len() as i32
    }

    /// Get layer by index
    pub fn get_layer(&self, index: i32) -> Option<&Layer> {
        self.layers.get(index as usize)
    }

    /// Get layer by index (mutable)
    pub fn get_layer_mut(&mut self, index: i32) -> Option<&mut Layer> {
        self.layers.get_mut(index as usize)
    }

    /// Get config count
    pub fn config_count(&self) -> i32 {
        self.configs.len() as i32
    }

    /// Get config by index
    pub fn get_config(&self, index: i32) -> Option<&LayerConfig> {
        self.configs.get(index as usize)
    }

    /// Add a configuration
    pub fn add_config(&mut self, name: Option<String>, creator: Option<String>) -> i32 {
        let index = self.configs.len() as i32;
        self.configs.push(LayerConfig { name, creator });
        index
    }

    /// Select a configuration
    pub fn select_config(&mut self, config_num: i32) {
        if config_num >= 0 && (config_num as usize) < self.configs.len() {
            self.current_config = config_num;
            self.modified = true;
        }
    }

    /// Get UI element count
    pub fn ui_count(&self) -> i32 {
        self.ui_elements.len() as i32
    }

    /// Get UI element by index
    pub fn get_ui(&self, index: i32) -> Option<&LayerConfigUi> {
        self.ui_elements.get(index as usize)
    }

    /// Get UI element by index (mutable)
    pub fn get_ui_mut(&mut self, index: i32) -> Option<&mut LayerConfigUi> {
        self.ui_elements.get_mut(index as usize)
    }

    /// Add a UI element
    pub fn add_ui_element(
        &mut self,
        text: Option<String>,
        depth: i32,
        ui_type: LayerConfigUiType,
        selected: bool,
        locked: bool,
    ) -> i32 {
        let index = self.ui_elements.len() as i32;
        self.ui_elements.push(LayerConfigUi {
            text,
            depth,
            ui_type,
            selected,
            locked,
        });
        index
    }
}

// ============================================================================
// Global Handle Store
// ============================================================================

/// Store for OCG descriptors (keyed by document handle)
pub static OCG_STORE: LazyLock<Mutex<HashMap<DocumentHandle, OcgDescriptor>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// ============================================================================
// FFI Functions - Layer Count and Enumeration
// ============================================================================

/// Count the number of layer configurations.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_count_layer_configs(_ctx: ContextHandle, doc: DocumentHandle) -> i32 {
    let store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get(&doc) {
        return ocg.config_count();
    }
    0
}

/// Count the number of layers.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_count_layers(_ctx: ContextHandle, doc: DocumentHandle) -> i32 {
    let store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get(&doc) {
        return ocg.layer_count();
    }
    0
}

/// Get layer name by index.
/// Caller must free the returned string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_layer_name(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    layer: i32,
) -> *const c_char {
    let store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get(&doc) {
        if let Some(l) = ocg.get_layer(layer) {
            if let Ok(cstr) = CString::new(l.name.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null()
}

/// Check if a layer is enabled.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_layer_is_enabled(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    layer: i32,
) -> i32 {
    let store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get(&doc) {
        if let Some(l) = ocg.get_layer(layer) {
            return if l.enabled { 1 } else { 0 };
        }
    }
    0
}

/// Enable or disable a layer.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_enable_layer(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    layer: i32,
    enabled: i32,
) {
    let mut store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get_mut(&doc) {
        if let Some(l) = ocg.get_layer_mut(layer) {
            l.enabled = enabled != 0;
            ocg.modified = true;
        }
    }
}

// ============================================================================
// FFI Functions - Layer Configuration Info
// ============================================================================

/// C-compatible layer config structure for FFI
#[repr(C)]
pub struct FfiLayerConfig {
    pub name: *const c_char,
    pub creator: *const c_char,
}

/// Get layer configuration info.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_layer_config_info(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    config_num: i32,
    info: *mut FfiLayerConfig,
) {
    if info.is_null() {
        return;
    }

    let store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get(&doc) {
        if let Some(config) = ocg.get_config(config_num) {
            unsafe {
                (*info).name = config
                    .name
                    .as_ref()
                    .and_then(|s| CString::new(s.clone()).ok())
                    .map(|c| c.into_raw() as *const c_char)
                    .unwrap_or(ptr::null());

                (*info).creator = config
                    .creator
                    .as_ref()
                    .and_then(|s| CString::new(s.clone()).ok())
                    .map(|c| c.into_raw() as *const c_char)
                    .unwrap_or(ptr::null());
            }
        }
    }
}

/// Get layer configuration creator.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_layer_config_creator(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    config_num: i32,
) -> *const c_char {
    let store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get(&doc) {
        if let Some(config) = ocg.get_config(config_num) {
            if let Some(ref creator) = config.creator {
                if let Ok(cstr) = CString::new(creator.clone()) {
                    return cstr.into_raw();
                }
            }
        }
    }
    ptr::null()
}

/// Get layer configuration name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_layer_config_name(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    config_num: i32,
) -> *const c_char {
    let store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get(&doc) {
        if let Some(config) = ocg.get_config(config_num) {
            if let Some(ref name) = config.name {
                if let Ok(cstr) = CString::new(name.clone()) {
                    return cstr.into_raw();
                }
            }
        }
    }
    ptr::null()
}

/// Select a layer configuration.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_select_layer_config(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    config_num: i32,
) {
    let mut store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get_mut(&doc) {
        ocg.select_config(config_num);
    }
}

// ============================================================================
// FFI Functions - Layer Config UI
// ============================================================================

/// Count UI elements in current layer configuration.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_count_layer_config_ui(_ctx: ContextHandle, doc: DocumentHandle) -> i32 {
    let store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get(&doc) {
        return ocg.ui_count();
    }
    0
}

/// C-compatible layer config UI structure for FFI
#[repr(C)]
pub struct FfiLayerConfigUi {
    pub text: *const c_char,
    pub depth: i32,
    pub ui_type: i32,
    pub selected: i32,
    pub locked: i32,
}

/// Get layer config UI element info.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_layer_config_ui_info(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    ui: i32,
    info: *mut FfiLayerConfigUi,
) {
    if info.is_null() {
        return;
    }

    let store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get(&doc) {
        if let Some(ui_elem) = ocg.get_ui(ui) {
            unsafe {
                (*info).text = ui_elem
                    .text
                    .as_ref()
                    .and_then(|s| CString::new(s.clone()).ok())
                    .map(|c| c.into_raw() as *const c_char)
                    .unwrap_or(ptr::null());
                (*info).depth = ui_elem.depth;
                (*info).ui_type = ui_elem.ui_type as i32;
                (*info).selected = if ui_elem.selected { 1 } else { 0 };
                (*info).locked = if ui_elem.locked { 1 } else { 0 };
            }
        }
    }
}

/// Free the text field of an FfiLayerConfigUi structure.
/// This should be called after using pdf_layer_config_ui_info to avoid memory leaks.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_free_layer_config_ui_text(info: *mut FfiLayerConfigUi) {
    if info.is_null() {
        return;
    }
    unsafe {
        if !(*info).text.is_null() {
            let _ = CString::from_raw((*info).text as *mut c_char);
            (*info).text = ptr::null();
        }
    }
}

/// Select a UI element (checkbox/radiobox).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_select_layer_config_ui(_ctx: ContextHandle, doc: DocumentHandle, ui: i32) {
    let mut store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get_mut(&doc) {
        if let Some(ui_elem) = ocg.get_ui_mut(ui) {
            if !ui_elem.locked && ui_elem.ui_type != LayerConfigUiType::Label {
                ui_elem.selected = true;
                ocg.modified = true;
            }
        }
    }
}

/// Deselect a UI element.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_deselect_layer_config_ui(_ctx: ContextHandle, doc: DocumentHandle, ui: i32) {
    let mut store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get_mut(&doc) {
        if let Some(ui_elem) = ocg.get_ui_mut(ui) {
            if !ui_elem.locked && ui_elem.ui_type != LayerConfigUiType::Label {
                ui_elem.selected = false;
                ocg.modified = true;
            }
        }
    }
}

/// Toggle a UI element.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_toggle_layer_config_ui(_ctx: ContextHandle, doc: DocumentHandle, ui: i32) {
    let mut store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get_mut(&doc) {
        if let Some(ui_elem) = ocg.get_ui_mut(ui) {
            if !ui_elem.locked && ui_elem.ui_type != LayerConfigUiType::Label {
                ui_elem.selected = !ui_elem.selected;
                ocg.modified = true;
            }
        }
    }
}

// ============================================================================
// FFI Functions - UI Type Conversion
// ============================================================================

/// Convert UI type to string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_layer_config_ui_type_to_string(ui_type: i32) -> *const c_char {
    let t = LayerConfigUiType::from_i32(ui_type);
    match t {
        LayerConfigUiType::Label => c"label".as_ptr(),
        LayerConfigUiType::Checkbox => c"checkbox".as_ptr(),
        LayerConfigUiType::Radiobox => c"radiobox".as_ptr(),
    }
}

/// Convert string to UI type.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_layer_config_ui_type_from_string(s: *const c_char) -> i32 {
    if s.is_null() {
        return LayerConfigUiType::Label as i32;
    }

    let str = unsafe { CStr::from_ptr(s) };
    if let Ok(s) = str.to_str() {
        return LayerConfigUiType::from_string(s) as i32;
    }
    LayerConfigUiType::Label as i32
}

// ============================================================================
// FFI Functions - OCG Management
// ============================================================================

/// Read OCG descriptor from document.
/// Creates an OCG descriptor if none exists.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_read_ocg(_ctx: ContextHandle, doc: DocumentHandle) -> Handle {
    let mut store = OCG_STORE.lock().unwrap();
    if !store.contains_key(&doc) {
        store.insert(doc, OcgDescriptor::new());
    }
    // Return the document handle as the OCG handle (they're linked)
    doc
}

/// Drop OCG descriptor for a document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_ocg(_ctx: ContextHandle, doc: DocumentHandle) {
    let mut store = OCG_STORE.lock().unwrap();
    store.remove(&doc);
}

/// Check if an OCG is hidden.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_is_ocg_hidden(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    _rdb: Handle,
    _usage: *const c_char,
    _ocg: Handle,
) -> i32 {
    // For now, return 0 (not hidden) - actual implementation would
    // check the OCG state based on the usage string and current config
    let store = OCG_STORE.lock().unwrap();
    if store.contains_key(&doc) {
        return 0; // Not hidden
    }
    0
}

/// Set current layer configuration as the default.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_layer_config_as_default(_ctx: ContextHandle, doc: DocumentHandle) {
    let mut store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get_mut(&doc) {
        // Mark as saved (clear modified flag)
        ocg.modified = false;
    }
}

// ============================================================================
// FFI Functions - Layer Management (Additional)
// ============================================================================

/// Add a new layer to the document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_add_layer(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    name: *const c_char,
    enabled: i32,
) -> i32 {
    if name.is_null() {
        return -1;
    }

    let name_str = unsafe {
        match CStr::from_ptr(name).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return -1,
        }
    };

    let mut store = OCG_STORE.lock().unwrap();

    // Ensure OCG exists for document
    if !store.contains_key(&doc) {
        store.insert(doc, OcgDescriptor::new());
    }

    if let Some(ocg) = store.get_mut(&doc) {
        return ocg.add_layer(&name_str, enabled != 0);
    }
    -1
}

/// Add a layer configuration.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_add_layer_config(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    name: *const c_char,
    creator: *const c_char,
) -> i32 {
    let name_opt = if !name.is_null() {
        unsafe { CStr::from_ptr(name).to_str().ok().map(String::from) }
    } else {
        None
    };

    let creator_opt = if !creator.is_null() {
        unsafe { CStr::from_ptr(creator).to_str().ok().map(String::from) }
    } else {
        None
    };

    let mut store = OCG_STORE.lock().unwrap();

    if !store.contains_key(&doc) {
        store.insert(doc, OcgDescriptor::new());
    }

    if let Some(ocg) = store.get_mut(&doc) {
        return ocg.add_config(name_opt, creator_opt);
    }
    -1
}

/// Add a UI element to the current configuration.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_add_layer_config_ui(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    text: *const c_char,
    depth: i32,
    ui_type: i32,
    selected: i32,
    locked: i32,
) -> i32 {
    let text_opt = if !text.is_null() {
        unsafe { CStr::from_ptr(text).to_str().ok().map(String::from) }
    } else {
        None
    };

    let mut store = OCG_STORE.lock().unwrap();

    if let Some(ocg) = store.get_mut(&doc) {
        return ocg.add_ui_element(
            text_opt,
            depth,
            LayerConfigUiType::from_i32(ui_type),
            selected != 0,
            locked != 0,
        );
    }
    -1
}

/// Check if OCG has unsaved changes.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_layer_has_unsaved_changes(_ctx: ContextHandle, doc: DocumentHandle) -> i32 {
    let store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get(&doc) {
        return if ocg.modified { 1 } else { 0 };
    }
    0
}

/// Get current layer configuration index.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_get_current_layer_config(_ctx: ContextHandle, doc: DocumentHandle) -> i32 {
    let store = OCG_STORE.lock().unwrap();
    if let Some(ocg) = store.get(&doc) {
        return ocg.current_config;
    }
    -1
}

/// Free a string allocated by layer functions.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_layer_free_string(_ctx: ContextHandle, s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
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
    fn test_ocg_descriptor_new() {
        let ocg = OcgDescriptor::new();
        assert_eq!(ocg.layer_count(), 0);
        assert_eq!(ocg.config_count(), 1); // Default config
        assert_eq!(ocg.ui_count(), 0);
    }

    #[test]
    fn test_add_layer() {
        let mut ocg = OcgDescriptor::new();

        let idx0 = ocg.add_layer("Layer 1", true);
        let idx1 = ocg.add_layer("Layer 2", false);

        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);
        assert_eq!(ocg.layer_count(), 2);

        let l0 = ocg.get_layer(0).unwrap();
        assert_eq!(l0.name, "Layer 1");
        assert!(l0.enabled);

        let l1 = ocg.get_layer(1).unwrap();
        assert_eq!(l1.name, "Layer 2");
        assert!(!l1.enabled);
    }

    #[test]
    fn test_enable_disable_layer() {
        let mut ocg = OcgDescriptor::new();
        ocg.add_layer("Test Layer", true);

        assert!(ocg.get_layer(0).unwrap().enabled);

        if let Some(l) = ocg.get_layer_mut(0) {
            l.enabled = false;
        }

        assert!(!ocg.get_layer(0).unwrap().enabled);
    }

    #[test]
    fn test_add_config() {
        let mut ocg = OcgDescriptor::new();

        let idx = ocg.add_config(
            Some("Print Config".to_string()),
            Some("Adobe Acrobat".to_string()),
        );

        assert_eq!(idx, 1); // After default config
        assert_eq!(ocg.config_count(), 2);

        let config = ocg.get_config(1).unwrap();
        assert_eq!(config.name, Some("Print Config".to_string()));
        assert_eq!(config.creator, Some("Adobe Acrobat".to_string()));
    }

    #[test]
    fn test_select_config() {
        let mut ocg = OcgDescriptor::new();
        ocg.add_config(Some("Config 1".to_string()), None);
        ocg.add_config(Some("Config 2".to_string()), None);

        assert_eq!(ocg.current_config, 0);
        assert!(!ocg.modified);

        ocg.select_config(2);

        assert_eq!(ocg.current_config, 2);
        assert!(ocg.modified);
    }

    #[test]
    fn test_add_ui_element() {
        let mut ocg = OcgDescriptor::new();

        let idx = ocg.add_ui_element(
            Some("Show Watermarks".to_string()),
            0,
            LayerConfigUiType::Checkbox,
            true,
            false,
        );

        assert_eq!(idx, 0);
        assert_eq!(ocg.ui_count(), 1);

        let ui = ocg.get_ui(0).unwrap();
        assert_eq!(ui.text, Some("Show Watermarks".to_string()));
        assert_eq!(ui.depth, 0);
        assert_eq!(ui.ui_type, LayerConfigUiType::Checkbox);
        assert!(ui.selected);
        assert!(!ui.locked);
    }

    #[test]
    fn test_toggle_ui() {
        let mut ocg = OcgDescriptor::new();
        ocg.add_ui_element(
            Some("Toggle Me".to_string()),
            0,
            LayerConfigUiType::Checkbox,
            false,
            false,
        );

        assert!(!ocg.get_ui(0).unwrap().selected);

        if let Some(ui) = ocg.get_ui_mut(0) {
            ui.selected = !ui.selected;
        }

        assert!(ocg.get_ui(0).unwrap().selected);
    }

    #[test]
    fn test_locked_ui() {
        let mut ocg = OcgDescriptor::new();
        ocg.add_ui_element(
            Some("Locked".to_string()),
            0,
            LayerConfigUiType::Checkbox,
            true,
            true, // locked
        );

        let ui = ocg.get_ui(0).unwrap();
        assert!(ui.locked);
        assert!(ui.selected);
    }

    #[test]
    fn test_ui_type_conversion() {
        assert_eq!(LayerConfigUiType::from_i32(0), LayerConfigUiType::Label);
        assert_eq!(LayerConfigUiType::from_i32(1), LayerConfigUiType::Checkbox);
        assert_eq!(LayerConfigUiType::from_i32(2), LayerConfigUiType::Radiobox);
        assert_eq!(LayerConfigUiType::from_i32(99), LayerConfigUiType::Label);

        assert_eq!(LayerConfigUiType::Label.to_string(), "label");
        assert_eq!(LayerConfigUiType::Checkbox.to_string(), "checkbox");
        assert_eq!(LayerConfigUiType::Radiobox.to_string(), "radiobox");

        assert_eq!(
            LayerConfigUiType::from_string("checkbox"),
            LayerConfigUiType::Checkbox
        );
        assert_eq!(
            LayerConfigUiType::from_string("RADIOBOX"),
            LayerConfigUiType::Radiobox
        );
    }

    #[test]
    fn test_ffi_count_layers() {
        let doc: DocumentHandle = 999;

        // Initially no OCG
        assert_eq!(pdf_count_layers(0, doc), 0);

        // Create OCG for document
        pdf_read_ocg(0, doc);

        // Add layers via FFI
        let name = CString::new("Test Layer").unwrap();
        pdf_add_layer(0, doc, name.as_ptr(), 1);

        assert_eq!(pdf_count_layers(0, doc), 1);

        // Cleanup
        pdf_drop_ocg(0, doc);
    }

    #[test]
    fn test_ffi_layer_enable_disable() {
        let doc: DocumentHandle = 998;

        pdf_read_ocg(0, doc);

        let name = CString::new("Layer").unwrap();
        pdf_add_layer(0, doc, name.as_ptr(), 1);

        assert_eq!(pdf_layer_is_enabled(0, doc, 0), 1);

        pdf_enable_layer(0, doc, 0, 0);
        assert_eq!(pdf_layer_is_enabled(0, doc, 0), 0);

        pdf_enable_layer(0, doc, 0, 1);
        assert_eq!(pdf_layer_is_enabled(0, doc, 0), 1);

        pdf_drop_ocg(0, doc);
    }

    #[test]
    fn test_ffi_layer_config() {
        let doc: DocumentHandle = 997;

        pdf_read_ocg(0, doc);

        assert_eq!(pdf_count_layer_configs(0, doc), 1); // Default

        let name = CString::new("Print").unwrap();
        let creator = CString::new("Test").unwrap();
        pdf_add_layer_config(0, doc, name.as_ptr(), creator.as_ptr());

        assert_eq!(pdf_count_layer_configs(0, doc), 2);

        pdf_select_layer_config(0, doc, 1);
        assert_eq!(pdf_get_current_layer_config(0, doc), 1);

        pdf_drop_ocg(0, doc);
    }

    #[test]
    fn test_ffi_ui_toggle() {
        let doc: DocumentHandle = 996;

        pdf_read_ocg(0, doc);

        let text = CString::new("Checkbox").unwrap();
        pdf_add_layer_config_ui(
            0,
            doc,
            text.as_ptr(),
            0,
            LayerConfigUiType::Checkbox as i32,
            0,
            0,
        );

        assert_eq!(pdf_count_layer_config_ui(0, doc), 1);

        // Test toggle
        pdf_select_layer_config_ui(0, doc, 0);

        let mut info = FfiLayerConfigUi {
            text: ptr::null(),
            depth: 0,
            ui_type: 0,
            selected: 0,
            locked: 0,
        };
        pdf_layer_config_ui_info(0, doc, 0, &mut info);
        assert_eq!(info.selected, 1);
        pdf_free_layer_config_ui_text(&mut info); // Free the text to avoid leak

        pdf_toggle_layer_config_ui(0, doc, 0);
        pdf_layer_config_ui_info(0, doc, 0, &mut info);
        assert_eq!(info.selected, 0);
        pdf_free_layer_config_ui_text(&mut info); // Free the text to avoid leak

        pdf_drop_ocg(0, doc);
    }
}
