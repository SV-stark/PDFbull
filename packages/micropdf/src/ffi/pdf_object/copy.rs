//! PDF Object Copy Operations FFI Functions

use super::super::Handle;
use super::refcount::with_obj;
use super::types::{PDF_OBJECTS, PdfObj, PdfObjHandle, PdfObjType};

#[unsafe(no_mangle)]
pub extern "C" fn pdf_copy_array(_ctx: Handle, _doc: Handle, array: PdfObjHandle) -> PdfObjHandle {
    let copied = with_obj(array, None, |o| match &o.obj_type {
        PdfObjType::Array(arr) => {
            let mut new_arr = PdfObj::new_array(arr.len());
            if let PdfObjType::Array(ref mut new_vec) = new_arr.obj_type {
                for item in arr {
                    new_vec.push(item.clone());
                }
            }
            Some(new_arr)
        }
        _ => None,
    });

    match copied {
        Some(obj) => PDF_OBJECTS.insert(obj),
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_copy_dict(_ctx: Handle, _doc: Handle, dict: PdfObjHandle) -> PdfObjHandle {
    let copied = with_obj(dict, None, |o| match &o.obj_type {
        PdfObjType::Dict(entries) => {
            let mut new_dict = PdfObj::new_dict(entries.len());
            if let PdfObjType::Dict(ref mut new_entries) = new_dict.obj_type {
                for (k, v) in entries {
                    new_entries.push((k.clone(), v.clone()));
                }
            }
            Some(new_dict)
        }
        _ => None,
    });

    match copied {
        Some(obj) => PDF_OBJECTS.insert(obj),
        None => 0,
    }
}

fn deep_copy_obj_inner(obj: &PdfObj) -> PdfObj {
    let new_type = match &obj.obj_type {
        PdfObjType::Null => PdfObjType::Null,
        PdfObjType::Bool(b) => PdfObjType::Bool(*b),
        PdfObjType::Int(i) => PdfObjType::Int(*i),
        PdfObjType::Real(r) => PdfObjType::Real(*r),
        PdfObjType::Name(s) => PdfObjType::Name(s.clone()),
        PdfObjType::String(s) => PdfObjType::String(s.clone()),
        PdfObjType::Array(arr) => PdfObjType::Array(arr.iter().map(deep_copy_obj_inner).collect()),
        PdfObjType::Dict(entries) => PdfObjType::Dict(
            entries
                .iter()
                .map(|(k, v)| (k.clone(), deep_copy_obj_inner(v)))
                .collect(),
        ),
        PdfObjType::Indirect { num, generation } => PdfObjType::Indirect {
            num: *num,
            generation: *generation,
        },
        PdfObjType::Stream { dict, data } => PdfObjType::Stream {
            dict: Box::new(deep_copy_obj_inner(dict)),
            data: data.clone(),
        },
    };

    PdfObj {
        obj_type: new_type,
        marked: false,
        dirty: false,
        parent_num: 0,
        refs: 1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_deep_copy_obj(_ctx: Handle, _doc: Handle, obj: PdfObjHandle) -> PdfObjHandle {
    let copied = with_obj(obj, None, |o| Some(deep_copy_obj_inner(o)));

    match copied {
        Some(new_obj) => PDF_OBJECTS.insert(new_obj),
        None => 0,
    }
}
