//! C FFI for pdf_form - MuPDF compatible form handling
//!
//! Provides FFI bindings for PDF interactive forms (AcroForms).

use super::{Handle, HandleStore};
use crate::pdf::form::{ChoiceOption, FieldFlags, Form, FormField, TextFormat, WidgetType};
use std::ffi::{CStr, c_char};
use std::sync::LazyLock;

/// Form storage
pub static FORMS: LazyLock<HandleStore<Form>> = LazyLock::new(HandleStore::default);

/// Form field storage (widgets)
pub static FORM_FIELDS: LazyLock<HandleStore<FormField>> = LazyLock::new(HandleStore::default);

// ============================================================================
// Form Access
// ============================================================================

/// Get form from document
#[unsafe(no_mangle)]
pub extern "C" fn pdf_form(_ctx: Handle, _doc: Handle) -> Handle {
    // Create or return existing form
    let form = Form::new();
    FORMS.insert(form)
}

/// Keep form reference
#[unsafe(no_mangle)]
pub extern "C" fn pdf_keep_form(_ctx: Handle, form: Handle) -> Handle {
    if FORMS.get(form).is_some() {
        return form;
    }
    0
}

/// Drop form reference
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_form(_ctx: Handle, form: Handle) {
    FORMS.remove(form);
}

// ============================================================================
// Field Iteration
// ============================================================================

/// Get first widget on page
#[unsafe(no_mangle)]
pub extern "C" fn pdf_first_widget(_ctx: Handle, page: Handle) -> Handle {
    if let Some(p) = super::document::PAGES.get(page) {
        if let Ok(guard) = p.lock() {
            return guard.first_widget().unwrap_or(0);
        }
    }
    0
}

/// Get next widget
#[unsafe(no_mangle)]
pub extern "C" fn pdf_next_widget(_ctx: Handle, widget: Handle) -> Handle {
    // Find the page this widget belongs to by searching all loaded pages
    if FORM_FIELDS.get(widget).is_some() {
        for page_handle in 1..10000 {
            // Reasonable page limit
            if let Some(p) = super::document::PAGES.get(page_handle) {
                if let Ok(guard) = p.lock() {
                    if guard.widgets.contains(&widget) {
                        return guard.next_widget(widget).unwrap_or(0);
                    }
                }
            }
        }
    }
    0
}

// ============================================================================
// Field Creation
// ============================================================================

/// Create a text field
#[unsafe(no_mangle)]
pub extern "C" fn pdf_create_text_field(
    _ctx: Handle,
    _form: Handle,
    name: *const std::ffi::c_char,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    max_len: i32,
) -> Handle {
    if name.is_null() {
        return 0;
    }

    if let Some(field_name) = super::safe_helpers::c_str_to_str(name) {
        let rect = crate::fitz::geometry::Rect {
            x0: x,
            y0: y,
            x1: x + width,
            y1: y + height,
        };

        let max_length = if max_len > 0 {
            Some(max_len as usize)
        } else {
            None
        };

        let field = FormField::text_field(field_name.to_string(), rect, max_length);
        return FORM_FIELDS.insert(field);
    }

    0
}

/// Create a checkbox
#[unsafe(no_mangle)]
pub extern "C" fn pdf_create_checkbox(
    _ctx: Handle,
    _form: Handle,
    name: *const std::ffi::c_char,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    checked: i32,
) -> Handle {
    if name.is_null() {
        return 0;
    }

    if let Some(field_name) = super::safe_helpers::c_str_to_str(name) {
        let rect = crate::fitz::geometry::Rect {
            x0: x,
            y0: y,
            x1: x + width,
            y1: y + height,
        };

        let field = FormField::checkbox(field_name.to_string(), rect, checked != 0);
        return FORM_FIELDS.insert(field);
    }

    0
}

/// Create a push button
#[unsafe(no_mangle)]
pub extern "C" fn pdf_create_push_button(
    _ctx: Handle,
    _form: Handle,
    name: *const std::ffi::c_char,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    caption: *const std::ffi::c_char,
) -> Handle {
    if name.is_null() {
        return 0;
    }

    if let Some(field_name) = super::safe_helpers::c_str_to_str(name) {
        let rect = crate::fitz::geometry::Rect {
            x0: x,
            y0: y,
            x1: x + width,
            y1: y + height,
        };

        let caption_str = if caption.is_null() {
            ""
        } else {
            super::safe_helpers::c_str_to_str(caption).unwrap_or("")
        };

        let field = FormField::push_button(field_name.to_string(), rect, caption_str);
        return FORM_FIELDS.insert(field);
    }

    0
}

