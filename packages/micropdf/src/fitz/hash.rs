//! Hash Table - MuPDF-compatible hash table wrapper
//!
//! Thin wrapper around Rust's HashMap to provide MuPDF semantics.

use std::collections::HashMap;

/// Maximum key length for hash table keys (for compatibility)
pub const HASH_TABLE_KEY_LENGTH: usize = 48;

/// Generic hash table with byte slice keys
///
/// Wraps Rust's HashMap with MuPDF-compatible semantics:
/// - Fixed-length keys
/// - Insert returns existing value if key exists (doesn't replace)
/// - Iteration and filtering support
pub struct HashTable<T> {
    map: HashMap<Vec<u8>, T>,
    keylen: usize,
}

impl<T> HashTable<T> {
    /// Create a new hash table with the specified initial capacity and key length
    pub fn new(capacity: usize, keylen: usize) -> Self {
        assert!(keylen <= HASH_TABLE_KEY_LENGTH, "Key length too large");
        Self {
            map: HashMap::with_capacity(capacity),
            keylen,
        }
    }

    /// Insert a key-value pair into the hash table
    ///
    /// MuPDF semantics: If a value with the same key already exists,
    /// returns Some(existing_value) and does NOT replace it.
    /// Otherwise, inserts the new value and returns None.
    pub fn insert(&mut self, key: &[u8], value: T) -> Option<T> {
        assert_eq!(key.len(), self.keylen, "Key length mismatch");

        if self.map.contains_key(key) {
            // Key exists, return the value without inserting
            Some(value)
        } else {
            self.map.insert(key.to_vec(), value);
            None
        }
    }

    /// Find a value by key
    pub fn find(&self, key: &[u8]) -> Option<&T> {
        assert_eq!(key.len(), self.keylen, "Key length mismatch");
        self.map.get(key)
    }

    /// Find a mutable value by key
    pub fn find_mut(&mut self, key: &[u8]) -> Option<&mut T> {
        assert_eq!(key.len(), self.keylen, "Key length mismatch");
        self.map.get_mut(key)
    }

    /// Remove a key-value pair from the hash table
    ///
    /// Returns the value if found, None otherwise.
    pub fn remove(&mut self, key: &[u8]) -> Option<T> {
        assert_eq!(key.len(), self.keylen, "Key length mismatch");
        self.map.remove(key)
    }

    /// Get the number of entries in the hash table
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Check if the hash table is empty
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Get the key length for this hash table
    pub fn keylen(&self) -> usize {
        self.keylen
    }

    /// Iterate over all key-value pairs
    pub fn for_each<F>(&self, mut callback: F)
    where
        F: FnMut(&[u8], &T),
    {
        for (key, value) in &self.map {
            callback(key, value);
        }
    }

    /// Iterate over all key-value pairs (mutable)
    pub fn for_each_mut<F>(&mut self, mut callback: F)
    where
        F: FnMut(&[u8], &mut T),
    {
        for (key, value) in &mut self.map {
            callback(key, value);
        }
    }

    /// Filter entries based on a predicate
    ///
    /// Removes all entries where the callback returns true.
    /// Returns a vector of removed (key, value) pairs.
    pub fn filter<F>(&mut self, mut predicate: F) -> Vec<(Vec<u8>, T)>
    where
        F: FnMut(&[u8], &T) -> bool,
    {
        let keys_to_remove: Vec<Vec<u8>> = self
            .map
            .iter()
            .filter(|(k, v)| predicate(k, v))
            .map(|(k, _)| k.clone())
            .collect();

        keys_to_remove
            .into_iter()
            .filter_map(|k| self.map.remove(&k).map(|v| (k, v)))
            .collect()
    }

    /// Clear all entries from the hash table
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Get all keys
    pub fn keys(&self) -> Vec<&[u8]> {
        self.map.keys().map(|k| k.as_slice()).collect()
    }

    /// Get all values
    pub fn values(&self) -> Vec<&T> {
        self.map.values().collect()
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    /// Reserve additional capacity
    pub fn reserve(&mut self, additional: usize) {
        self.map.reserve(additional);
    }
}

impl<T> Default for HashTable<T> {
    fn default() -> Self {
        Self::new(16, 4) // Default: 16 entries, 4-byte keys
    }
}

impl<T: Clone> Clone for HashTable<T> {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            keylen: self.keylen,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_table_new() {
        let table: HashTable<i32> = HashTable::new(10, 4);
        assert_eq!(table.len(), 0);
        assert!(table.is_empty());
        assert_eq!(table.keylen(), 4);
    }

    #[test]
    fn test_hash_table_insert_find() {
        let mut table: HashTable<i32> = HashTable::new(10, 4);
        let key = [1, 2, 3, 4];

        let existing = table.insert(&key, 42);
        assert!(existing.is_none());

        let found = table.find(&key);
        assert_eq!(found, Some(&42));
    }

    #[test]
    fn test_hash_table_insert_duplicate() {
        let mut table: HashTable<i32> = HashTable::new(10, 4);
        let key = [1, 2, 3, 4];

        assert!(table.insert(&key, 42).is_none());

        // MuPDF semantics: returns the new value, doesn't replace
        let duplicate = table.insert(&key, 99);
        assert_eq!(duplicate, Some(99));

        // Original value should still be there
        assert_eq!(table.find(&key), Some(&42));
    }

