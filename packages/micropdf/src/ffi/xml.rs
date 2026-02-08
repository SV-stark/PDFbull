//! C FFI for XML parsing - MuPDF compatible
//! Safe Rust implementation of fz_xml

use super::{Handle, HandleStore};
use std::collections::HashMap;
use std::ffi::{CStr, c_char};
use std::sync::LazyLock;

/// XML node type
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmlNodeType {
    /// Document root
    Document = 0,
    /// Element node
    Element = 1,
    /// Text content
    Text = 2,
    /// Comment
    Comment = 3,
    /// CDATA section
    CData = 4,
    /// Processing instruction
    ProcessingInstruction = 5,
}

/// XML node structure
#[derive(Debug, Clone)]
pub struct XmlNode {
    /// Node type
    pub node_type: XmlNodeType,
    /// Tag name (for elements)
    pub name: String,
    /// Namespace URI
    pub namespace_uri: String,
    /// Namespace prefix
    pub namespace_prefix: String,
    /// Text content
    pub content: String,
    /// Attributes
    pub attributes: HashMap<String, String>,
    /// Child nodes
    pub children: Vec<Handle>,
    /// Parent node
    pub parent: Handle,
    /// Next sibling
    pub next: Handle,
    /// Previous sibling
    pub prev: Handle,
}

impl Default for XmlNode {
    fn default() -> Self {
        Self {
            node_type: XmlNodeType::Element,
            name: String::new(),
            namespace_uri: String::new(),
            namespace_prefix: String::new(),
            content: String::new(),
            attributes: HashMap::new(),
            children: Vec::new(),
            parent: 0,
            next: 0,
            prev: 0,
        }
    }
}

/// XML document structure
#[derive(Debug)]
pub struct XmlDocument {
    /// Root element
    pub root: Handle,
    /// All nodes (for handle lookup)
    pub nodes: Vec<Handle>,
    /// Namespace declarations
    pub namespaces: HashMap<String, String>,
    /// XML version
    pub version: String,
    /// Encoding
    pub encoding: String,
    /// Standalone flag
    pub standalone: bool,
}

impl Default for XmlDocument {
    fn default() -> Self {
        Self {
            root: 0,
            nodes: Vec::new(),
            namespaces: HashMap::new(),
            version: "1.0".to_string(),
            encoding: "UTF-8".to_string(),
            standalone: false,
        }
    }
}

/// Global XML node storage
pub static XML_NODES: LazyLock<HandleStore<XmlNode>> = LazyLock::new(HandleStore::new);

/// Global XML document storage
pub static XML_DOCS: LazyLock<HandleStore<XmlDocument>> = LazyLock::new(HandleStore::new);

// ============================================================================
// Document Creation and Parsing
// ============================================================================

/// Create a new empty XML document
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_xml_document(_ctx: Handle) -> Handle {
    XML_DOCS.insert(XmlDocument::default())
}

/// Parse XML from string
///
/// # Safety
/// `xml_string` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub extern "C" fn fz_parse_xml(
    _ctx: Handle,
    xml_string: *const c_char,
    _preserve_whitespace: i32,
) -> Handle {
    if xml_string.is_null() {
        return 0;
    }

    let xml_str = unsafe { CStr::from_ptr(xml_string) };
    let xml = match xml_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    // Simple XML parser
    match parse_xml_string(xml) {
        Some(doc_handle) => doc_handle,
        None => 0,
    }
}

/// Parse XML from buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_parse_xml_from_buffer(
    _ctx: Handle,
    buffer: Handle,
    _preserve_whitespace: i32,
) -> Handle {
    if let Some(buf) = super::BUFFERS.get(buffer) {
        if let Ok(guard) = buf.lock() {
            if let Ok(xml_str) = std::str::from_utf8(guard.data()) {
                return parse_xml_string(xml_str).unwrap_or(0);
            }
        }
    }
    0
}

