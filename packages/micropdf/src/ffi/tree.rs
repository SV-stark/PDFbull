//! C FFI for structured content tree - MuPDF compatible
//! Safe Rust implementation of fz_tree (for tagged PDF support)

use super::{Handle, HandleStore};
use std::ffi::{CStr, c_char};
use std::sync::LazyLock;

/// Structure element type (PDF spec)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureType {
    /// Unknown/generic
    Unknown = 0,
    /// Document root
    Document = 1,
    /// Part (grouping)
    Part = 2,
    /// Article
    Art = 3,
    /// Section
    Sect = 4,
    /// Division
    Div = 5,
    /// Block quote
    BlockQuote = 6,
    /// Caption
    Caption = 7,
    /// Table of contents
    TOC = 8,
    /// TOC item
    TOCI = 9,
    /// Index
    Index = 10,
    /// Non-structural (private)
    NonStruct = 11,
    /// Private element
    Private = 12,
    // Paragraph-level
    /// Paragraph
    P = 20,
    /// Heading (generic)
    H = 21,
    /// Heading level 1
    H1 = 22,
    /// Heading level 2
    H2 = 23,
    /// Heading level 3
    H3 = 24,
    /// Heading level 4
    H4 = 25,
    /// Heading level 5
    H5 = 26,
    /// Heading level 6
    H6 = 27,
    // List elements
    /// List
    L = 30,
    /// List item
    LI = 31,
    /// Label (list bullet/number)
    Lbl = 32,
    /// List body
    LBody = 33,
    // Table elements
    /// Table
    Table = 40,
    /// Table row
    TR = 41,
    /// Table header cell
    TH = 42,
    /// Table data cell
    TD = 43,
    /// Table header group
    THead = 44,
    /// Table body group
    TBody = 45,
    /// Table footer group
    TFoot = 46,
    // Inline elements
    /// Span
    Span = 50,
    /// Quote (inline)
    Quote = 51,
    /// Note
    Note = 52,
    /// Reference
    Reference = 53,
    /// Bibliography entry
    BibEntry = 54,
    /// Code
    Code = 55,
    /// Link
    Link = 56,
    /// Annotation
    Annot = 57,
    // Special elements
    /// Figure
    Figure = 60,
    /// Formula
    Formula = 61,
    /// Form field
    Form = 62,
}

/// Reading order for tree traversal
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingOrder {
    /// Natural document order
    Column = 0,
    /// Row-based reading (tables)
    Row = 1,
    /// Unordered
    Unordered = 2,
}

/// Tree node structure
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// Structure type
    pub struct_type: StructureType,
    /// Type name (string form)
    pub type_name: String,
    /// Title/alt text
    pub title: String,
    /// Alternative text (for images)
    pub alt_text: String,
    /// Actual text (replacement)
    pub actual_text: String,
    /// Language (BCP 47)
    pub lang: String,
    /// Expansion (for abbreviations)
    pub expansion: String,
    /// ID attribute
    pub id: String,
    /// Child nodes
    pub children: Vec<Handle>,
    /// Parent node
    pub parent: Handle,
    /// Page number (if applicable)
    pub page: i32,
    /// Content bounds [x0, y0, x1, y1]
    pub bbox: [f32; 4],
    /// Associated marked content ID
    pub mcid: i32,
    /// Reading order hint
    pub reading_order: ReadingOrder,
    /// Custom attributes
    pub attributes: std::collections::HashMap<String, String>,
}

impl Default for TreeNode {
    fn default() -> Self {
        Self {
            struct_type: StructureType::Unknown,
            type_name: String::new(),
            title: String::new(),
            alt_text: String::new(),
            actual_text: String::new(),
            lang: String::new(),
            expansion: String::new(),
            id: String::new(),
            children: Vec::new(),
            parent: 0,
            page: -1,
            bbox: [0.0, 0.0, 0.0, 0.0],
            mcid: -1,
            reading_order: ReadingOrder::Column,
            attributes: std::collections::HashMap::new(),
        }
    }
}

/// Structure tree (document-level)
#[derive(Debug)]
pub struct StructureTree {
    /// Root node
    pub root: Handle,
    /// All nodes for lookup
    pub nodes: Vec<Handle>,
    /// ID to node mapping
    pub id_map: std::collections::HashMap<String, Handle>,
    /// Role map (custom type -> standard type)
    pub role_map: std::collections::HashMap<String, StructureType>,
}

