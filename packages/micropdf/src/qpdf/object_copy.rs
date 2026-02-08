//! Foreign object copying with dependency tracking
//!
//! This module provides functionality to copy objects between PDF files
//! while properly handling object references and dependencies.

use super::error::{QpdfError, Result};
use std::collections::{HashMap, HashSet};

/// Object identifier (object number, generation number)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjGen {
    /// Object number
    pub obj: u32,
    /// Generation number
    pub generation: u32,
}

impl ObjGen {
    /// Create a new object identifier
    pub fn new(obj: u32, generation: u32) -> Self {
        Self { obj, generation }
    }

    /// Create from object number with generation 0
    pub fn from_obj(obj: u32) -> Self {
        Self { obj, generation: 0 }
    }
}

impl std::fmt::Display for ObjGen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} R", self.obj, self.generation)
    }
}

/// Mapping from source object to destination object
pub type ObjectMap = HashMap<ObjGen, ObjGen>;

/// Copy context for tracking copied objects
#[derive(Debug)]
pub struct CopyContext {
    /// Mapping from source objects to destination objects
    pub object_map: ObjectMap,
    /// Objects that need to be copied
    pending: HashSet<ObjGen>,
    /// Next available object number in destination
    next_obj_num: u32,
    /// Whether to copy stream data immediately
    immediate_copy: bool,
}

impl CopyContext {
    /// Create a new copy context
    pub fn new(start_obj_num: u32) -> Self {
        Self {
            object_map: HashMap::new(),
            pending: HashSet::new(),
            next_obj_num: start_obj_num,
            immediate_copy: false,
        }
    }

    /// Set immediate copy mode
    ///
    /// When enabled, stream data is copied into memory immediately.
    /// This allows the source file to be closed before writing the destination.
    pub fn set_immediate_copy(&mut self, immediate: bool) {
        self.immediate_copy = immediate;
    }

    /// Reserve a destination object number for a source object
    pub fn reserve_object(&mut self, source: ObjGen) -> ObjGen {
        if let Some(&dest) = self.object_map.get(&source) {
            return dest;
        }

        let dest = ObjGen::new(self.next_obj_num, 0);
        self.next_obj_num += 1;
        self.object_map.insert(source, dest);
        self.pending.insert(source);
        dest
    }

    /// Mark an object as copied
    pub fn mark_copied(&mut self, source: ObjGen) {
        self.pending.remove(&source);
    }

    /// Check if an object has been mapped
    pub fn is_mapped(&self, source: ObjGen) -> bool {
        self.object_map.contains_key(&source)
    }

    /// Get the destination object for a source object
    pub fn get_destination(&self, source: ObjGen) -> Option<ObjGen> {
        self.object_map.get(&source).copied()
    }

    /// Get objects that still need to be copied
    pub fn pending_objects(&self) -> impl Iterator<Item = &ObjGen> {
        self.pending.iter()
    }

    /// Check if all objects have been copied
    pub fn is_complete(&self) -> bool {
        self.pending.is_empty()
    }
}

/// Configuration for object copying
#[derive(Debug, Clone)]
pub struct CopyConfig {
    /// Copy stream data immediately into memory
    pub immediate_copy: bool,
    /// Follow and copy referenced objects
    pub follow_references: bool,
    /// Maximum depth for following references
    pub max_depth: u32,
    /// Skip certain object types
    pub skip_types: HashSet<String>,
}

impl Default for CopyConfig {
    fn default() -> Self {
        Self {
            immediate_copy: false,
            follow_references: true,
            max_depth: 100,
            skip_types: HashSet::new(),
        }
    }
}

/// Extract object references from PDF object data
///
/// This scans the data for patterns like "N G R" and returns all references found.
pub fn extract_references(data: &[u8]) -> Vec<ObjGen> {
    let mut refs = Vec::new();
    let text = String::from_utf8_lossy(data);

    // Simple regex-like scanning for "N G R" pattern
    let mut chars = text.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        if c.is_ascii_digit() {
            // Might be start of reference
            let mut num_str = String::new();
            num_str.push(c);

            // Collect rest of first number
            while let Some(&(_, next_c)) = chars.peek() {
                if next_c.is_ascii_digit() {
                    num_str.push(next_c);
                    chars.next();
                } else {
                    break;
                }
            }

            // Look for whitespace
            let mut has_space = false;
            while let Some(&(_, next_c)) = chars.peek() {
                if next_c.is_whitespace() {
                    has_space = true;
                    chars.next();
                } else {
                    break;
                }
            }

            if !has_space {
                continue;
            }

            // Collect generation number
            let mut gen_str = String::new();
            while let Some(&(_, next_c)) = chars.peek() {
                if next_c.is_ascii_digit() {
                    gen_str.push(next_c);
                    chars.next();
                } else {
                    break;
                }
            }

            if gen_str.is_empty() {
                continue;
            }

            // Look for whitespace and 'R'
            while let Some(&(_, next_c)) = chars.peek() {
                if next_c.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }

            if let Some(&(_, 'R')) = chars.peek() {
                chars.next();
                // Found a reference
                if let (Ok(obj), Ok(generation)) = (num_str.parse::<u32>(), gen_str.parse::<u32>())
                {
                    refs.push(ObjGen::new(obj, generation));
                }
            }
        }
    }

    refs
}