/// Simple XML parser
fn parse_xml_string(xml: &str) -> Option<Handle> {
    let mut doc = XmlDocument::default();

    // Create root document node
    let root_node = XmlNode {
        node_type: XmlNodeType::Document,
        ..Default::default()
    };
    let root_handle = XML_NODES.insert(root_node);
    doc.root = root_handle;
    doc.nodes.push(root_handle);

    // Simple recursive descent parser
    let mut chars = xml.chars().peekable();
    let mut current_parent = root_handle;

    while chars.peek().is_some() {
        skip_whitespace(&mut chars);

        if chars.peek() == Some(&'<') {
            chars.next(); // consume '<'

            if chars.peek() == Some(&'/') {
                // Closing tag
                chars.next();
                let _tag_name = read_until(&mut chars, '>');

                // Move up to parent
                if let Some(node) = XML_NODES.get(current_parent) {
                    if let Ok(guard) = node.lock() {
                        if guard.parent != 0 {
                            current_parent = guard.parent;
                        }
                    }
                }
            } else if chars.peek() == Some(&'?') {
                // Processing instruction
                chars.next();
                let _pi = read_until(&mut chars, '>');
            } else if chars.peek() == Some(&'!') {
                // Comment or CDATA
                chars.next();
                if chars.peek() == Some(&'-') {
                    // Comment
                    read_until(&mut chars, '>');
                } else {
                    // CDATA or DOCTYPE
                    read_until(&mut chars, '>');
                }
            } else {
                // Opening tag
                let (tag_name, attributes, self_closing) = parse_start_tag(&mut chars);

                let mut new_node = XmlNode {
                    node_type: XmlNodeType::Element,
                    name: tag_name,
                    parent: current_parent,
                    ..Default::default()
                };

                // Parse attributes
                for (key, mut value) in attributes {
                    // Append null terminator for C string compatibility
                    value.push('\0');

                    if key.starts_with("xmlns") {
                        // Namespace declaration
                        if key == "xmlns" {
                            new_node.namespace_uri = value;
                        } else if let Some(prefix) = key.strip_prefix("xmlns:") {
                            doc.namespaces.insert(prefix.to_string(), value);
                        }
                    } else {
                        new_node.attributes.insert(key, value);
                    }
                }

                let new_handle = XML_NODES.insert(new_node);
                doc.nodes.push(new_handle);

                // Link to parent
                if let Some(parent_node) = XML_NODES.get(current_parent) {
                    if let Ok(mut guard) = parent_node.lock() {
                        // Update sibling links
                        if let Some(&prev_sibling) = guard.children.last() {
                            if let Some(prev) = XML_NODES.get(prev_sibling) {
                                if let Ok(mut prev_guard) = prev.lock() {
                                    prev_guard.next = new_handle;
                                }
                            }
                            if let Some(new) = XML_NODES.get(new_handle) {
                                if let Ok(mut new_guard) = new.lock() {
                                    new_guard.prev = prev_sibling;
                                }
                            }
                        }
                        guard.children.push(new_handle);
                    }
                }

                if !self_closing {
                    current_parent = new_handle;
                }
            }
        } else {
            // Text content
            let text = read_until_char(&mut chars, '<');
            let trimmed = text.trim();

            if !trimmed.is_empty() {
                let text_node = XmlNode {
                    node_type: XmlNodeType::Text,
                    content: trimmed.to_string(),
                    parent: current_parent,
                    ..Default::default()
                };
                let text_handle = XML_NODES.insert(text_node);
                doc.nodes.push(text_handle);

                if let Some(parent_node) = XML_NODES.get(current_parent) {
                    if let Ok(mut guard) = parent_node.lock() {
                        guard.children.push(text_handle);
                    }
                }
            }
        }
    }

    Some(XML_DOCS.insert(doc))
}

fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
        chars.next();
    }
}

fn read_until(chars: &mut std::iter::Peekable<std::str::Chars>, end: char) -> String {
    let mut result = String::new();
    while let Some(&c) = chars.peek() {
        chars.next();
        if c == end {
            break;
        }
        result.push(c);
    }
    result
}

fn read_until_char(chars: &mut std::iter::Peekable<std::str::Chars>, end: char) -> String {
    let mut result = String::new();
    while let Some(&c) = chars.peek() {
        if c == end {
            break;
        }
        chars.next();
        result.push(c);
    }
    result
}

fn parse_start_tag(
    chars: &mut std::iter::Peekable<std::str::Chars>,
) -> (String, Vec<(String, String)>, bool) {
    let mut tag_name = String::new();
    let mut attributes = Vec::new();
    let mut self_closing = false;

    // Read tag name
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() || c == '>' || c == '/' {
            break;
        }
        chars.next();
        tag_name.push(c);
    }

    // Read attributes
    loop {
        skip_whitespace(chars);

        match chars.peek() {
            Some(&'>') => {
                chars.next();
                break;
            }
            Some(&'/') => {
                chars.next();
                if chars.peek() == Some(&'>') {
                    chars.next();
                    self_closing = true;
                }
                break;
            }
            Some(_) => {
                // Read attribute name
                let attr_name = read_until_chars(chars, &['=', '>', '/', ' ']);
                skip_whitespace(chars);

                if chars.peek() == Some(&'=') {
                    chars.next();
                    skip_whitespace(chars);

                    // Read attribute value
                    let quote = chars.next().unwrap_or('"');
                    let attr_value = read_until(chars, quote);
                    attributes.push((attr_name, attr_value));
                }
            }
            None => break,
        }
    }

    (tag_name, attributes, self_closing)
}

fn read_until_chars(chars: &mut std::iter::Peekable<std::str::Chars>, ends: &[char]) -> String {
    let mut result = String::new();
    while let Some(&c) = chars.peek() {
        if ends.contains(&c) {
            break;
        }
        chars.next();
        result.push(c);
    }
    result
}

