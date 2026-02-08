//! Cross-reference table - PDF object location tracking
//!
//! The xref table maps object numbers to file offsets for efficient PDF parsing.

use crate::fitz::error::{Error, Result};
use std::collections::HashMap;

/// Type of xref entry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrefEntryType {
    /// Free object (available for reuse)
    Free,
    /// In-use object (normal entry)
    InUse,
    /// Object in object stream
    ObjStm,
}

/// Cross-reference table entry
#[derive(Debug, Clone)]
pub struct XrefEntry {
    /// Entry type (free, in-use, or compressed in object stream)
    pub entry_type: XrefEntryType,
    /// Generation number
    pub generation: u16,
    /// File offset (for in-use) or object stream number (for compressed)
    pub offset: i64,
    /// Index within object stream (for compressed objects)
    pub stm_index: u16,
    /// Object number
    pub num: i32,
    /// Marked flag (for garbage collection)
    pub marked: bool,
}

impl XrefEntry {
    /// Create a new free entry
    pub fn free(num: i32, generation: u16) -> Self {
        Self {
            entry_type: XrefEntryType::Free,
            generation,
            offset: 0,
            stm_index: 0,
            num,
            marked: false,
        }
    }

    /// Create a new in-use entry
    pub fn in_use(num: i32, generation: u16, offset: i64) -> Self {
        Self {
            entry_type: XrefEntryType::InUse,
            generation,
            offset,
            stm_index: 0,
            num,
            marked: false,
        }
    }

    /// Create a new compressed entry (in object stream)
    pub fn compressed(num: i32, stm_num: i64, stm_index: u16) -> Self {
        Self {
            entry_type: XrefEntryType::ObjStm,
            generation: 0, // Compressed objects always have generation 0
            offset: stm_num,
            stm_index,
            num,
            marked: false,
        }
    }

    /// Check if entry is free
    pub fn is_free(&self) -> bool {
        self.entry_type == XrefEntryType::Free
    }

    /// Check if entry is in use
    pub fn is_in_use(&self) -> bool {
        self.entry_type == XrefEntryType::InUse
    }

    /// Check if entry is compressed
    pub fn is_compressed(&self) -> bool {
        self.entry_type == XrefEntryType::ObjStm
    }
}

/// Cross-reference subsection
#[derive(Debug, Clone)]
pub struct XrefSubsection {
    /// Starting object number for this subsection
    pub start: i32,
    /// Entries in this subsection
    pub entries: Vec<XrefEntry>,
}

impl XrefSubsection {
    /// Create a new subsection
    pub fn new(start: i32, count: usize) -> Self {
        Self {
            start,
            entries: Vec::with_capacity(count),
        }
    }

    /// Add an entry to this subsection
    pub fn add(&mut self, entry: XrefEntry) {
        self.entries.push(entry);
    }

    /// Get entry by index within subsection
    pub fn get(&self, index: usize) -> Option<&XrefEntry> {
        self.entries.get(index)
    }

    /// Get entry by object number
    pub fn get_by_num(&self, num: i32) -> Option<&XrefEntry> {
        if num < self.start {
            return None;
        }
        let index = (num - self.start) as usize;
        self.entries.get(index)
    }

    /// Get the range of object numbers in this subsection
    pub fn range(&self) -> (i32, i32) {
        (self.start, self.start + self.entries.len() as i32)
    }
}

/// Cross-reference table
pub struct XrefTable {
    /// Subsections of the xref table
    subsections: Vec<XrefSubsection>,
    /// Fast lookup map for object numbers to entries
    lookup: HashMap<i32, XrefEntry>,
    /// Maximum object number
    max_num: i32,
}

impl XrefTable {
    /// Create a new empty xref table
    pub fn new() -> Self {
        Self {
            subsections: Vec::new(),
            lookup: HashMap::new(),
            max_num: 0,
        }
    }