impl Default for StructureTree {
    fn default() -> Self {
        Self {
            root: 0,
            nodes: Vec::new(),
            id_map: std::collections::HashMap::new(),
            role_map: std::collections::HashMap::new(),
        }
    }
}

/// Global tree node storage
pub static TREE_NODES: LazyLock<HandleStore<TreeNode>> = LazyLock::new(HandleStore::new);

/// Global structure tree storage
pub static STRUCTURE_TREES: LazyLock<HandleStore<StructureTree>> = LazyLock::new(HandleStore::new);

// ============================================================================
// Structure Tree Creation
// ============================================================================

/// Create a new structure tree
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_structure_tree(_ctx: Handle) -> Handle {
    // Create root node
    let root_node = TreeNode {
        struct_type: StructureType::Document,
        type_name: "Document".to_string(),
        ..Default::default()
    };
    let root_handle = TREE_NODES.insert(root_node);

    let tree = StructureTree {
        root: root_handle,
        nodes: vec![root_handle],
        ..Default::default()
    };

    STRUCTURE_TREES.insert(tree)
}

/// Add a new node to the tree
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_add_node(
    _ctx: Handle,
    tree: Handle,
    parent: Handle,
    struct_type: i32,
) -> Handle {
    let st = match struct_type {
        1 => StructureType::Document,
        2 => StructureType::Part,
        3 => StructureType::Art,
        4 => StructureType::Sect,
        5 => StructureType::Div,
        20 => StructureType::P,
        21 => StructureType::H,
        22..=27 => match struct_type {
            22 => StructureType::H1,
            23 => StructureType::H2,
            24 => StructureType::H3,
            25 => StructureType::H4,
            26 => StructureType::H5,
            _ => StructureType::H6,
        },
        30 => StructureType::L,
        31 => StructureType::LI,
        40 => StructureType::Table,
        41 => StructureType::TR,
        42 => StructureType::TH,
        43 => StructureType::TD,
        50 => StructureType::Span,
        56 => StructureType::Link,
        60 => StructureType::Figure,
        _ => StructureType::Unknown,
    };

    let node = TreeNode {
        struct_type: st,
        parent,
        ..Default::default()
    };

    let node_handle = TREE_NODES.insert(node);

    // Link to parent
    if let Some(p) = TREE_NODES.get(parent) {
        if let Ok(mut guard) = p.lock() {
            guard.children.push(node_handle);
        }
    }

    // Add to tree
    if let Some(t) = STRUCTURE_TREES.get(tree) {
        if let Ok(mut guard) = t.lock() {
            guard.nodes.push(node_handle);
        }
    }

    node_handle
}

// ============================================================================
// Node Properties
// ============================================================================

/// Get node structure type
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_type(_ctx: Handle, node: Handle) -> i32 {
    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            return guard.struct_type as i32;
        }
    }
    0
}

/// Get node type name
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_type_name(_ctx: Handle, node: Handle) -> *const c_char {
    static EMPTY: &[u8] = b"\0";

    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            if !guard.type_name.is_empty() {
                return guard.type_name.as_ptr().cast();
            }
        }
    }
    EMPTY.as_ptr().cast()
}

/// Set node title
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_set_title(_ctx: Handle, node: Handle, title: *const c_char) {
    if title.is_null() {
        return;
    }

    let title_str = unsafe { CStr::from_ptr(title) };
    let title_text = title_str.to_str().unwrap_or("").to_string();

    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(mut guard) = n.lock() {
            guard.title = title_text;
        }
    }
}

/// Get node title
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_title(_ctx: Handle, node: Handle) -> *const c_char {
    static EMPTY: &[u8] = b"\0";

    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            if !guard.title.is_empty() {
                return guard.title.as_ptr().cast();
            }
        }
    }
    EMPTY.as_ptr().cast()
}

/// Set alt text (for images/figures)
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_set_alt(_ctx: Handle, node: Handle, alt: *const c_char) {
    if alt.is_null() {
        return;
    }

    let alt_str = unsafe { CStr::from_ptr(alt) };
    let alt_text = alt_str.to_str().unwrap_or("").to_string();

    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(mut guard) = n.lock() {
            guard.alt_text = alt_text;
        }
    }
}

/// Get alt text
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_alt(_ctx: Handle, node: Handle) -> *const c_char {
    static EMPTY: &[u8] = b"\0";

    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            if !guard.alt_text.is_empty() {
                return guard.alt_text.as_ptr().cast();
            }
        }
    }
    EMPTY.as_ptr().cast()
}