// ============================================================================
// Document Navigation
// ============================================================================

/// Get root element of document
#[unsafe(no_mangle)]
pub extern "C" fn fz_xml_root(_ctx: Handle, doc: Handle) -> Handle {
    if let Some(d) = XML_DOCS.get(doc) {
        if let Ok(guard) = d.lock() {
            // Return first child of document node
            if let Some(root) = XML_NODES.get(guard.root) {
                if let Ok(root_guard) = root.lock() {
                    return root_guard.children.first().copied().unwrap_or(0);
                }
            }
        }
    }
    0
}

/// Get first child element
#[unsafe(no_mangle)]
pub extern "C" fn fz_xml_down(_ctx: Handle, node: Handle) -> Handle {
    if let Some(n) = XML_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            return guard.children.first().copied().unwrap_or(0);
        }
    }
    0
}

/// Get next sibling element
#[unsafe(no_mangle)]
pub extern "C" fn fz_xml_next(_ctx: Handle, node: Handle) -> Handle {
    if let Some(n) = XML_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            return guard.next;
        }
    }
    0
}

/// Get previous sibling element
#[unsafe(no_mangle)]
pub extern "C" fn fz_xml_prev(_ctx: Handle, node: Handle) -> Handle {
    if let Some(n) = XML_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            return guard.prev;
        }
    }
    0
}

/// Get parent element
#[unsafe(no_mangle)]
pub extern "C" fn fz_xml_up(_ctx: Handle, node: Handle) -> Handle {
    if let Some(n) = XML_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            return guard.parent;
        }
    }
    0
}

// ============================================================================
// Node Properties
// ============================================================================

/// Get node tag name
#[unsafe(no_mangle)]
pub extern "C" fn fz_xml_tag(_ctx: Handle, node: Handle) -> *const c_char {
    static EMPTY: &[u8] = b"\0";

    if let Some(n) = XML_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            if !guard.name.is_empty() {
                return guard.name.as_ptr().cast();
            }
        }
    }
    EMPTY.as_ptr().cast()
}

/// Check if node has specific tag
#[unsafe(no_mangle)]
pub extern "C" fn fz_xml_is_tag(_ctx: Handle, node: Handle, tag: *const c_char) -> i32 {
    if tag.is_null() {
        return 0;
    }

    let tag_str = unsafe { CStr::from_ptr(tag) };
    let tag_name = tag_str.to_str().unwrap_or("");

    if let Some(n) = XML_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            return i32::from(guard.name == tag_name);
        }
    }
    0
}

/// Get node text content
#[unsafe(no_mangle)]
pub extern "C" fn fz_xml_text(_ctx: Handle, node: Handle) -> *const c_char {
    static EMPTY: &[u8] = b"\0";

    if let Some(n) = XML_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            if guard.node_type == XmlNodeType::Text && !guard.content.is_empty() {
                return guard.content.as_ptr().cast();
            }
        }
    }
    EMPTY.as_ptr().cast()
}

/// Get attribute value
#[unsafe(no_mangle)]
pub extern "C" fn fz_xml_att(_ctx: Handle, node: Handle, name: *const c_char) -> *const c_char {
    if name.is_null() {
        return std::ptr::null();
    }

    let name_str = unsafe { CStr::from_ptr(name) };
    let attr_name = name_str.to_str().unwrap_or("");

    if let Some(n) = XML_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            if let Some(value) = guard.attributes.get(attr_name) {
                return value.as_ptr().cast();
            }
        }
    }
    std::ptr::null()
}

/// Get attribute count
#[unsafe(no_mangle)]
pub extern "C" fn fz_xml_att_count(_ctx: Handle, node: Handle) -> i32 {
    if let Some(n) = XML_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            return guard.attributes.len() as i32;
        }
    }
    0
}

/// Get child count
#[unsafe(no_mangle)]
pub extern "C" fn fz_xml_child_count(_ctx: Handle, node: Handle) -> i32 {
    if let Some(n) = XML_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            return guard.children.len() as i32;
        }
    }
    0
}

/// Get node type
#[unsafe(no_mangle)]
pub extern "C" fn fz_xml_node_type(_ctx: Handle, node: Handle) -> i32 {
    if let Some(n) = XML_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            return guard.node_type as i32;
        }
    }
    -1
}

// ============================================================================
// XPath Queries (Simple Implementation)
// ============================================================================

/// Find element by simple path (e.g., "root/child/grandchild")
#[unsafe(no_mangle)]
pub extern "C" fn fz_xml_find(_ctx: Handle, node: Handle, path: *const c_char) -> Handle {
    if path.is_null() {
        return 0;
    }

    let path_str = unsafe { CStr::from_ptr(path) };
    let xpath = path_str.to_str().unwrap_or("");

    let mut current = node;
    for segment in xpath.split('/') {
        if segment.is_empty() {
            continue;
        }

        current = find_child_by_tag(current, segment);
        if current == 0 {
            return 0;
        }
    }

    current
}