/// Create a combo box (dropdown)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_create_combo_box(
    _ctx: Handle,
    _form: Handle,
    name: *const std::ffi::c_char,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Handle {
    if name.is_null() {
        return 0;
    }

    if let Some(field_name) = super::safe_helpers::c_str_to_str(name) {
        let rect = crate::fitz::geometry::Rect {
            x0: x,
            y0: y,
            x1: x + width,
            y1: y + height,
        };

        let field = FormField::combo_box(field_name.to_string(), rect, Vec::new());
        return FORM_FIELDS.insert(field);
    }

    0
}

/// Create a signature field
#[unsafe(no_mangle)]
pub extern "C" fn pdf_create_signature_field(
    _ctx: Handle,
    _form: Handle,
    name: *const std::ffi::c_char,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Handle {
    if name.is_null() {
        return 0;
    }

    if let Some(field_name) = super::safe_helpers::c_str_to_str(name) {
        let rect = crate::fitz::geometry::Rect {
            x0: x,
            y0: y,
            x1: x + width,
            y1: y + height,
        };

        let field = FormField::signature(field_name.to_string(), rect);
        return FORM_FIELDS.insert(field);
    }

    0
}

// ============================================================================
// Field Properties
// ============================================================================

/// Get field name
///
/// # Safety
/// Caller must ensure buf points to valid memory of at least size bytes
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_name(
    _ctx: Handle,
    field: Handle,
    buf: *mut std::ffi::c_char,
    size: i32,
) -> i32 {
    if buf.is_null() || size <= 0 {
        return 0;
    }

    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return super::safe_helpers::str_to_c_buffer(guard.name(), buf, size);
        }
    }

    0
}

/// Get field type
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_type(_ctx: Handle, field: Handle) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return match guard.field_type() {
                WidgetType::Unknown => -1,
                WidgetType::Button => 0,
                WidgetType::Checkbox => 1,
                WidgetType::ComboBox => 2,
                WidgetType::ListBox => 3,
                WidgetType::RadioButton => 4,
                WidgetType::Signature => 5,
                WidgetType::Text => 6,
            };
        }
    }
    -1
}

/// Get field value
///
/// # Safety
/// Caller must ensure buf points to valid memory of at least size bytes
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_value(
    _ctx: Handle,
    field: Handle,
    buf: *mut std::ffi::c_char,
    size: i32,
) -> i32 {
    if buf.is_null() || size <= 0 {
        return 0;
    }

    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return super::safe_helpers::str_to_c_buffer(guard.value(), buf, size);
        }
    }

    0
}

/// Set field value
///
/// # Safety
/// Caller must ensure value is a valid null-terminated C string
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_field_value(
    _ctx: Handle,
    field: Handle,
    value: *const std::ffi::c_char,
) -> i32 {
    if value.is_null() {
        return 0;
    }

    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(mut guard) = f.lock() {
            if let Some(val_str) = super::safe_helpers::c_str_to_str(value) {
                if guard.set_value(val_str.to_string()).is_ok() {
                    return 1;
                }
            }
        }
    }

    0
}

/// Get field rectangle
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_rect(_ctx: Handle, field: Handle) -> super::geometry::fz_rect {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            let rect = guard.rect();
            return super::geometry::fz_rect {
                x0: rect.x0,
                y0: rect.y0,
                x1: rect.x1,
                y1: rect.y1,
            };
        }
    }

    super::geometry::fz_rect {
        x0: 0.0,
        y0: 0.0,
        x1: 0.0,
        y1: 0.0,
    }
}

/// Get field flags
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_flags(_ctx: Handle, field: Handle) -> u32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return guard.flags().value();
        }
    }
    0
}

/// Set field flags
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_field_flags(_ctx: Handle, field: Handle, flags: u32) {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(mut guard) = f.lock() {
            guard.set_flags(FieldFlags::new(flags));
        }
    }
}

// ============================================================================
// Field State
// ============================================================================

/// Check if field is read-only
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_is_read_only(_ctx: Handle, field: Handle) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return guard.is_read_only() as i32;
        }
    }
    0
}

/// Check if field is required
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_is_required(_ctx: Handle, field: Handle) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return guard.is_required() as i32;
        }
    }
    0
}

/// Check if checkbox/radio button is checked
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_is_checked(_ctx: Handle, field: Handle) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return guard.is_checked() as i32;
        }
    }
    0
}