/// Set actual text (replacement text)
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_set_actual_text(_ctx: Handle, node: Handle, text: *const c_char) {
    if text.is_null() {
        return;
    }

    let text_str = unsafe { CStr::from_ptr(text) };
    let actual = text_str.to_str().unwrap_or("").to_string();

    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(mut guard) = n.lock() {
            guard.actual_text = actual;
        }
    }
}

/// Get actual text
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_actual_text(_ctx: Handle, node: Handle) -> *const c_char {
    static EMPTY: &[u8] = b"\0";

    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            if !guard.actual_text.is_empty() {
                return guard.actual_text.as_ptr().cast();
            }
        }
    }
    EMPTY.as_ptr().cast()
}

/// Set language
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_set_lang(_ctx: Handle, node: Handle, lang: *const c_char) {
    if lang.is_null() {
        return;
    }

    let lang_str = unsafe { CStr::from_ptr(lang) };
    let language = lang_str.to_str().unwrap_or("").to_string();

    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(mut guard) = n.lock() {
            guard.lang = language;
        }
    }
}

/// Get language
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_lang(_ctx: Handle, node: Handle) -> *const c_char {
    static EMPTY: &[u8] = b"\0";

    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            if !guard.lang.is_empty() {
                return guard.lang.as_ptr().cast();
            }
        }
    }
    EMPTY.as_ptr().cast()
}

/// Set ID
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_set_id(_ctx: Handle, tree: Handle, node: Handle, id: *const c_char) {
    if id.is_null() {
        return;
    }

    let id_str = unsafe { CStr::from_ptr(id) };
    let id_text = id_str.to_str().unwrap_or("").to_string();

    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(mut guard) = n.lock() {
            guard.id = id_text.clone();
        }
    }

    // Update ID map
    if let Some(t) = STRUCTURE_TREES.get(tree) {
        if let Ok(mut guard) = t.lock() {
            guard.id_map.insert(id_text, node);
        }
    }
}

/// Set page and bbox
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_set_page(
    _ctx: Handle,
    node: Handle,
    page: i32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
) {
    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(mut guard) = n.lock() {
            guard.page = page;
            guard.bbox = [x0, y0, x1, y1];
        }
    }
}

/// Get page number
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_page(_ctx: Handle, node: Handle) -> i32 {
    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            return guard.page;
        }
    }
    -1
}

/// Get bbox
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_bbox(_ctx: Handle, node: Handle, bbox: *mut f32) {
    if bbox.is_null() {
        return;
    }

    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            let bbox_slice = unsafe { std::slice::from_raw_parts_mut(bbox, 4) };
            bbox_slice.copy_from_slice(&guard.bbox);
        }
    }
}

/// Set marked content ID
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_set_mcid(_ctx: Handle, node: Handle, mcid: i32) {
    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(mut guard) = n.lock() {
            guard.mcid = mcid;
        }
    }
}

/// Get marked content ID
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_mcid(_ctx: Handle, node: Handle) -> i32 {
    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            return guard.mcid;
        }
    }
    -1
}

// ============================================================================
// Tree Navigation
// ============================================================================

/// Get tree root
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_root(_ctx: Handle, tree: Handle) -> Handle {
    if let Some(t) = STRUCTURE_TREES.get(tree) {
        if let Ok(guard) = t.lock() {
            return guard.root;
        }
    }
    0
}

/// Get first child
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_first_child(_ctx: Handle, node: Handle) -> Handle {
    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            return guard.children.first().copied().unwrap_or(0);
        }
    }
    0
}

/// Get child count
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_child_count(_ctx: Handle, node: Handle) -> i32 {
    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            return guard.children.len() as i32;
        }
    }
    0
}

/// Get child at index
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_child(_ctx: Handle, node: Handle, index: i32) -> Handle {
    if index < 0 {
        return 0;
    }

    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            return guard.children.get(index as usize).copied().unwrap_or(0);
        }
    }
    0
}

/// Get parent
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_node_parent(_ctx: Handle, node: Handle) -> Handle {
    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            return guard.parent;
        }
    }
    0
}