fn find_child_by_tag(parent: Handle, tag: &str) -> Handle {
    if let Some(n) = XML_NODES.get(parent) {
        if let Ok(guard) = n.lock() {
            for &child in &guard.children {
                if let Some(child_node) = XML_NODES.get(child) {
                    if let Ok(child_guard) = child_node.lock() {
                        if child_guard.name == tag {
                            return child;
                        }
                    }
                }
            }
        }
    }
    0
}

/// Find all elements matching tag
#[unsafe(no_mangle)]
pub extern "C" fn fz_xml_find_all(
    _ctx: Handle,
    node: Handle,
    tag: *const c_char,
    results: *mut Handle,
    max_results: i32,
) -> i32 {
    if tag.is_null() || results.is_null() || max_results <= 0 {
        return 0;
    }

    let tag_str = unsafe { CStr::from_ptr(tag) };
    let tag_name = tag_str.to_str().unwrap_or("");

    let mut found = Vec::new();
    find_all_recursive(node, tag_name, &mut found, max_results as usize);

    let count = found.len().min(max_results as usize);
    let result_slice = unsafe { std::slice::from_raw_parts_mut(results, count) };
    result_slice.copy_from_slice(&found[..count]);

    count as i32
}

fn find_all_recursive(node: Handle, tag: &str, results: &mut Vec<Handle>, max: usize) {
    if results.len() >= max {
        return;
    }

    if let Some(n) = XML_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            if guard.name == tag {
                results.push(node);
            }

            for &child in &guard.children {
                find_all_recursive(child, tag, results, max);
            }
        }
    }
}

// ============================================================================
// Reference Counting
// ============================================================================

/// Drop XML document
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_xml(_ctx: Handle, doc: Handle) {
    if let Some(d) = XML_DOCS.get(doc) {
        if let Ok(guard) = d.lock() {
            // Drop all nodes
            for &node in &guard.nodes {
                XML_NODES.remove(node);
            }
        }
    }
    XML_DOCS.remove(doc);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_xml() {
        let xml = c"<root><child>text</child></root>";
        let doc = fz_parse_xml(0, xml.as_ptr(), 0);
        assert!(doc > 0);

        let root = fz_xml_root(0, doc);
        assert!(root > 0);
        assert_eq!(fz_xml_is_tag(0, root, c"root".as_ptr()), 1);

        let child = fz_xml_down(0, root);
        assert!(child > 0);
        assert_eq!(fz_xml_is_tag(0, child, c"child".as_ptr()), 1);

        fz_drop_xml(0, doc);
    }

    #[test]
    fn test_xml_attributes() {
        let xml = c"<elem attr=\"value\" num=\"42\"/>";
        let doc = fz_parse_xml(0, xml.as_ptr(), 0);
        let root = fz_xml_root(0, doc);

        assert_eq!(fz_xml_att_count(0, root), 2);

        let attr = fz_xml_att(0, root, c"attr".as_ptr());
        assert!(!attr.is_null());
        let attr_str = unsafe { CStr::from_ptr(attr) };
        assert_eq!(attr_str.to_str().unwrap(), "value");

        fz_drop_xml(0, doc);
    }

    #[test]
    fn test_xml_navigation() {
        let xml = c"<root><a/><b/><c/></root>";
        let doc = fz_parse_xml(0, xml.as_ptr(), 0);
        let root = fz_xml_root(0, doc);

        let a = fz_xml_down(0, root);
        assert_eq!(fz_xml_is_tag(0, a, c"a".as_ptr()), 1);

        let b = fz_xml_next(0, a);
        assert_eq!(fz_xml_is_tag(0, b, c"b".as_ptr()), 1);

        let c = fz_xml_next(0, b);
        assert_eq!(fz_xml_is_tag(0, c, c"c".as_ptr()), 1);

        // Navigate back
        let b_again = fz_xml_prev(0, c);
        assert_eq!(fz_xml_is_tag(0, b_again, c"b".as_ptr()), 1);

        fz_drop_xml(0, doc);
    }

    #[test]
    fn test_xml_find() {
        let xml = c"<root><level1><level2>deep</level2></level1></root>";
        let doc = fz_parse_xml(0, xml.as_ptr(), 0);
        let root = fz_xml_root(0, doc);

        let level2 = fz_xml_find(0, root, c"level1/level2".as_ptr());
        assert!(level2 > 0);
        assert_eq!(fz_xml_is_tag(0, level2, c"level2".as_ptr()), 1);

        fz_drop_xml(0, doc);
    }
}
