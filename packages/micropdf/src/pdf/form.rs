//! PDF interactive forms (AcroForms)
//!
//! Provides support for PDF form fields including text, buttons, choices, and signatures.

use crate::fitz::error::{Error, Result};
use crate::fitz::geometry::Rect;
use crate::pdf::annot::Annotation;
use std::collections::HashMap;

/// Widget/Field type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetType {
    Unknown,
    /// Push button
    Button,
    /// Checkbox
    Checkbox,
    /// Combo box (dropdown)
    ComboBox,
    /// List box
    ListBox,
    /// Radio button
    RadioButton,
    /// Digital signature
    Signature,
    /// Text field
    Text,
}

impl WidgetType {
    pub fn from_string(s: &str) -> Self {
        match s {
            "Btn" => Self::Button,
            "Tx" => Self::Text,
            "Ch" => Self::ComboBox, // Or ListBox, determined by flags
            "Sig" => Self::Signature,
            _ => Self::Unknown,
        }
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Button => "Btn",
            Self::Checkbox => "Btn",
            Self::ComboBox => "Ch",
            Self::ListBox => "Ch",
            Self::RadioButton => "Btn",
            Self::Signature => "Sig",
            Self::Text => "Tx",
        }
    }
}

/// Text widget format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFormat {
    None,
    Number,
    Special,
    Date,
    Time,
}

/// Field flags (bitfield)
#[derive(Debug, Clone, Copy, Default)]
pub struct FieldFlags(u32);

impl FieldFlags {
    // All fields
    pub const READ_ONLY: u32 = 1 << 0;
    pub const REQUIRED: u32 = 1 << 1;
    pub const NO_EXPORT: u32 = 1 << 2;

    // Text fields
    pub const MULTILINE: u32 = 1 << 12;
    pub const PASSWORD: u32 = 1 << 13;
    pub const FILE_SELECT: u32 = 1 << 20;
    pub const DO_NOT_SPELL_CHECK: u32 = 1 << 22;
    pub const DO_NOT_SCROLL: u32 = 1 << 23;
    pub const COMB: u32 = 1 << 24;
    pub const RICH_TEXT: u32 = 1 << 25;

    // Button fields
    pub const NO_TOGGLE_TO_OFF: u32 = 1 << 14;
    pub const RADIO: u32 = 1 << 15;
    pub const PUSHBUTTON: u32 = 1 << 16;
    pub const RADIOS_IN_UNISON: u32 = 1 << 25;

    // Choice fields
    pub const COMBO: u32 = 1 << 17;
    pub const EDIT: u32 = 1 << 18;
    pub const SORT: u32 = 1 << 19;
    pub const MULTI_SELECT: u32 = 1 << 21;
    pub const COMMIT_ON_SEL_CHANGE: u32 = 1 << 26;

    pub fn new(flags: u32) -> Self {
        Self(flags)
    }

    pub fn has(&self, flag: u32) -> bool {
        (self.0 & flag) != 0
    }

    pub fn set(&mut self, flag: u32) {
        self.0 |= flag;
    }