/// Set checkbox/radio button checked state
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_field_checked(_ctx: Handle, field: Handle, checked: i32) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(mut guard) = f.lock() {
            if guard.set_checked(checked != 0).is_ok() {
                return 1;
            }
        }
    }
    0
}

/// Check if text field is multiline
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_is_multiline(_ctx: Handle, field: Handle) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return guard.is_multiline() as i32;
        }
    }
    0
}

/// Check if text field is password
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_is_password(_ctx: Handle, field: Handle) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return guard.is_password() as i32;
        }
    }
    0
}

/// Check if signature field is signed
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_is_signed(_ctx: Handle, field: Handle) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return guard.is_signed() as i32;
        }
    }
    0
}

// ============================================================================
// Text Field Properties
// ============================================================================

/// Get text field maximum length
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_max_len(_ctx: Handle, field: Handle) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return guard.max_len().map(|l| l as i32).unwrap_or(0);
        }
    }
    0
}

/// Set text field maximum length
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_field_max_len(_ctx: Handle, field: Handle, max_len: i32) {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(mut guard) = f.lock() {
            let max = if max_len > 0 {
                Some(max_len as usize)
            } else {
                None
            };
            guard.set_max_len(max);
        }
    }
}

/// Get text field format
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_text_format(_ctx: Handle, field: Handle) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return match guard.text_format() {
                TextFormat::None => 0,
                TextFormat::Number => 1,
                TextFormat::Special => 2,
                TextFormat::Date => 3,
                TextFormat::Time => 4,
            };
        }
    }
    0
}

// ============================================================================
// Choice Field Properties
// ============================================================================

/// Get number of choice options
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_choice_count(_ctx: Handle, field: Handle) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return guard.options().len() as i32;
        }
    }
    0
}

/// Get choice option label
///
/// # Safety
/// Caller must ensure buf points to valid memory of at least size bytes
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_choice_label(
    _ctx: Handle,
    field: Handle,
    index: i32,
    buf: *mut std::ffi::c_char,
    size: i32,
) -> i32 {
    if buf.is_null() || size <= 0 || index < 0 {
        return 0;
    }

    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            let options = guard.options();
            if let Some(option) = options.get(index as usize) {
                return super::safe_helpers::str_to_c_buffer(&option.label, buf, size);
            }
        }
    }

    0
}

/// Get choice option value
///
/// # Safety
/// Caller must ensure buf points to valid memory of at least size bytes
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_choice_value(
    _ctx: Handle,
    field: Handle,
    index: i32,
    buf: *mut std::ffi::c_char,
    size: i32,
) -> i32 {
    if buf.is_null() || size <= 0 || index < 0 {
        return 0;
    }

    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            let options = guard.options();
            if let Some(option) = options.get(index as usize) {
                return super::safe_helpers::str_to_c_buffer(&option.value, buf, size);
            }
        }
    }

    0
}

/// Add choice option
///
/// # Safety
/// Caller must ensure label and value are valid null-terminated C strings
#[unsafe(no_mangle)]
pub extern "C" fn pdf_add_field_choice(
    _ctx: Handle,
    field: Handle,
    label: *const std::ffi::c_char,
    value: *const std::ffi::c_char,
) -> i32 {
    if label.is_null() || value.is_null() {
        return 0;
    }

    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(mut guard) = f.lock() {
            if let (Some(label_str), Some(value_str)) = (
                super::safe_helpers::c_str_to_str(label),
                super::safe_helpers::c_str_to_str(value),
            ) {
                let mut options = guard.options().to_vec();
                options.push(ChoiceOption::new(
                    label_str.to_string(),
                    value_str.to_string(),
                ));
                guard.set_options(options);
                return 1;
            }
        }
    }

    0
}

// ============================================================================
// Form-level Operations
// ============================================================================

/// Get form field count
#[unsafe(no_mangle)]
pub extern "C" fn pdf_form_field_count(_ctx: Handle, form: Handle) -> i32 {
    if let Some(f) = FORMS.get(form) {
        if let Ok(guard) = f.lock() {
            return guard.len() as i32;
        }
    }
    0
}

/// Get form field by name
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lookup_field(
    _ctx: Handle,
    form: Handle,
    name: *const std::ffi::c_char,
) -> Handle {
    if name.is_null() {
        return 0;
    }

    if let Some(f) = FORMS.get(form) {
        if let Ok(guard) = f.lock() {
            if let Some(field_name) = super::safe_helpers::c_str_to_str(name) {
                if let Some(field) = guard.get_field(field_name) {
                    return FORM_FIELDS.insert(field.clone());
                }
            }
        }
    }

    0
}

