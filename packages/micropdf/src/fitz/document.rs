//! Document trait
use crate::fitz::error::Result;
use crate::pdf::document::Page;

pub trait Document {
    fn page_count(&self) -> i32;
    fn load_page(&self, page_num: i32) -> Result<Box<Page>>;
}
