//! Page abstraction
use crate::fitz::geometry::Rect;

pub trait Page {
    fn bounds(&self) -> Rect;
}