/// Find node by ID
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_find_by_id(_ctx: Handle, tree: Handle, id: *const c_char) -> Handle {
    if id.is_null() {
        return 0;
    }

    let id_str = unsafe { CStr::from_ptr(id) };
    let id_text = id_str.to_str().unwrap_or("");

    if let Some(t) = STRUCTURE_TREES.get(tree) {
        if let Ok(guard) = t.lock() {
            return guard.id_map.get(id_text).copied().unwrap_or(0);
        }
    }
    0
}

// ============================================================================
// Reading Order
// ============================================================================

/// Get text in reading order
#[unsafe(no_mangle)]
pub extern "C" fn fz_tree_get_text_in_order(
    _ctx: Handle,
    node: Handle,
    buffer: *mut c_char,
    buffer_size: usize,
) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }

    let mut text = String::new();
    collect_text_recursive(node, &mut text);

    let bytes = text.as_bytes();
    let copy_len = bytes.len().min(buffer_size - 1);

    let buffer_slice = unsafe { std::slice::from_raw_parts_mut(buffer as *mut u8, copy_len + 1) };
    buffer_slice[..copy_len].copy_from_slice(&bytes[..copy_len]);
    buffer_slice[copy_len] = 0;

    copy_len
}

fn collect_text_recursive(node: Handle, text: &mut String) {
    if let Some(n) = TREE_NODES.get(node) {
        if let Ok(guard) = n.lock() {
            // Add actual text if present
            if !guard.actual_text.is_empty() {
                text.push_str(&guard.actual_text);
                text.push(' ');
            }

            // Recurse into children
            for &child in &guard.children {
                collect_text_recursive(child, text);
            }
        }
    }
}

// ============================================================================
// Reference Counting
// ============================================================================

/// Keep tree reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_structure_tree(_ctx: Handle, tree: Handle) -> Handle {
    STRUCTURE_TREES.keep(tree)
}

/// Drop structure tree
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_structure_tree(_ctx: Handle, tree: Handle) {
    if let Some(t) = STRUCTURE_TREES.get(tree) {
        if let Ok(guard) = t.lock() {
            for &node in &guard.nodes {
                TREE_NODES.remove(node);
            }
        }
    }
    STRUCTURE_TREES.remove(tree);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tree() {
        let tree = fz_new_structure_tree(0);
        assert!(tree > 0);

        let root = fz_tree_root(0, tree);
        assert!(root > 0);
        assert_eq!(fz_tree_node_type(0, root), StructureType::Document as i32);

        fz_drop_structure_tree(0, tree);
    }

    #[test]
    fn test_add_nodes() {
        let tree = fz_new_structure_tree(0);
        let root = fz_tree_root(0, tree);

        let sect = fz_tree_add_node(0, tree, root, StructureType::Sect as i32);
        let para = fz_tree_add_node(0, tree, sect, StructureType::P as i32);

        assert_eq!(fz_tree_node_child_count(0, root), 1);
        assert_eq!(fz_tree_node_child_count(0, sect), 1);
        assert_eq!(fz_tree_node_parent(0, para), sect);

        fz_drop_structure_tree(0, tree);
    }

    #[test]
    fn test_node_properties() {
        let tree = fz_new_structure_tree(0);
        let root = fz_tree_root(0, tree);
        let fig = fz_tree_add_node(0, tree, root, StructureType::Figure as i32);

        fz_tree_node_set_alt(0, fig, c"A beautiful image".as_ptr());
        fz_tree_node_set_title(0, fig, c"Figure 1".as_ptr());
        fz_tree_node_set_page(0, fig, 5, 100.0, 200.0, 400.0, 500.0);

        assert_eq!(fz_tree_node_page(0, fig), 5);

        let mut bbox = [0.0f32; 4];
        fz_tree_node_bbox(0, fig, bbox.as_mut_ptr());
        assert_eq!(bbox, [100.0, 200.0, 400.0, 500.0]);

        fz_drop_structure_tree(0, tree);
    }

    #[test]
    fn test_find_by_id() {
        let tree = fz_new_structure_tree(0);
        let root = fz_tree_root(0, tree);
        let node = fz_tree_add_node(0, tree, root, StructureType::P as i32);

        fz_tree_node_set_id(0, tree, node, c"para-1".as_ptr());

        let found = fz_tree_find_by_id(0, tree, c"para-1".as_ptr());
        assert_eq!(found, node);

        let not_found = fz_tree_find_by_id(0, tree, c"nonexistent".as_ptr());
        assert_eq!(not_found, 0);

        fz_drop_structure_tree(0, tree);
    }
}