/// Reset form to default values
#[unsafe(no_mangle)]
pub extern "C" fn pdf_reset_form(_ctx: Handle, form: Handle) {
    if let Some(f) = FORMS.get(form) {
        if let Ok(mut guard) = f.lock() {
            guard.reset();
        }
    }
}

/// Validate form
#[unsafe(no_mangle)]
pub extern "C" fn pdf_validate_form(_ctx: Handle, form: Handle) -> i32 {
    if let Some(f) = FORMS.get(form) {
        if let Ok(guard) = f.lock() {
            return guard.validate().is_ok() as i32;
        }
    }
    0
}

/// Delete a form field
#[unsafe(no_mangle)]
pub extern "C" fn pdf_delete_field(_ctx: Handle, _form: Handle, field: Handle) -> i32 {
    FORM_FIELDS.remove(field);
    1
}

/// Get field default value
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_default_value(
    _ctx: Handle,
    field: Handle,
    buf: *mut c_char,
    buf_size: i32,
) -> i32 {
    if buf.is_null() || buf_size <= 0 {
        return 0;
    }

    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            let default_val = &guard.default_value;
            let bytes = default_val.as_bytes();
            let len = (bytes.len() as i32).min(buf_size - 1);

            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, len as usize);
                *buf.offset(len as isize) = 0;
            }
            return len;
        }
    }
    0
}

/// Set field default value
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_field_default_value(
    _ctx: Handle,
    field: Handle,
    value: *const c_char,
) -> i32 {
    if value.is_null() {
        return 0;
    }

    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(mut guard) = f.lock() {
            if let Ok(val_str) = unsafe { CStr::from_ptr(value).to_str() } {
                guard.default_value = val_str.to_string();
                return 1;
            }
        }
    }
    0
}

/// Get field border width
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_border_width(_ctx: Handle, field: Handle) -> f32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return guard.border_width;
        }
    }
    1.0
}

/// Set field border width
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_field_border_width(_ctx: Handle, field: Handle, width: f32) {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(mut guard) = f.lock() {
            guard.border_width = width.max(0.0);
        }
    }
}

/// Get field border color (RGB)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_border_color(_ctx: Handle, field: Handle, color: *mut f32) {
    if color.is_null() {
        return;
    }

    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            unsafe {
                *color.offset(0) = guard.border_color[0];
                *color.offset(1) = guard.border_color[1];
                *color.offset(2) = guard.border_color[2];
            }
        }
    }
}

/// Set field border color (RGB)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_field_border_color(_ctx: Handle, field: Handle, color: *const f32) {
    if color.is_null() {
        return;
    }

    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(mut guard) = f.lock() {
            unsafe {
                guard.border_color = [*color.offset(0), *color.offset(1), *color.offset(2)];
            }
        }
    }
}

/// Get field background color (RGB)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_bg_color(_ctx: Handle, field: Handle, color: *mut f32) {
    if color.is_null() {
        return;
    }

    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            unsafe {
                *color.offset(0) = guard.bg_color[0];
                *color.offset(1) = guard.bg_color[1];
                *color.offset(2) = guard.bg_color[2];
            }
        }
    }
}

/// Set field background color (RGB)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_field_bg_color(_ctx: Handle, field: Handle, color: *const f32) {
    if color.is_null() {
        return;
    }

    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(mut guard) = f.lock() {
            unsafe {
                guard.bg_color = [*color.offset(0), *color.offset(1), *color.offset(2)];
            }
        }
    }
}

/// Get field font size
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_font_size(_ctx: Handle, field: Handle) -> f32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return guard.font_size;
        }
    }
    12.0
}

/// Set field font size
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_field_font_size(_ctx: Handle, field: Handle, size: f32) {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(mut guard) = f.lock() {
            guard.font_size = size.max(1.0);
        }
    }
}

/// Get field alignment (0=left, 1=center, 2=right)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_alignment(_ctx: Handle, field: Handle) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return guard.alignment;
        }
    }
    0
}

/// Set field alignment (0=left, 1=center, 2=right)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_field_alignment(_ctx: Handle, field: Handle, align: i32) {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(mut guard) = f.lock() {
            guard.alignment = align.clamp(0, 2);
        }
    }
}

/// Check if field is combo (vs list)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_is_combo(_ctx: Handle, field: Handle) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return guard.is_combo as i32;
        }
    }
    0
}

/// Check if choice allows edit
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_is_edit(_ctx: Handle, field: Handle) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return guard.editable as i32;
        }
    }
    0
}