/// Rewrite object references in data using the provided mapping
pub fn rewrite_references(data: &[u8], object_map: &ObjectMap) -> Result<Vec<u8>> {
    let text = String::from_utf8_lossy(data);
    let mut result = String::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        if c.is_ascii_digit() {
            let start_pos = i;
            let mut num_str = String::new();
            num_str.push(c);

            // Collect first number
            while let Some(&(_, next_c)) = chars.peek() {
                if next_c.is_ascii_digit() {
                    num_str.push(next_c);
                    chars.next();
                } else {
                    break;
                }
            }

            // Collect whitespace
            let mut space = String::new();
            while let Some(&(_, next_c)) = chars.peek() {
                if next_c.is_whitespace() {
                    space.push(next_c);
                    chars.next();
                } else {
                    break;
                }
            }

            // Collect generation number
            let mut gen_str = String::new();
            while let Some(&(_, next_c)) = chars.peek() {
                if next_c.is_ascii_digit() {
                    gen_str.push(next_c);
                    chars.next();
                } else {
                    break;
                }
            }

            // Check for 'R'
            let mut space2 = String::new();
            while let Some(&(_, next_c)) = chars.peek() {
                if next_c.is_whitespace() {
                    space2.push(next_c);
                    chars.next();
                } else {
                    break;
                }
            }

            let is_ref = if let Some(&(_, 'R')) = chars.peek() {
                chars.next();
                true
            } else {
                false
            };

            if is_ref && !gen_str.is_empty() {
                // This is a reference - check if we need to rewrite it
                if let (Ok(obj), Ok(generation)) = (num_str.parse::<u32>(), gen_str.parse::<u32>())
                {
                    let source = ObjGen::new(obj, generation);
                    if let Some(dest) = object_map.get(&source) {
                        result.push_str(&format!("{} {} R", dest.obj, dest.generation));
                        continue;
                    }
                }
                // Keep original if not mapped
                result.push_str(&num_str);
                result.push_str(&space);
                result.push_str(&gen_str);
                result.push_str(&space2);
                result.push('R');
            } else {
                // Not a reference, output what we collected
                result.push_str(&num_str);
                result.push_str(&space);
                result.push_str(&gen_str);
                result.push_str(&space2);
            }
        } else {
            result.push(c);
        }
    }

    Ok(result.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_objgen() {
        let og = ObjGen::new(1, 0);
        assert_eq!(og.obj, 1);
        assert_eq!(og.generation, 0);
        assert_eq!(og.to_string(), "1 0 R");
    }

    #[test]
    fn test_copy_context() {
        let mut ctx = CopyContext::new(100);

        let source1 = ObjGen::new(1, 0);
        let source2 = ObjGen::new(2, 0);

        let dest1 = ctx.reserve_object(source1);
        assert_eq!(dest1, ObjGen::new(100, 0));

        let dest2 = ctx.reserve_object(source2);
        assert_eq!(dest2, ObjGen::new(101, 0));

        // Re-reserving should return same destination
        assert_eq!(ctx.reserve_object(source1), ObjGen::new(100, 0));
    }

    #[test]
    fn test_extract_references() {
        let data = b"<</Type/Page/Contents 5 0 R/Resources 10 0 R>>";
        let refs = extract_references(data);

        assert_eq!(refs.len(), 2);
        assert!(refs.contains(&ObjGen::new(5, 0)));
        assert!(refs.contains(&ObjGen::new(10, 0)));
    }

    #[test]
    fn test_rewrite_references() {
        let data = b"<</Contents 5 0 R>>";
        let mut map = ObjectMap::new();
        map.insert(ObjGen::new(5, 0), ObjGen::new(100, 0));

        let result = rewrite_references(data, &map).unwrap();
        let result_str = String::from_utf8_lossy(&result);

        assert!(result_str.contains("100 0 R"));
        assert!(!result_str.contains("5 0 R"));
    }
}
