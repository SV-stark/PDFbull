//! PDF Object FFI - MuPDF API Compatible Exports
//!
//! This module provides C-compatible exports for PDF object manipulation.

// Module declarations
pub mod arena;
pub mod array;
pub mod check;
pub mod compare;
pub mod copy;
pub mod create;
pub mod dict;
pub mod extract;
pub mod marking;
pub mod refcount;
pub mod string;
pub mod types;
pub mod utils;

// Re-export public types and functions
pub use arena::*;
pub use array::*;
pub use check::*;
pub use compare::*;
pub use copy::*;
pub use create::*;
pub use dict::*;
pub use extract::*;
pub use marking::*;
pub use refcount::*;
pub use string::*;
pub use types::{PDF_OBJECTS, PdfObj, PdfObjHandle, PdfObjType};
pub use utils::*;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_char;

    // ============================================================================
    // Object Creation Tests
    // ============================================================================

    #[test]
    fn test_pdf_new_null() {
        let null = pdf_new_null(0);
        assert_eq!(pdf_is_null(0, null), 1);
        assert_eq!(pdf_is_bool(0, null), 0);
        assert_eq!(pdf_is_int(0, null), 0);
    }

    #[test]
    fn test_pdf_new_bool() {
        let bool_true = pdf_new_bool(0, 1);
        assert_eq!(pdf_is_bool(0, bool_true), 1);
        assert_eq!(pdf_to_bool(0, bool_true), 1);

        let bool_false = pdf_new_bool(0, 0);
        assert_eq!(pdf_is_bool(0, bool_false), 1);
        assert_eq!(pdf_to_bool(0, bool_false), 0);

        // Non-zero should also be true
        let bool_nonzero = pdf_new_bool(0, 42);
        assert_eq!(pdf_to_bool(0, bool_nonzero), 1);
    }

    #[test]
    fn test_pdf_new_int() {
        let int_val = pdf_new_int(0, 42);
        assert_eq!(pdf_is_int(0, int_val), 1);
        assert_eq!(pdf_to_int(0, int_val), 42);
        assert_eq!(pdf_to_int64(0, int_val), 42);

        // Negative value
        let neg_val = pdf_new_int(0, -100);
        assert_eq!(pdf_to_int(0, neg_val), -100);
        assert_eq!(pdf_to_int64(0, neg_val), -100);

        // Large value
        let large_val = pdf_new_int(0, i64::MAX);
        assert_eq!(pdf_to_int64(0, large_val), i64::MAX);
    }

    #[test]
    fn test_pdf_new_real() {
        let real_val = pdf_new_real(0, std::f32::consts::PI);
        assert_eq!(pdf_is_real(0, real_val), 1);
        assert!((pdf_to_real(0, real_val) - std::f32::consts::PI).abs() < 0.01);

        // Negative value
        let neg_real = pdf_new_real(0, -2.5);
        assert!((pdf_to_real(0, neg_real) + 2.5).abs() < 0.01);
    }

    #[test]
    fn test_pdf_is_number() {
        let int_val = pdf_new_int(0, 42);
        let real_val = pdf_new_real(0, std::f32::consts::PI);
        let null_val = pdf_new_null(0);

        assert_eq!(pdf_is_number(0, int_val), 1);
        assert_eq!(pdf_is_number(0, real_val), 1);
        assert_eq!(pdf_is_number(0, null_val), 0);
    }

    #[test]
    fn test_pdf_new_name() {
        let name = pdf_new_name(0, c"Type".as_ptr());
        assert_eq!(pdf_is_name(0, name), 1);

        // Empty name
        let empty_name = pdf_new_name(0, std::ptr::null());
        assert_eq!(pdf_is_name(0, empty_name), 1);
    }

    #[test]
    fn test_pdf_new_string() {
        let data = b"Hello, PDF!";
        let str_obj = pdf_new_string(0, data.as_ptr() as *const c_char, data.len());
        assert_eq!(pdf_is_string(0, str_obj), 1);

        // Empty string
        let empty_str = pdf_new_string(0, std::ptr::null(), 0);
        assert_eq!(pdf_is_string(0, empty_str), 1);

        // Null pointer with non-zero length
        let null_str = pdf_new_string(0, std::ptr::null(), 10);
        assert_eq!(pdf_is_string(0, null_str), 1);
    }

    #[test]
    fn test_pdf_new_text_string() {
        let text_obj = pdf_new_text_string(0, c"Hello World".as_ptr());
        assert_eq!(pdf_is_string(0, text_obj), 1);

        // Null text
        let null_text = pdf_new_text_string(0, std::ptr::null());
        assert_eq!(pdf_is_string(0, null_text), 1);
    }

    #[test]
    fn test_pdf_new_indirect() {
        let indirect = pdf_new_indirect(0, 0, 10, 2);
        assert_eq!(pdf_is_indirect(0, indirect), 1);
        assert_eq!(pdf_to_num(0, indirect), 10);
        assert_eq!(pdf_to_gen(0, indirect), 2);
    }

    // ============================================================================
    // Reference Counting Tests
    // ============================================================================

    #[test]
    fn test_pdf_keep_drop_obj() {
        let obj = pdf_new_int(0, 42);
        assert_eq!(pdf_obj_refs(0, obj), 1);

        pdf_keep_obj(0, obj);
        assert_eq!(pdf_obj_refs(0, obj), 2);

        pdf_drop_obj(0, obj);
        assert_eq!(pdf_obj_refs(0, obj), 1);

        pdf_drop_obj(0, obj);
        // Object should be removed, so refs should be 0 (default)
        assert_eq!(pdf_obj_refs(0, obj), 0);
    }

    #[test]
    fn test_pdf_keep_invalid_handle() {
        let invalid = pdf_keep_obj(0, 99999);
        assert_eq!(invalid, 99999); // Should return same handle
    }

    // ============================================================================
    // Value Extraction with Defaults Tests
    // ============================================================================

    #[test]
    fn test_pdf_to_bool_default() {
        let bool_obj = pdf_new_bool(0, 1);
        let null_obj = pdf_new_null(0);

        assert_eq!(pdf_to_bool_default(0, bool_obj, 0), 1);
        assert_eq!(pdf_to_bool_default(0, null_obj, 99), 99);
    }

    #[test]
    fn test_pdf_to_int_default() {
        let int_obj = pdf_new_int(0, 42);
        let null_obj = pdf_new_null(0);
        let real_obj = pdf_new_real(0, 3.7);

        assert_eq!(pdf_to_int_default(0, int_obj, 0), 42);
        assert_eq!(pdf_to_int_default(0, null_obj, 99), 99);
        assert_eq!(pdf_to_int_default(0, real_obj, 0), 3); // Truncated
    }

    #[test]
    fn test_pdf_to_real_default() {
        let real_obj = pdf_new_real(0, std::f32::consts::PI);
        let null_obj = pdf_new_null(0);
        let int_obj = pdf_new_int(0, 5);

        assert!((pdf_to_real_default(0, real_obj, 0.0) - std::f32::consts::PI).abs() < 0.01);
        assert!((pdf_to_real_default(0, null_obj, 99.0) - 99.0).abs() < 0.01);
        assert!((pdf_to_real_default(0, int_obj, 0.0) - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_pdf_to_name() {
        let name = pdf_new_name(0, c"TestName".as_ptr());
        let ptr = pdf_to_name(0, name);
        assert!(!ptr.is_null());

        // Test non-name object returns empty
        let int_obj = pdf_new_int(0, 42);
        let ptr2 = pdf_to_name(0, int_obj);
        assert!(!ptr2.is_null());
    }

    // ============================================================================
    // Array Operations Tests
    // ============================================================================

    #[test]
    fn test_pdf_array_operations() {
        let arr = pdf_new_array(0, 0, 10);
        assert_eq!(pdf_is_array(0, arr), 1);
        assert_eq!(pdf_array_len(0, arr), 0);

        // Push an integer
        let int_obj = pdf_new_int(0, 42);
        pdf_array_push(0, arr, int_obj);
        assert_eq!(pdf_array_len(0, arr), 1);

        // Push primitives
        pdf_array_push_int(0, arr, 100);
        pdf_array_push_real(0, arr, std::f64::consts::PI);
        pdf_array_push_bool(0, arr, 1);
        assert_eq!(pdf_array_len(0, arr), 4);

        // Get elements
        let elem0 = pdf_array_get(0, arr, 0);
        assert_eq!(pdf_to_int(0, elem0), 42);

        let elem1 = pdf_array_get(0, arr, 1);
        assert_eq!(pdf_to_int(0, elem1), 100);

        // Delete element
        pdf_array_delete(0, arr, 0);
        assert_eq!(pdf_array_len(0, arr), 3);

        let new_elem0 = pdf_array_get(0, arr, 0);
        assert_eq!(pdf_to_int(0, new_elem0), 100);
    }

    #[test]
    fn test_pdf_array_put_insert() {
        let arr = pdf_new_array(0, 0, 5);

        // Initial push
        pdf_array_push_int(0, arr, 1);
        pdf_array_push_int(0, arr, 2);
        pdf_array_push_int(0, arr, 3);
        assert_eq!(pdf_array_len(0, arr), 3);

        // Replace element
        let new_val = pdf_new_int(0, 99);
        pdf_array_put(0, arr, 1, new_val);
        let elem = pdf_array_get(0, arr, 1);
        assert_eq!(pdf_to_int(0, elem), 99);

        // Insert element
        let insert_val = pdf_new_int(0, 50);
        pdf_array_insert(0, arr, 1, insert_val);
        assert_eq!(pdf_array_len(0, arr), 4);
        let inserted = pdf_array_get(0, arr, 1);
        assert_eq!(pdf_to_int(0, inserted), 50);
    }

    // ============================================================================
    // Dictionary Operations Tests
    // ============================================================================

    #[test]
    fn test_pdf_dict_operations() {
        let dict = pdf_new_dict(0, 0, 5);
        assert_eq!(pdf_is_dict(0, dict), 1);
        assert_eq!(pdf_dict_len(0, dict), 0);

        // Add entries using puts
        let val = pdf_new_int(0, 42);
        pdf_dict_puts(0, dict, c"IntKey".as_ptr(), val);
        assert_eq!(pdf_dict_len(0, dict), 1);

        // Add more entries
        let key2 = pdf_new_name(0, c"RealKey".as_ptr());
        pdf_dict_put_real(0, dict, key2, std::f64::consts::PI);
        assert_eq!(pdf_dict_len(0, dict), 2);

        // Retrieve value
        let retrieved = pdf_dict_gets(0, dict, c"IntKey".as_ptr());
        assert_eq!(pdf_to_int(0, retrieved), 42);

        // Delete entry
        pdf_dict_dels(0, dict, c"IntKey".as_ptr());
        assert_eq!(pdf_dict_len(0, dict), 1);
    }

    #[test]
    fn test_pdf_dict_put_typed() {
        let dict = pdf_new_dict(0, 0, 5);

        let key1 = pdf_new_name(0, c"Int".as_ptr());
        pdf_dict_put_int(0, dict, key1, 100);

        let key2 = pdf_new_name(0, c"Bool".as_ptr());
        pdf_dict_put_bool(0, dict, key2, 1);

        let key3 = pdf_new_name(0, c"Real".as_ptr());
        pdf_dict_put_real(0, dict, key3, 2.5);

        assert_eq!(pdf_dict_len(0, dict), 3);

        let int_val = pdf_dict_gets(0, dict, c"Int".as_ptr());
        assert_eq!(pdf_to_int(0, int_val), 100);
    }

    // ============================================================================
    // Object Marking Tests
    // ============================================================================

    #[test]
    fn test_pdf_mark_obj() {
        let obj = pdf_new_int(0, 42);

        // Initially not marked
        assert_eq!(pdf_obj_marked(0, obj), 0);

        // Mark it
        let was_marked = pdf_mark_obj(0, obj);
        assert_eq!(was_marked, 0); // Was not marked before
        assert_eq!(pdf_obj_marked(0, obj), 1);

        // Mark again
        let was_marked2 = pdf_mark_obj(0, obj);
        assert_eq!(was_marked2, 1); // Was already marked

        // Unmark
        pdf_unmark_obj(0, obj);
        assert_eq!(pdf_obj_marked(0, obj), 0);
    }

    // ============================================================================
    // Dirty Tracking Tests
    // ============================================================================

    #[test]
    fn test_pdf_dirty_obj() {
        let obj = pdf_new_int(0, 42);

        // Initially not dirty
        assert_eq!(pdf_obj_is_dirty(0, obj), 0);

        // Mark as dirty
        pdf_dirty_obj(0, obj);
        assert_eq!(pdf_obj_is_dirty(0, obj), 1);

        // Clean it
        pdf_clean_obj(0, obj);
        assert_eq!(pdf_obj_is_dirty(0, obj), 0);
    }

    // ============================================================================
    // Parent Tracking Tests
    // ============================================================================

    #[test]
    fn test_pdf_obj_parent() {
        let obj = pdf_new_int(0, 42);

        // Initially no parent
        assert_eq!(pdf_obj_parent_num(0, obj), 0);

        // Set parent
        pdf_set_obj_parent(0, obj, 100);
        assert_eq!(pdf_obj_parent_num(0, obj), 100);

        // Change parent
        pdf_set_obj_parent(0, obj, 200);
        assert_eq!(pdf_obj_parent_num(0, obj), 200);
    }

    // ============================================================================
    // Comparison Tests
    // ============================================================================

    #[test]
    fn test_pdf_objcmp() {
        let int1 = pdf_new_int(0, 42);
        let int2 = pdf_new_int(0, 42);
        let int3 = pdf_new_int(0, 99);

        // Same values should be equal
        assert_eq!(pdf_objcmp(0, int1, int2), 0);

        // Different values should not be equal
        assert_eq!(pdf_objcmp(0, int1, int3), 1);
    }

    #[test]
    fn test_pdf_name_eq() {
        let name1 = pdf_new_name(0, c"Type".as_ptr());
        let name2 = pdf_new_name(0, c"Type".as_ptr());
        let name3 = pdf_new_name(0, c"Other".as_ptr());

        assert_eq!(pdf_name_eq(0, name1, name2), 1);
        assert_eq!(pdf_name_eq(0, name1, name3), 0);
    }

    // ============================================================================
    // Copy Tests
    // ============================================================================

    #[test]
    fn test_pdf_copy_array() {
        let arr = pdf_new_array(0, 0, 3);
        pdf_array_push_int(0, arr, 1);
        pdf_array_push_int(0, arr, 2);
        pdf_array_push_int(0, arr, 3);

        let copy = pdf_copy_array(0, 0, arr);
        assert_eq!(pdf_array_len(0, copy), 3);

        let elem = pdf_array_get(0, copy, 0);
        assert_eq!(pdf_to_int(0, elem), 1);
    }

    #[test]
    fn test_pdf_copy_dict() {
        let dict = pdf_new_dict(0, 0, 2);
        let key = pdf_new_name(0, c"Key".as_ptr());
        pdf_dict_put_int(0, dict, key, 42);

        let copy = pdf_copy_dict(0, 0, dict);
        assert_eq!(pdf_dict_len(0, copy), 1);

        let val = pdf_dict_gets(0, copy, c"Key".as_ptr());
        assert_eq!(pdf_to_int(0, val), 42);
    }

    #[test]
    fn test_pdf_deep_copy_obj() {
        let arr = pdf_new_array(0, 0, 2);
        pdf_array_push_int(0, arr, 10);

        let nested_arr = pdf_new_array(0, 0, 1);
        pdf_array_push_int(0, nested_arr, 20);
        pdf_array_push(0, arr, nested_arr);

        let deep_copy = pdf_deep_copy_obj(0, 0, arr);
        assert_eq!(pdf_array_len(0, deep_copy), 2);
    }

    // ============================================================================
    // Geometry Object Tests
    // ============================================================================

    #[test]
    fn test_pdf_new_point() {
        let point = pdf_new_point(0, 0, 10.0, 20.0);
        assert_eq!(pdf_is_array(0, point), 1);
        assert_eq!(pdf_array_len(0, point), 2);

        let x = pdf_array_get(0, point, 0);
        assert!((pdf_to_real(0, x) - 10.0).abs() < 0.01);

        let y = pdf_array_get(0, point, 1);
        assert!((pdf_to_real(0, y) - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_pdf_new_rect() {
        let rect = pdf_new_rect(0, 0, 0.0, 0.0, 100.0, 200.0);
        assert_eq!(pdf_is_array(0, rect), 1);
        assert_eq!(pdf_array_len(0, rect), 4);
    }

    #[test]
    fn test_pdf_new_matrix() {
        let matrix = pdf_new_matrix(0, 0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        assert_eq!(pdf_is_array(0, matrix), 1);
        assert_eq!(pdf_array_len(0, matrix), 6);
    }

    #[test]
    fn test_pdf_new_date() {
        let date = pdf_new_date(0, 0, 2024, 1, 15, 10, 30, 45);
        assert_eq!(pdf_is_string(0, date), 1);
    }

    // ============================================================================
    // Dictionary Key Access Tests
    // ============================================================================

    #[test]
    fn test_pdf_dict_get_key_val() {
        let dict = pdf_new_dict(0, 0, 2);

        let key1 = pdf_new_name(0, c"First".as_ptr());
        pdf_dict_put_int(0, dict, key1, 100);

        let key2 = pdf_new_name(0, c"Second".as_ptr());
        pdf_dict_put_int(0, dict, key2, 200);

        // Get key at index 0
        let key_obj = pdf_dict_get_key(0, dict, 0);
        assert_eq!(pdf_is_name(0, key_obj), 1);

        // Get value at index 0
        let val_obj = pdf_dict_get_val(0, dict, 0);
        assert_eq!(pdf_to_int(0, val_obj), 100);
    }
}