    #[test]
    fn test_hash_table_remove() {
        let mut table: HashTable<i32> = HashTable::new(10, 4);
        let key = [1, 2, 3, 4];

        table.insert(&key, 42);
        let removed = table.remove(&key);

        assert_eq!(removed, Some(42));
        assert!(table.is_empty());
        assert!(table.find(&key).is_none());
    }

    #[test]
    fn test_hash_table_find_mut() {
        let mut table: HashTable<i32> = HashTable::new(10, 4);
        let key = [1, 2, 3, 4];

        table.insert(&key, 42);

        if let Some(value) = table.find_mut(&key) {
            *value = 100;
        }

        assert_eq!(table.find(&key), Some(&100));
    }

    #[test]
    fn test_hash_table_for_each() {
        let mut table: HashTable<i32> = HashTable::new(10, 4);
        let key1 = [1, 2, 3, 4];
        let key2 = [5, 6, 7, 8];

        table.insert(&key1, 42);
        table.insert(&key2, 99);

        let mut sum = 0;
        table.for_each(|_key, value| {
            sum += value;
        });

        assert_eq!(sum, 141);
    }

    #[test]
    fn test_hash_table_for_each_mut() {
        let mut table: HashTable<i32> = HashTable::new(10, 4);
        let key1 = [1, 2, 3, 4];
        let key2 = [5, 6, 7, 8];

        table.insert(&key1, 42);
        table.insert(&key2, 99);

        table.for_each_mut(|_key, value| {
            *value *= 2;
        });

        assert_eq!(table.find(&key1), Some(&84));
        assert_eq!(table.find(&key2), Some(&198));
    }

    #[test]
    fn test_hash_table_filter() {
        let mut table: HashTable<i32> = HashTable::new(10, 4);
        let key1 = [1, 2, 3, 4];
        let key2 = [5, 6, 7, 8];
        let key3 = [9, 10, 11, 12];

        table.insert(&key1, 42);
        table.insert(&key2, 99);
        table.insert(&key3, 33);

        // Remove entries where value > 50
        let removed = table.filter(|_key, value| *value > 50);

        assert_eq!(removed.len(), 1);
        assert_eq!(table.len(), 2);
        assert!(table.find(&key2).is_none());
        assert!(table.find(&key1).is_some());
        assert!(table.find(&key3).is_some());
    }

    #[test]
    fn test_hash_table_clear() {
        let mut table: HashTable<i32> = HashTable::new(10, 4);
        table.insert(&[1, 2, 3, 4], 42);
        table.insert(&[5, 6, 7, 8], 99);

        assert_eq!(table.len(), 2);
        table.clear();
        assert!(table.is_empty());
    }

    #[test]
    fn test_hash_table_keys_values() {
        let mut table: HashTable<i32> = HashTable::new(10, 4);
        let key1 = [1, 2, 3, 4];
        let key2 = [5, 6, 7, 8];

        table.insert(&key1, 42);
        table.insert(&key2, 99);

        let keys = table.keys();
        assert_eq!(keys.len(), 2);

        let values = table.values();
        assert_eq!(values.len(), 2);

        let sum: i32 = values.iter().map(|&&v| v).sum();
        assert_eq!(sum, 141);
    }

    #[test]
    fn test_hash_table_capacity() {
        let mut table: HashTable<i32> = HashTable::new(10, 4);
        assert!(table.capacity() >= 10);

        table.reserve(100);
        assert!(table.capacity() >= 110);
    }

    #[test]
    #[should_panic(expected = "Key length mismatch")]
    fn test_hash_table_wrong_key_length() {
        let mut table: HashTable<i32> = HashTable::new(10, 4);
        table.insert(&[1, 2], 42); // Should panic: key too short
    }

    #[test]
    #[should_panic(expected = "Key length too large")]
    fn test_hash_table_key_too_large() {
        let _table: HashTable<i32> = HashTable::new(10, HASH_TABLE_KEY_LENGTH + 1);
    }

    #[test]
    fn test_hash_table_default() {
        let table: HashTable<String> = HashTable::default();
        assert_eq!(table.keylen(), 4);
        assert!(table.is_empty());
    }

    #[test]
    fn test_hash_table_clone() {
        let mut table: HashTable<i32> = HashTable::new(10, 4);
        table.insert(&[1, 2, 3, 4], 42);
        table.insert(&[5, 6, 7, 8], 99);

        let cloned = table.clone();
        assert_eq!(cloned.len(), 2);
        assert_eq!(cloned.find(&[1, 2, 3, 4]), Some(&42));
        assert_eq!(cloned.find(&[5, 6, 7, 8]), Some(&99));
    }

    #[test]
    fn test_hash_table_with_strings() {
        let mut table: HashTable<String> = HashTable::new(10, 8);
        let key1 = b"key1\0\0\0\0";
        let key2 = b"key2\0\0\0\0";

        table.insert(key1, "value1".to_string());
        table.insert(key2, "value2".to_string());

        assert_eq!(table.find(key1), Some(&"value1".to_string()));
        assert_eq!(table.find(key2), Some(&"value2".to_string()));
    }
}