    pub fn clear(&mut self, flag: u32) {
        self.0 &= !flag;
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

/// Choice option (for combo boxes and list boxes)
#[derive(Debug, Clone)]
pub struct ChoiceOption {
    /// Display value
    pub label: String,
    /// Export value (value submitted)
    pub value: String,
}

impl ChoiceOption {
    pub fn new(label: String, value: String) -> Self {
        Self { label, value }
    }

    pub fn simple(value: String) -> Self {
        Self {
            value: value.clone(),
            label: value,
        }
    }
}

/// PDF form field
#[derive(Clone)]
pub struct FormField {
    /// Field name (fully qualified)
    pub name: String,
    /// Field type
    pub field_type: WidgetType,
    /// Field flags
    pub flags: FieldFlags,
    /// Field value
    pub value: String,
    /// Default value
    pub default_value: String,
    /// Field rectangle
    pub rect: Rect,
    /// Maximum text length (for text fields)
    pub max_len: Option<usize>,
    /// Text format (for text fields)
    pub text_format: TextFormat,
    /// Choice options (for combo/list boxes)
    pub options: Vec<ChoiceOption>,
    /// Selected options indices (for multi-select)
    pub selected: Vec<usize>,
    /// Tooltip/alternate description
    pub tooltip: Option<String>,
    /// Widget annotation
    pub widget: Option<Annotation>,
    /// Custom properties
    pub properties: HashMap<String, String>,
    /// Border width
    pub border_width: f32,
    /// Border color (RGB)
    pub border_color: [f32; 3],
    /// Background color (RGB)
    pub bg_color: [f32; 3],
    /// Font size
    pub font_size: f32,
    /// Text alignment (0=left, 1=center, 2=right)
    pub alignment: i32,
    /// Is combo box (vs list box)
    pub is_combo: bool,
    /// Is editable (for choice fields)
    pub editable: bool,
    /// Allows multiple selection (for choice fields)
    pub multi_select: bool,
    /// Selected index (for single-select choice fields)
    pub selected_index: i32,
    /// Choice options (simpler representation for FFI)
    pub choices: Vec<(String, String)>,
}

impl FormField {
    /// Create a new form field
    pub fn new(name: String, field_type: WidgetType, rect: Rect) -> Self {
        Self {
            name,
            field_type,
            flags: FieldFlags::default(),
            value: String::new(),
            default_value: String::new(),
            rect,
            max_len: None,
            text_format: TextFormat::None,
            options: Vec::new(),
            selected: Vec::new(),
            tooltip: None,
            widget: None,
            properties: HashMap::new(),
            border_width: 1.0,
            border_color: [0.0, 0.0, 0.0],
            bg_color: [1.0, 1.0, 1.0],
            font_size: 12.0,
            alignment: 0,
            is_combo: false,
            editable: false,
            multi_select: false,
            selected_index: -1,
            choices: Vec::new(),
        }
    }

    /// Create a text field
    pub fn text_field(name: String, rect: Rect, max_len: Option<usize>) -> Self {
        let mut field = Self::new(name, WidgetType::Text, rect);
        field.max_len = max_len;
        field
    }

    /// Create a checkbox
    pub fn checkbox(name: String, rect: Rect, checked: bool) -> Self {
        let mut field = Self::new(name, WidgetType::Checkbox, rect);
        field.value = if checked {
            "Yes".to_string()
        } else {
            "Off".to_string()
        };
        field
    }

    /// Create a radio button
    pub fn radio_button(name: String, rect: Rect, _group: &str, value: &str) -> Self {
        let mut field = Self::new(name, WidgetType::RadioButton, rect);
        field.value = value.to_string();
        field.flags.set(FieldFlags::RADIO);
        field
    }

    /// Create a combo box
    pub fn combo_box(name: String, rect: Rect, options: Vec<ChoiceOption>) -> Self {
        let mut field = Self::new(name, WidgetType::ComboBox, rect);
        field.options = options;
        field.flags.set(FieldFlags::COMBO);
        field
    }

    /// Create a list box
    pub fn list_box(name: String, rect: Rect, options: Vec<ChoiceOption>) -> Self {
        let mut field = Self::new(name, WidgetType::ListBox, rect);
        field.options = options;
        field
    }

    /// Create a push button
    pub fn push_button(name: String, rect: Rect, caption: &str) -> Self {
        let mut field = Self::new(name, WidgetType::Button, rect);
        field.value = caption.to_string();
        field.flags.set(FieldFlags::PUSHBUTTON);
        field
    }

    /// Create a signature field
    pub fn signature(name: String, rect: Rect) -> Self {
        Self::new(name, WidgetType::Signature, rect)
    }

    /// Get field name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get field type
    pub fn field_type(&self) -> WidgetType {
        self.field_type
    }

    /// Get field flags
    pub fn flags(&self) -> FieldFlags {
        self.flags
    }

    /// Set field flags
    pub fn set_flags(&mut self, flags: FieldFlags) {
        self.flags = flags;
    }

    /// Check if field is read-only
    pub fn is_read_only(&self) -> bool {
        self.flags.has(FieldFlags::READ_ONLY)
    }

    /// Check if field is required
    pub fn is_required(&self) -> bool {
        self.flags.has(FieldFlags::REQUIRED)
    }

    /// Get field value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Set field value
    pub fn set_value(&mut self, value: String) -> Result<()> {
        // Validate based on field type
        match self.field_type {
            WidgetType::Text => {
                if let Some(max_len) = self.max_len {
                    if value.len() > max_len {
                        return Err(Error::Argument(format!(
                            "Text exceeds maximum length of {}",
                            max_len
                        )));
                    }
                }
                self.value = value;
            }
            WidgetType::Checkbox | WidgetType::RadioButton => {
                if value != "Yes" && value != "Off" {
                    return Err(Error::Argument(
                        "Checkbox/radio button value must be 'Yes' or 'Off'".into(),
                    ));
                }
                self.value = value;
            }
            WidgetType::ComboBox | WidgetType::ListBox => {
                // Validate against options
                if !self.options.iter().any(|opt| opt.value == value)
                    && !self.flags.has(FieldFlags::EDIT)
                {
                    return Err(Error::Argument("Invalid choice value".into()));
                }
                self.value = value;
            }
            _ => {
                self.value = value;
            }
        }
        Ok(())
    }

    /// Get rectangle
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Set rectangle
    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    /// Get maximum text length
    pub fn max_len(&self) -> Option<usize> {
        self.max_len
    }

    /// Set maximum text length
    pub fn set_max_len(&mut self, max_len: Option<usize>) {
        self.max_len = max_len;
    }

    /// Get text format
    pub fn text_format(&self) -> TextFormat {
        self.text_format
    }

    /// Set text format
    pub fn set_text_format(&mut self, format: TextFormat) {
        self.text_format = format;
    }

    /// Get choice options
    pub fn options(&self) -> &[ChoiceOption] {
        &self.options
    }

    /// Set choice options
    pub fn set_options(&mut self, options: Vec<ChoiceOption>) {
        self.options = options;
    }

    /// Get selected options (for multi-select)
    pub fn selected(&self) -> &[usize] {
        &self.selected
    }

    /// Set selected options
    pub fn set_selected(&mut self, selected: Vec<usize>) -> Result<()> {
        // Validate indices
        for &idx in &selected {
            if idx >= self.options.len() {
                return Err(Error::Argument(format!("Invalid option index: {}", idx)));
            }
        }

        // Check multi-select flag
        if selected.len() > 1 && !self.flags.has(FieldFlags::MULTI_SELECT) {
            return Err(Error::Argument(
                "Field does not allow multiple selections".into(),
            ));
        }

        self.selected = selected;
        Ok(())
    }

    /// Get default value
    pub fn default_value(&self) -> &str {
        &self.default_value
    }

    /// Set default value
    pub fn set_default_value(&mut self, value: String) {
        self.default_value = value;
    }

    /// Reset to default value
    pub fn reset(&mut self) {
        if !self.default_value.is_empty() {
            self.value = self.default_value.clone();
        } else {
            self.value.clear();
            self.selected.clear();
        }
    }

    /// Get tooltip
    pub fn tooltip(&self) -> Option<&str> {
        self.tooltip.as_deref()
    }

    /// Set tooltip
    pub fn set_tooltip(&mut self, tooltip: Option<String>) {
        self.tooltip = tooltip;
    }

    /// Get widget annotation
    pub fn widget(&self) -> Option<&Annotation> {
        self.widget.as_ref()
    }

    /// Set widget annotation
    pub fn set_widget(&mut self, widget: Option<Annotation>) {
        self.widget = widget;
    }

    /// Get property
    pub fn get_property(&self, key: &str) -> Option<&str> {
        self.properties.get(key).map(|s| s.as_str())
    }

    /// Set property
    pub fn set_property(&mut self, key: String, value: String) {
        self.properties.insert(key, value);
    }

    /// Check if checkbox/radio is checked
    pub fn is_checked(&self) -> bool {
        matches!(
            self.field_type,
            WidgetType::Checkbox | WidgetType::RadioButton
        ) && self.value == "Yes"
    }

    /// Set checkbox/radio checked state
    pub fn set_checked(&mut self, checked: bool) -> Result<()> {
        if !matches!(
            self.field_type,
            WidgetType::Checkbox | WidgetType::RadioButton
        ) {
            return Err(Error::Argument(
                "Field is not a checkbox or radio button".into(),
            ));
        }
        self.value = if checked {
            "Yes".to_string()
        } else {
            "Off".to_string()
        };
        Ok(())
    }

    /// Check if field is a multiline text field
    pub fn is_multiline(&self) -> bool {
        self.field_type == WidgetType::Text && self.flags.has(FieldFlags::MULTILINE)
    }

    /// Check if field is a password field
    pub fn is_password(&self) -> bool {
        self.field_type == WidgetType::Text && self.flags.has(FieldFlags::PASSWORD)
    }

    /// Check if field is signed
    pub fn is_signed(&self) -> bool {
        self.field_type == WidgetType::Signature && !self.value.is_empty()
    }
}

impl std::fmt::Debug for FormField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormField")
            .field("name", &self.name)
            .field("type", &self.field_type)
            .field("value", &self.value)
            .field("rect", &self.rect)
            .finish()
    }
}