/// Check if choice allows multiple selection
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_is_multiselect(_ctx: Handle, field: Handle) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return guard.multi_select as i32;
        }
    }
    0
}

/// Get selected choice index
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_selected_index(_ctx: Handle, field: Handle) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            return guard.selected_index;
        }
    }
    -1
}

/// Set selected choice index
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_field_selected_index(_ctx: Handle, field: Handle, idx: i32) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(mut guard) = f.lock() {
            if idx >= 0 && (idx as usize) < guard.choices.len() {
                guard.selected_index = idx;
                return 1;
            }
        }
    }
    0
}

/// Clear all choice selections
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_clear_selection(_ctx: Handle, field: Handle) {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(mut guard) = f.lock() {
            guard.selected_index = -1;
        }
    }
}

/// Remove a choice option by index
#[unsafe(no_mangle)]
pub extern "C" fn pdf_remove_field_choice(_ctx: Handle, field: Handle, idx: i32) -> i32 {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(mut guard) = f.lock() {
            if idx >= 0 && (idx as usize) < guard.choices.len() {
                guard.choices.remove(idx as usize);
                return 1;
            }
        }
    }
    0
}

/// Check if field is valid
#[unsafe(no_mangle)]
pub extern "C" fn pdf_field_is_valid(_ctx: Handle, field: Handle) -> i32 {
    if FORM_FIELDS.get(field).is_some() {
        1
    } else {
        0
    }
}

/// Clone a field (create a copy with new handle)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_clone_field(_ctx: Handle, field: Handle) -> Handle {
    if let Some(f) = FORM_FIELDS.get(field) {
        if let Ok(guard) = f.lock() {
            let cloned = guard.clone();
            return FORM_FIELDS.insert(cloned);
        }
    }
    0
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_create_text_field() {
        let name = CString::new("username").unwrap();
        let field = pdf_create_text_field(0, 0, name.as_ptr(), 10.0, 10.0, 200.0, 30.0, 50);
        assert_ne!(field, 0);

        let field_type = pdf_field_type(0, field);
        assert_eq!(field_type, 6); // Text field

        FORM_FIELDS.remove(field);
    }

    #[test]
    fn test_create_checkbox() {
        let name = CString::new("agree").unwrap();
        let field = pdf_create_checkbox(0, 0, name.as_ptr(), 10.0, 10.0, 20.0, 20.0, 1);
        assert_ne!(field, 0);

        let checked = pdf_field_is_checked(0, field);
        assert_eq!(checked, 1);

        FORM_FIELDS.remove(field);
    }

    #[test]
    fn test_field_value() {
        let name = CString::new("testfield").unwrap();
        let field = pdf_create_text_field(0, 0, name.as_ptr(), 0.0, 0.0, 100.0, 30.0, 0);

        let value = CString::new("Test Value").unwrap();
        let result = pdf_set_field_value(0, field, value.as_ptr());
        assert_eq!(result, 1);

        let mut buf = [0i8; 256];
        let len = pdf_field_value(0, field, buf.as_mut_ptr(), 256);
        assert!(len > 0);

        FORM_FIELDS.remove(field);
    }

    #[test]
    fn test_field_flags() {
        let name = CString::new("readonly").unwrap();
        let field = pdf_create_text_field(0, 0, name.as_ptr(), 0.0, 0.0, 100.0, 30.0, 0);

        pdf_set_field_flags(0, field, 1); // READ_ONLY flag
        let is_readonly = pdf_field_is_read_only(0, field);
        assert_eq!(is_readonly, 1);

        FORM_FIELDS.remove(field);
    }

    #[test]
    fn test_choice_field() {
        let name = CString::new("country").unwrap();
        let field = pdf_create_combo_box(0, 0, name.as_ptr(), 0.0, 0.0, 150.0, 30.0);

        let label1 = CString::new("United States").unwrap();
        let value1 = CString::new("US").unwrap();
        pdf_add_field_choice(0, field, label1.as_ptr(), value1.as_ptr());

        let label2 = CString::new("Canada").unwrap();
        let value2 = CString::new("CA").unwrap();
        pdf_add_field_choice(0, field, label2.as_ptr(), value2.as_ptr());

        let count = pdf_field_choice_count(0, field);
        assert_eq!(count, 2);

        FORM_FIELDS.remove(field);
    }

    #[test]
    fn test_form_operations() {
        let form = pdf_form(0, 0);
        assert_ne!(form, 0);

        let count = pdf_form_field_count(0, form);
        assert_eq!(count, 0); // Empty form

        pdf_drop_form(0, form);
    }
}
