//! PDF Dictionary Operations FFI Functions

use super::super::Handle;
use super::refcount::{with_obj, with_obj_mut};
use super::types::{PDF_OBJECTS, PdfObj, PdfObjHandle, PdfObjType};
use std::ffi::{CStr, c_char};

#[unsafe(no_mangle)]
pub extern "C" fn pdf_dict_len(_ctx: Handle, dict: PdfObjHandle) -> i32 {
    with_obj(dict, 0, |o| match &o.obj_type {
        PdfObjType::Dict(d) => d.len() as i32,
        _ => 0,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_dict_puts(
    _ctx: Handle,
    dict: PdfObjHandle,
    key: *const c_char,
    val: PdfObjHandle,
) {
    if key.is_null() {
        return;
    }

    let key_str = unsafe { CStr::from_ptr(key) }
        .to_str()
        .unwrap_or("")
        .to_string();

    let val_obj = with_obj(val, None, |o| Some(o.clone()));

    if let Some(val_clone) = val_obj {
        with_obj_mut(dict, (), |d| {
            if let PdfObjType::Dict(ref mut dict_entries) = d.obj_type {
                if let Some(entry) = dict_entries.iter_mut().find(|(k, _)| k == &key_str) {
                    entry.1 = val_clone;
                } else {
                    dict_entries.push((key_str.clone(), val_clone));
                }
                d.dirty = true;
            }
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_dict_dels(_ctx: Handle, dict: PdfObjHandle, key: *const c_char) {
    if key.is_null() {
        return;
    }

    let key_str = unsafe { CStr::from_ptr(key) }
        .to_str()
        .unwrap_or("")
        .to_string();

    with_obj_mut(dict, (), |d| {
        if let PdfObjType::Dict(ref mut dict_entries) = d.obj_type {
            dict_entries.retain(|(k, _)| k != &key_str);
            d.dirty = true;
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_dict_put_int(_ctx: Handle, dict: PdfObjHandle, key: PdfObjHandle, x: i64) {
    let key_name = with_obj(key, None, |o| match &o.obj_type {
        PdfObjType::Name(s) => Some(s.clone()),
        _ => None,
    });

    if let Some(key_str) = key_name {
        with_obj_mut(dict, (), |d| {
            if let PdfObjType::Dict(ref mut dict_entries) = d.obj_type {
                let val = PdfObj::new_int(x);
                if let Some(entry) = dict_entries.iter_mut().find(|(k, _)| k == &key_str) {
                    entry.1 = val;
                } else {
                    dict_entries.push((key_str.clone(), val));
                }
                d.dirty = true;
            }
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_dict_put_real(_ctx: Handle, dict: PdfObjHandle, key: PdfObjHandle, x: f64) {
    let key_name = with_obj(key, None, |o| match &o.obj_type {
        PdfObjType::Name(s) => Some(s.clone()),
        _ => None,
    });

    if let Some(key_str) = key_name {
        with_obj_mut(dict, (), |d| {
            if let PdfObjType::Dict(ref mut dict_entries) = d.obj_type {
                let val = PdfObj::new_real(x);
                if let Some(entry) = dict_entries.iter_mut().find(|(k, _)| k == &key_str) {
                    entry.1 = val;
                } else {
                    dict_entries.push((key_str.clone(), val));
                }
                d.dirty = true;
            }
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_dict_put_bool(_ctx: Handle, dict: PdfObjHandle, key: PdfObjHandle, x: i32) {
    let key_name = with_obj(key, None, |o| match &o.obj_type {
        PdfObjType::Name(s) => Some(s.clone()),
        _ => None,
    });

    if let Some(key_str) = key_name {
        with_obj_mut(dict, (), |d| {
            if let PdfObjType::Dict(ref mut dict_entries) = d.obj_type {
                let val = PdfObj::new_bool(x != 0);
                if let Some(entry) = dict_entries.iter_mut().find(|(k, _)| k == &key_str) {
                    entry.1 = val;
                } else {
                    dict_entries.push((key_str.clone(), val));
                }
                d.dirty = true;
            }
        });
    }
}

// ============================================================================
// PDF Dictionary Get/Put Operations
// ============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn pdf_dict_get(
    _ctx: Handle,
    dict: PdfObjHandle,
    key: PdfObjHandle,
) -> PdfObjHandle {
    let key_name = with_obj(key, None, |o| match &o.obj_type {
        PdfObjType::Name(s) => Some(s.clone()),
        _ => None,
    });

    let key_str = match key_name {
        Some(k) => k,
        None => return 0,
    };

    let obj = with_obj(dict, None, |o| match &o.obj_type {
        PdfObjType::Dict(entries) => entries
            .iter()
            .find(|(k, _)| k == &key_str)
            .map(|(_, v)| v.clone()),
        _ => None,
    });

    match obj {
        Some(o) => PDF_OBJECTS.insert(o),
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_dict_gets(
    _ctx: Handle,
    dict: PdfObjHandle,
    key: *const c_char,
) -> PdfObjHandle {
    if key.is_null() {
        return 0;
    }

    let key_str = unsafe { CStr::from_ptr(key) }
        .to_str()
        .unwrap_or("")
        .to_string();

    let obj = with_obj(dict, None, |o| match &o.obj_type {
        PdfObjType::Dict(entries) => entries
            .iter()
            .find(|(k, _)| k == &key_str)
            .map(|(_, v)| v.clone()),
        _ => None,
    });

    match obj {
        Some(o) => PDF_OBJECTS.insert(o),
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_dict_put(
    _ctx: Handle,
    dict: PdfObjHandle,
    key: PdfObjHandle,
    val: PdfObjHandle,
) {
    let key_name = with_obj(key, None, |o| match &o.obj_type {
        PdfObjType::Name(s) => Some(s.clone()),
        _ => None,
    });

    let key_str = match key_name {
        Some(k) => k,
        None => return,
    };

    let val_obj = with_obj(val, None, |o| Some(o.clone()));

    if let Some(val_clone) = val_obj {
        with_obj_mut(dict, (), |d| {
            if let PdfObjType::Dict(ref mut entries) = d.obj_type {
                if let Some(entry) = entries.iter_mut().find(|(k, _)| k == &key_str) {
                    entry.1 = val_clone;
                } else {
                    entries.push((key_str.clone(), val_clone));
                }
                d.dirty = true;
            }
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_dict_put_name(
    _ctx: Handle,
    dict: PdfObjHandle,
    key: PdfObjHandle,
    name: *const c_char,
) {
    if name.is_null() {
        return;
    }

    let key_name = with_obj(key, None, |o| match &o.obj_type {
        PdfObjType::Name(s) => Some(s.clone()),
        _ => None,
    });

    let key_str = match key_name {
        Some(k) => k,
        None => return,
    };

    let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");

    with_obj_mut(dict, (), |d| {
        if let PdfObjType::Dict(ref mut entries) = d.obj_type {
            let val = PdfObj::new_name(name_str);
            if let Some(entry) = entries.iter_mut().find(|(k, _)| k == &key_str) {
                entry.1 = val;
            } else {
                entries.push((key_str.clone(), val));
            }
            d.dirty = true;
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_dict_put_string(
    _ctx: Handle,
    dict: PdfObjHandle,
    key: PdfObjHandle,
    str: *const c_char,
    len: usize,
) {
    let key_name = with_obj(key, None, |o| match &o.obj_type {
        PdfObjType::Name(s) => Some(s.clone()),
        _ => None,
    });

    let key_str = match key_name {
        Some(k) => k,
        None => return,
    };

    let data = if str.is_null() || len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(str as *const u8, len) }.to_vec()
    };

    with_obj_mut(dict, (), |d| {
        if let PdfObjType::Dict(ref mut entries) = d.obj_type {
            let val = PdfObj::new_string(&data);
            if let Some(entry) = entries.iter_mut().find(|(k, _)| k == &key_str) {
                entry.1 = val;
            } else {
                entries.push((key_str.clone(), val));
            }
            d.dirty = true;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::super::create::{pdf_new_dict, pdf_new_int, pdf_new_name};
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_pdf_dict_len() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);

        assert_eq!(pdf_dict_len(ctx, dict), 0); // Empty dict

        // Add an item
        let key = CString::new("Test").unwrap();
        let val = pdf_new_int(ctx, 42);
        pdf_dict_puts(ctx, dict, key.as_ptr(), val);

        assert_eq!(pdf_dict_len(ctx, dict), 1);
    }

    #[test]
    fn test_pdf_dict_puts_and_gets() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let key = CString::new("TestKey").unwrap();
        let val = pdf_new_int(ctx, 123);

        pdf_dict_puts(ctx, dict, key.as_ptr(), val);

        let retrieved = pdf_dict_gets(ctx, dict, key.as_ptr());
        assert_ne!(retrieved, 0);
    }

    #[test]
    fn test_pdf_dict_puts_null_key() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let val = pdf_new_int(ctx, 42);

        pdf_dict_puts(ctx, dict, std::ptr::null(), val);

        // Should not crash, just return without adding
        assert_eq!(pdf_dict_len(ctx, dict), 0);
    }

    #[test]
    fn test_pdf_dict_dels() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let key = CString::new("DeleteMe").unwrap();
        let val = pdf_new_int(ctx, 99);

        pdf_dict_puts(ctx, dict, key.as_ptr(), val);
        assert_eq!(pdf_dict_len(ctx, dict), 1);

        pdf_dict_dels(ctx, dict, key.as_ptr());
        assert_eq!(pdf_dict_len(ctx, dict), 0);
    }

    #[test]
    fn test_pdf_dict_dels_null_key() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);

        pdf_dict_dels(ctx, dict, std::ptr::null());

        // Should not crash
        assert_eq!(pdf_dict_len(ctx, dict), 0);
    }

    #[test]
    fn test_pdf_dict_put_int() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let key = pdf_new_name(ctx, CString::new("Count").unwrap().as_ptr());

        pdf_dict_put_int(ctx, dict, key, 42);

        assert_eq!(pdf_dict_len(ctx, dict), 1);
    }

    #[test]
    fn test_pdf_dict_put_real() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let key = pdf_new_name(ctx, CString::new("PI").unwrap().as_ptr());

        pdf_dict_put_real(ctx, dict, key, std::f64::consts::PI);

        assert_eq!(pdf_dict_len(ctx, dict), 1);
    }

    #[test]
    fn test_pdf_dict_put_bool() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let key = pdf_new_name(ctx, CString::new("Flag").unwrap().as_ptr());

        pdf_dict_put_bool(ctx, dict, key, 1);

        assert_eq!(pdf_dict_len(ctx, dict), 1);
    }

    #[test]
    fn test_pdf_dict_get() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let key = pdf_new_name(ctx, CString::new("TestKey").unwrap().as_ptr());
        let val = pdf_new_int(ctx, 777);

        pdf_dict_put(ctx, dict, key, val);

        let retrieved = pdf_dict_get(ctx, dict, key);
        assert_ne!(retrieved, 0);
    }

    #[test]
    fn test_pdf_dict_get_invalid_key() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let invalid_key = pdf_new_int(ctx, 123); // Not a name

        let result = pdf_dict_get(ctx, dict, invalid_key);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_pdf_dict_gets_null_key() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);

        let result = pdf_dict_gets(ctx, dict, std::ptr::null());
        assert_eq!(result, 0);
    }

    #[test]
    fn test_pdf_dict_put() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let key = pdf_new_name(ctx, CString::new("Key1").unwrap().as_ptr());
        let val = pdf_new_int(ctx, 100);

        pdf_dict_put(ctx, dict, key, val);

        assert_eq!(pdf_dict_len(ctx, dict), 1);
    }

    #[test]
    fn test_pdf_dict_put_invalid_key() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let invalid_key = pdf_new_int(ctx, 999); // Not a name
        let val = pdf_new_int(ctx, 100);

        pdf_dict_put(ctx, dict, invalid_key, val);

        // Should not add because key is not a name
        assert_eq!(pdf_dict_len(ctx, dict), 0);
    }

    #[test]
    fn test_pdf_dict_put_name() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let key = pdf_new_name(ctx, CString::new("Type").unwrap().as_ptr());
        let name = CString::new("Page").unwrap();

        pdf_dict_put_name(ctx, dict, key, name.as_ptr());

        assert_eq!(pdf_dict_len(ctx, dict), 1);
    }

    #[test]
    fn test_pdf_dict_put_name_null() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let key = pdf_new_name(ctx, CString::new("Type").unwrap().as_ptr());

        pdf_dict_put_name(ctx, dict, key, std::ptr::null());

        // Should not crash
        assert_eq!(pdf_dict_len(ctx, dict), 0);
    }

    #[test]
    fn test_pdf_dict_put_name_invalid_key() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let invalid_key = pdf_new_int(ctx, 123); // Not a name
        let name = CString::new("Test").unwrap();

        pdf_dict_put_name(ctx, dict, invalid_key, name.as_ptr());

        // Should not add
        assert_eq!(pdf_dict_len(ctx, dict), 0);
    }

    #[test]
    fn test_pdf_dict_put_string() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let key = pdf_new_name(ctx, CString::new("Title").unwrap().as_ptr());
        let str_data = b"My Title";

        pdf_dict_put_string(
            ctx,
            dict,
            key,
            str_data.as_ptr() as *const i8,
            str_data.len(),
        );

        assert_eq!(pdf_dict_len(ctx, dict), 1);
    }

    #[test]
    fn test_pdf_dict_put_string_null() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let key = pdf_new_name(ctx, CString::new("Title").unwrap().as_ptr());

        pdf_dict_put_string(ctx, dict, key, std::ptr::null(), 0);

        assert_eq!(pdf_dict_len(ctx, dict), 1); // Should add empty string
    }

    #[test]
    fn test_pdf_dict_put_string_invalid_key() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let invalid_key = pdf_new_int(ctx, 456);
        let str_data = b"Test";

        pdf_dict_put_string(
            ctx,
            dict,
            invalid_key,
            str_data.as_ptr() as *const i8,
            str_data.len(),
        );

        // Should not add
        assert_eq!(pdf_dict_len(ctx, dict), 0);
    }

    #[test]
    fn test_pdf_dict_update_existing_key() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 2);
        let key = CString::new("UpdateMe").unwrap();
        let val1 = pdf_new_int(ctx, 1);
        let val2 = pdf_new_int(ctx, 2);

        pdf_dict_puts(ctx, dict, key.as_ptr(), val1);
        assert_eq!(pdf_dict_len(ctx, dict), 1);

        pdf_dict_puts(ctx, dict, key.as_ptr(), val2);
        assert_eq!(pdf_dict_len(ctx, dict), 1); // Still 1, value updated
    }

    #[test]
    fn test_pdf_dict_multiple_keys() {
        let ctx = 0;
        let dict = pdf_new_dict(ctx, 0, 5);

        let key1 = CString::new("Key1").unwrap();
        let key2 = CString::new("Key2").unwrap();
        let key3 = CString::new("Key3").unwrap();

        pdf_dict_puts(ctx, dict, key1.as_ptr(), pdf_new_int(ctx, 1));
        pdf_dict_puts(ctx, dict, key2.as_ptr(), pdf_new_int(ctx, 2));
        pdf_dict_puts(ctx, dict, key3.as_ptr(), pdf_new_int(ctx, 3));

        assert_eq!(pdf_dict_len(ctx, dict), 3);
    }
}