    /// Create xref table with initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            subsections: Vec::new(),
            lookup: HashMap::with_capacity(capacity),
            max_num: 0,
        }
    }

    /// Add a subsection to the xref table
    pub fn add_subsection(&mut self, subsection: XrefSubsection) {
        // Update max_num
        let (_, end) = subsection.range();
        if end > self.max_num {
            self.max_num = end;
        }

        // Add entries to lookup map
        for entry in &subsection.entries {
            self.lookup.insert(entry.num, entry.clone());
        }

        self.subsections.push(subsection);
    }

    /// Add a single entry
    pub fn add_entry(&mut self, entry: XrefEntry) {
        if entry.num > self.max_num {
            self.max_num = entry.num;
        }
        self.lookup.insert(entry.num, entry);
    }

    /// Get entry by object number
    pub fn get(&self, num: i32) -> Option<&XrefEntry> {
        self.lookup.get(&num)
    }

    /// Get mutable entry by object number
    pub fn get_mut(&mut self, num: i32) -> Option<&mut XrefEntry> {
        self.lookup.get_mut(&num)
    }

    /// Check if object exists
    pub fn contains(&self, num: i32) -> bool {
        self.lookup.contains_key(&num)
    }

    /// Update an entry
    pub fn update(&mut self, entry: XrefEntry) -> Result<()> {
        let num = entry.num;
        if !self.lookup.contains_key(&num) {
            return Err(Error::Generic(format!("Object {} not in xref table", num)));
        }
        self.lookup.insert(num, entry);
        Ok(())
    }

    /// Delete an entry (mark as free)
    pub fn delete(&mut self, num: i32) -> Result<()> {
        if let Some(entry) = self.lookup.get_mut(&num) {
            entry.entry_type = XrefEntryType::Free;
            entry.generation = entry.generation.wrapping_add(1); // Increment generation
            Ok(())
        } else {
            Err(Error::Generic(format!("Object {} not in xref table", num)))
        }
    }

    /// Allocate a new object number
    pub fn allocate(&mut self) -> i32 {
        self.max_num += 1;
        let num = self.max_num;

        // Add a free entry for now
        self.add_entry(XrefEntry::free(num, 0));
        num
    }

    /// Get the maximum object number
    pub fn max_num(&self) -> i32 {
        self.max_num
    }

    /// Get the number of objects
    pub fn len(&self) -> usize {
        self.lookup.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.lookup.is_empty()
    }

    /// Get all object numbers
    pub fn object_numbers(&self) -> Vec<i32> {
        let mut nums: Vec<i32> = self.lookup.keys().copied().collect();
        nums.sort_unstable();
        nums
    }

    /// Mark an object (for garbage collection)
    pub fn mark(&mut self, num: i32) {
        if let Some(entry) = self.lookup.get_mut(&num) {
            entry.marked = true;
        }
    }

    /// Unmark an object
    pub fn unmark(&mut self, num: i32) {
        if let Some(entry) = self.lookup.get_mut(&num) {
            entry.marked = false;
        }
    }

    /// Clear all marks
    pub fn clear_marks(&mut self) {
        for entry in self.lookup.values_mut() {
            entry.marked = false;
        }
    }

    /// Get all marked objects
    pub fn marked_objects(&self) -> Vec<i32> {
        self.lookup
            .values()
            .filter(|e| e.marked)
            .map(|e| e.num)
            .collect()
    }

    /// Get count of in-use objects
    pub fn in_use_count(&self) -> usize {
        self.lookup.values().filter(|e| e.is_in_use()).count()
    }

    /// Get count of free objects
    pub fn free_count(&self) -> usize {
        self.lookup.values().filter(|e| e.is_free()).count()
    }

    /// Get count of compressed objects
    pub fn compressed_count(&self) -> usize {
        self.lookup.values().filter(|e| e.is_compressed()).count()
    }
}

