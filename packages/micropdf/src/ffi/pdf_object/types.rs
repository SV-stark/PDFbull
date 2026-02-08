//! PDF Object Types and Core Data Structures

use super::super::{Handle, HandleStore};
use std::sync::LazyLock;

/// PDF Object type enumeration
#[derive(Debug, Clone)]
pub enum PdfObjType {
    Null,
    Bool(bool),
    Int(i64),
    Real(f64),
    Name(String),
    String(Vec<u8>),
    Array(Vec<PdfObj>),
    Dict(Vec<(String, PdfObj)>),
    Indirect { num: i32, generation: i32 },
    Stream { dict: Box<PdfObj>, data: Vec<u8> },
}

impl PdfObjType {
    /// Compare two object types for equality (shallow comparison)
    pub fn shallow_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (PdfObjType::Null, PdfObjType::Null) => true,
            (PdfObjType::Bool(a), PdfObjType::Bool(b)) => a == b,
            (PdfObjType::Int(a), PdfObjType::Int(b)) => a == b,
            (PdfObjType::Real(a), PdfObjType::Real(b)) => (a - b).abs() < f64::EPSILON,
            (PdfObjType::Name(a), PdfObjType::Name(b)) => a == b,
            (PdfObjType::String(a), PdfObjType::String(b)) => a == b,
            (PdfObjType::Array(a), PdfObjType::Array(b)) => a.len() == b.len(),
            (PdfObjType::Dict(a), PdfObjType::Dict(b)) => a.len() == b.len(),
            (
                PdfObjType::Indirect {
                    num: n1,
                    generation: g1,
                },
                PdfObjType::Indirect {
                    num: n2,
                    generation: g2,
                },
            ) => n1 == n2 && g1 == g2,
            (PdfObjType::Stream { .. }, PdfObjType::Stream { .. }) => false, // Streams never match
            _ => false,
        }
    }
}

/// Internal PDF object representation
#[derive(Debug, Clone)]
pub struct PdfObj {
    pub obj_type: PdfObjType,
    pub marked: bool,
    pub dirty: bool,
    pub parent_num: i32,
    pub refs: i32,
}

impl PdfObj {
    pub fn new_null() -> Self {
        Self {
            obj_type: PdfObjType::Null,
            marked: false,
            dirty: false,
            parent_num: 0,
            refs: 1,
        }
    }

    pub fn new_bool(b: bool) -> Self {
        Self {
            obj_type: PdfObjType::Bool(b),
            marked: false,
            dirty: false,
            parent_num: 0,
            refs: 1,
        }
    }

    pub fn new_int(i: i64) -> Self {
        Self {
            obj_type: PdfObjType::Int(i),
            marked: false,
            dirty: false,
            parent_num: 0,
            refs: 1,
        }
    }

    pub fn new_real(f: f64) -> Self {
        Self {
            obj_type: PdfObjType::Real(f),
            marked: false,
            dirty: false,
            parent_num: 0,
            refs: 1,
        }
    }

    pub fn new_name(s: &str) -> Self {
        Self {
            obj_type: PdfObjType::Name(s.to_string()),
            marked: false,
            dirty: false,
            parent_num: 0,
            refs: 1,
        }
    }

    pub fn new_string(data: &[u8]) -> Self {
        Self {
            obj_type: PdfObjType::String(data.to_vec()),
            marked: false,
            dirty: false,
            parent_num: 0,
            refs: 1,
        }
    }

    pub fn new_array(cap: usize) -> Self {
        Self {
            obj_type: PdfObjType::Array(Vec::with_capacity(cap)),
            marked: false,
            dirty: false,
            parent_num: 0,
            refs: 1,
        }
    }

    pub fn new_dict(cap: usize) -> Self {
        Self {
            obj_type: PdfObjType::Dict(Vec::with_capacity(cap)),
            marked: false,
            dirty: false,
            parent_num: 0,
            refs: 1,
        }
    }

    pub fn new_indirect(num: i32, generation: i32) -> Self {
        Self {
            obj_type: PdfObjType::Indirect { num, generation },
            marked: false,
            dirty: false,
            parent_num: 0,
            refs: 1,
        }
    }
}

/// Handle type for PDF objects
pub type PdfObjHandle = Handle;

/// Global PDF object storage
pub static PDF_OBJECTS: LazyLock<HandleStore<PdfObj>> = LazyLock::new(HandleStore::default);