/// PDF form (AcroForm)
#[derive(Clone, Default)]
pub struct Form {
    /// Form fields by name
    fields: HashMap<String, FormField>,
    /// Field order (for tab order)
    field_order: Vec<String>,
}

impl Form {
    /// Create a new empty form
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            field_order: Vec::new(),
        }
    }

    /// Add a field to the form
    pub fn add_field(&mut self, field: FormField) {
        let name = field.name().to_string();
        if !self.fields.contains_key(&name) {
            self.field_order.push(name.clone());
        }
        self.fields.insert(name, field);
    }

    /// Get a field by name
    pub fn get_field(&self, name: &str) -> Option<&FormField> {
        self.fields.get(name)
    }

    /// Get a mutable field by name
    pub fn get_field_mut(&mut self, name: &str) -> Option<&mut FormField> {
        self.fields.get_mut(name)
    }

    /// Remove a field
    pub fn remove_field(&mut self, name: &str) -> Option<FormField> {
        if let Some(field) = self.fields.remove(name) {
            self.field_order.retain(|n| n != name);
            Some(field)
        } else {
            None
        }
    }

    /// Get all fields
    pub fn fields(&self) -> impl Iterator<Item = &FormField> {
        self.field_order
            .iter()
            .filter_map(|name| self.fields.get(name))
    }

    /// Get all field names
    pub fn field_names(&self) -> impl Iterator<Item = &str> {
        self.field_order.iter().map(|s| s.as_str())
    }

    /// Get number of fields
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Check if form is empty
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Set field value by name
    pub fn set_field_value(&mut self, name: &str, value: String) -> Result<()> {
        let field = self
            .get_field_mut(name)
            .ok_or_else(|| Error::Argument(format!("Field not found: {}", name)))?;
        field.set_value(value)
    }

    /// Get field value by name
    pub fn get_field_value(&self, name: &str) -> Option<&str> {
        self.get_field(name).map(|f| f.value())
    }

    /// Reset all fields to default values
    pub fn reset(&mut self) {
        for field in self.fields.values_mut() {
            field.reset();
        }
    }

    /// Reset specific fields
    pub fn reset_fields(&mut self, names: &[&str]) {
        for name in names {
            if let Some(field) = self.get_field_mut(name) {
                field.reset();
            }
        }
    }

    /// Get fields by type
    pub fn fields_by_type(&self, field_type: WidgetType) -> impl Iterator<Item = &FormField> {
        self.fields().filter(move |f| f.field_type() == field_type)
    }

    /// Validate all required fields
    pub fn validate(&self) -> Result<()> {
        for field in self.fields() {
            if field.is_required() && field.value().is_empty() {
                return Err(Error::Argument(format!(
                    "Required field '{}' is empty",
                    field.name()
                )));
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Form")
            .field("field_count", &self.fields.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_type_from_string() {
        assert_eq!(WidgetType::from_string("Btn"), WidgetType::Button);
        assert_eq!(WidgetType::from_string("Tx"), WidgetType::Text);
        assert_eq!(WidgetType::from_string("Ch"), WidgetType::ComboBox);
        assert_eq!(WidgetType::from_string("Sig"), WidgetType::Signature);
    }

    #[test]
    fn test_field_flags() {
        let mut flags = FieldFlags::default();
        assert!(!flags.has(FieldFlags::READ_ONLY));

        flags.set(FieldFlags::READ_ONLY);
        assert!(flags.has(FieldFlags::READ_ONLY));

        flags.clear(FieldFlags::READ_ONLY);
        assert!(!flags.has(FieldFlags::READ_ONLY));
    }

    #[test]
    fn test_text_field() {
        let rect = Rect::new(10.0, 10.0, 200.0, 30.0);
        let field = FormField::text_field("name".to_string(), rect, Some(50));

        assert_eq!(field.name(), "name");
        assert_eq!(field.field_type(), WidgetType::Text);
        assert_eq!(field.max_len(), Some(50));
    }

    #[test]
    fn test_text_field_max_length() {
        let mut field = FormField::text_field("test".to_string(), Rect::EMPTY, Some(5));

        assert!(field.set_value("short".to_string()).is_ok());
        assert!(field.set_value("toolong".to_string()).is_err());
    }

    #[test]
    fn test_checkbox() {
        let mut field = FormField::checkbox("agree".to_string(), Rect::EMPTY, true);

        assert_eq!(field.field_type(), WidgetType::Checkbox);
        assert!(field.is_checked());

        field.set_checked(false).unwrap();
        assert!(!field.is_checked());
        assert_eq!(field.value(), "Off");
    }

    #[test]
    fn test_combo_box() {
        let options = vec![
            ChoiceOption::simple("Option 1".to_string()),
            ChoiceOption::simple("Option 2".to_string()),
            ChoiceOption::simple("Option 3".to_string()),
        ];

        let field = FormField::combo_box("choice".to_string(), Rect::EMPTY, options);

        assert_eq!(field.field_type(), WidgetType::ComboBox);
        assert_eq!(field.options().len(), 3);
        assert!(field.flags().has(FieldFlags::COMBO));
    }

    #[test]
    fn test_field_value_validation() {
        let options = vec![
            ChoiceOption::simple("A".to_string()),
            ChoiceOption::simple("B".to_string()),
        ];

        let mut field = FormField::combo_box("test".to_string(), Rect::EMPTY, options);

        assert!(field.set_value("A".to_string()).is_ok());
        assert!(field.set_value("Invalid".to_string()).is_err());
    }

    #[test]
    fn test_form_add_field() {
        let mut form = Form::new();
        let field = FormField::text_field("name".to_string(), Rect::EMPTY, None);

        form.add_field(field);

        assert_eq!(form.len(), 1);
        assert!(form.get_field("name").is_some());
    }

    #[test]
    fn test_form_set_get_value() {
        let mut form = Form::new();
        let field = FormField::text_field("email".to_string(), Rect::EMPTY, None);
        form.add_field(field);

        form.set_field_value("email", "test@example.com".to_string())
            .unwrap();
        assert_eq!(form.get_field_value("email"), Some("test@example.com"));
    }

    #[test]
    fn test_form_remove_field() {
        let mut form = Form::new();
        let field = FormField::text_field("temp".to_string(), Rect::EMPTY, None);
        form.add_field(field);

        assert_eq!(form.len(), 1);

        form.remove_field("temp");
        assert_eq!(form.len(), 0);
        assert!(form.get_field("temp").is_none());
    }

    #[test]
    fn test_form_reset() {
        let mut form = Form::new();
        let mut field = FormField::text_field("name".to_string(), Rect::EMPTY, None);
        field.set_default_value("Default".to_string());
        field.set_value("Changed".to_string()).unwrap();
        form.add_field(field);

        assert_eq!(form.get_field_value("name"), Some("Changed"));

        form.reset();
        assert_eq!(form.get_field_value("name"), Some("Default"));
    }

    #[test]
    fn test_form_validate() {
        let mut form = Form::new();

        let mut field = FormField::text_field("required".to_string(), Rect::EMPTY, None);
        let mut flags = FieldFlags::default();
        flags.set(FieldFlags::REQUIRED);
        field.set_flags(flags);

        form.add_field(field);

        // Should fail validation (required field is empty)
        assert!(form.validate().is_err());

        // Set value and validate again
        form.set_field_value("required", "value".to_string())
            .unwrap();
        assert!(form.validate().is_ok());
    }

    #[test]
    fn test_choice_option() {
        let opt = ChoiceOption::new("Display".to_string(), "export".to_string());
        assert_eq!(opt.label, "Display");
        assert_eq!(opt.value, "export");

        let simple = ChoiceOption::simple("Same".to_string());
        assert_eq!(simple.label, "Same");
        assert_eq!(simple.value, "Same");
    }

    #[test]
    fn test_field_properties() {
        let mut field = FormField::text_field("test".to_string(), Rect::EMPTY, None);

        field.set_property("custom".to_string(), "value".to_string());
        assert_eq!(field.get_property("custom"), Some("value"));
    }

    #[test]
    fn test_form_fields_by_type() {
        let mut form = Form::new();

        form.add_field(FormField::text_field(
            "text1".to_string(),
            Rect::EMPTY,
            None,
        ));
        form.add_field(FormField::text_field(
            "text2".to_string(),
            Rect::EMPTY,
            None,
        ));
        form.add_field(FormField::checkbox(
            "check1".to_string(),
            Rect::EMPTY,
            false,
        ));

        let text_fields: Vec<_> = form.fields_by_type(WidgetType::Text).collect();
        assert_eq!(text_fields.len(), 2);
    }

    #[test]
    fn test_multiselect() {
        let options = vec![
            ChoiceOption::simple("A".to_string()),
            ChoiceOption::simple("B".to_string()),
            ChoiceOption::simple("C".to_string()),
        ];

        let mut field = FormField::list_box("multi".to_string(), Rect::EMPTY, options);

        // Should fail without multi-select flag
        assert!(field.set_selected(vec![0, 1]).is_err());

        // Enable multi-select
        let mut flags = FieldFlags::default();
        flags.set(FieldFlags::MULTI_SELECT);
        field.set_flags(flags);

        // Now should work
        assert!(field.set_selected(vec![0, 2]).is_ok());
        assert_eq!(field.selected(), &[0, 2]);
    }
}