impl Default for XrefTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xref_entry_free() {
        let entry = XrefEntry::free(1, 0);
        assert_eq!(entry.num, 1);
        assert_eq!(entry.generation, 0);
        assert!(entry.is_free());
        assert!(!entry.is_in_use());
        assert!(!entry.is_compressed());
    }

    #[test]
    fn test_xref_entry_in_use() {
        let entry = XrefEntry::in_use(5, 0, 12345);
        assert_eq!(entry.num, 5);
        assert_eq!(entry.generation, 0);
        assert_eq!(entry.offset, 12345);
        assert!(!entry.is_free());
        assert!(entry.is_in_use());
        assert!(!entry.is_compressed());
    }

    #[test]
    fn test_xref_entry_compressed() {
        let entry = XrefEntry::compressed(10, 7, 3);
        assert_eq!(entry.num, 10);
        assert_eq!(entry.generation, 0); // Always 0 for compressed
        assert_eq!(entry.offset, 7); // Stream number
        assert_eq!(entry.stm_index, 3);
        assert!(!entry.is_free());
        assert!(!entry.is_in_use());
        assert!(entry.is_compressed());
    }

    #[test]
    fn test_xref_subsection_new() {
        let subsec = XrefSubsection::new(5, 10);
        assert_eq!(subsec.start, 5);
        assert_eq!(subsec.entries.len(), 0);
    }

    #[test]
    fn test_xref_subsection_add_get() {
        let mut subsec = XrefSubsection::new(5, 10);
        subsec.add(XrefEntry::in_use(5, 0, 100));
        subsec.add(XrefEntry::in_use(6, 0, 200));

        assert_eq!(subsec.entries.len(), 2);
        assert_eq!(subsec.get(0).unwrap().offset, 100);
        assert_eq!(subsec.get_by_num(6).unwrap().offset, 200);
    }

    #[test]
    fn test_xref_subsection_range() {
        let mut subsec = XrefSubsection::new(10, 5);
        for i in 0..5 {
            subsec.add(XrefEntry::in_use(10 + i, 0, i as i64 * 100));
        }

        let (start, end) = subsec.range();
        assert_eq!(start, 10);
        assert_eq!(end, 15);
    }

    #[test]
    fn test_xref_table_new() {
        let table = XrefTable::new();
        assert!(table.is_empty());
        assert_eq!(table.len(), 0);
        assert_eq!(table.max_num(), 0);
    }

    #[test]
    fn test_xref_table_add_entry() {
        let mut table = XrefTable::new();
        table.add_entry(XrefEntry::in_use(1, 0, 100));
        table.add_entry(XrefEntry::in_use(2, 0, 200));

        assert_eq!(table.len(), 2);
        assert_eq!(table.max_num(), 2);
        assert!(table.contains(1));
        assert!(table.contains(2));
    }

    #[test]
    fn test_xref_table_get() {
        let mut table = XrefTable::new();
        table.add_entry(XrefEntry::in_use(5, 0, 12345));

        let entry = table.get(5).unwrap();
        assert_eq!(entry.offset, 12345);

        assert!(table.get(99).is_none());
    }

    #[test]
    fn test_xref_table_update() {
        let mut table = XrefTable::new();
        table.add_entry(XrefEntry::in_use(5, 0, 100));

        let updated = XrefEntry::in_use(5, 0, 200);
        table.update(updated).unwrap();

        assert_eq!(table.get(5).unwrap().offset, 200);
    }

    #[test]
    fn test_xref_table_delete() {
        let mut table = XrefTable::new();
        table.add_entry(XrefEntry::in_use(5, 0, 100));

        table.delete(5).unwrap();

        let entry = table.get(5).unwrap();
        assert!(entry.is_free());
        assert_eq!(entry.generation, 1); // Generation incremented
    }

    #[test]
    fn test_xref_table_allocate() {
        let mut table = XrefTable::new();

        let num1 = table.allocate();
        let num2 = table.allocate();

        assert_eq!(num1, 1);
        assert_eq!(num2, 2);
        assert_eq!(table.max_num(), 2);
    }

    #[test]
    fn test_xref_table_add_subsection() {
        let mut table = XrefTable::new();
        let mut subsec = XrefSubsection::new(5, 3);
        subsec.add(XrefEntry::in_use(5, 0, 100));
        subsec.add(XrefEntry::in_use(6, 0, 200));
        subsec.add(XrefEntry::in_use(7, 0, 300));

        table.add_subsection(subsec);

        assert_eq!(table.len(), 3);
        assert_eq!(table.max_num(), 8);
        assert_eq!(table.get(6).unwrap().offset, 200);
    }

    #[test]
    fn test_xref_table_object_numbers() {
        let mut table = XrefTable::new();
        table.add_entry(XrefEntry::in_use(3, 0, 100));
        table.add_entry(XrefEntry::in_use(1, 0, 200));
        table.add_entry(XrefEntry::in_use(2, 0, 300));

        let nums = table.object_numbers();
        assert_eq!(nums, vec![1, 2, 3]);
    }

    #[test]
    fn test_xref_table_marking() {
        let mut table = XrefTable::new();
        table.add_entry(XrefEntry::in_use(1, 0, 100));
        table.add_entry(XrefEntry::in_use(2, 0, 200));

        table.mark(1);
        assert!(table.get(1).unwrap().marked);
        assert!(!table.get(2).unwrap().marked);

        let marked = table.marked_objects();
        assert_eq!(marked, vec![1]);

        table.clear_marks();
        assert!(!table.get(1).unwrap().marked);
    }

    #[test]
    fn test_xref_table_counts() {
        let mut table = XrefTable::new();
        table.add_entry(XrefEntry::in_use(1, 0, 100));
        table.add_entry(XrefEntry::free(2, 0));
        table.add_entry(XrefEntry::compressed(3, 5, 0));

        assert_eq!(table.in_use_count(), 1);
        assert_eq!(table.free_count(), 1);
        assert_eq!(table.compressed_count(), 1);
    }
}
