//! Link - Hyperlink handling for documents
//!
//! Provides structures and functions for managing interactive links in documents.

use crate::fitz::geometry::Rect;

/// Link destination type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkDestType {
    /// Fit page to window
    Fit,
    /// Fit page bounding box to window
    FitB,
    /// Fit page width to window
    FitH,
    /// Fit page bounding box width to window
    FitBH,
    /// Fit page height to window
    FitV,
    /// Fit page bounding box height to window
    FitBV,
    /// Fit rectangle to window
    FitR,
    /// Specific x, y, zoom destination
    XYZ,
}

/// Location within a document
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub chapter: i32,
    pub page: i32,
}

impl Location {
    pub fn new(chapter: i32, page: i32) -> Self {
        Self { chapter, page }
    }
}

/// Link destination specification
#[derive(Debug, Clone)]
pub struct LinkDest {
    /// Location in document
    pub location: Location,
    /// Destination type
    pub dest_type: LinkDestType,
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
    /// Width
    pub w: f32,
    /// Height
    pub h: f32,
    /// Zoom level
    pub zoom: f32,
}

impl LinkDest {
    /// Create a "none" destination
    pub fn none() -> Self {
        Self {
            location: Location::new(-1, -1),
            dest_type: LinkDestType::Fit,
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            zoom: 0.0,
        }
    }

    /// Create an XYZ destination
    pub fn xyz(chapter: i32, page: i32, x: f32, y: f32, zoom: f32) -> Self {
        Self {
            location: Location::new(chapter, page),
            dest_type: LinkDestType::XYZ,
            x,
            y,
            w: 0.0,
            h: 0.0,
            zoom,
        }
    }

    /// Check if destination is valid
    pub fn is_valid(&self) -> bool {
        self.location.page >= 0
    }
}

/// An interactive link
#[derive(Debug, Clone)]
pub struct Link {
    /// Hotspot rectangle (clickable area)
    pub rect: Rect,
    /// URI or internal destination
    pub uri: String,
}

impl Link {
    /// Create a new link
    pub fn new(rect: Rect, uri: impl Into<String>) -> Self {
        Self {
            rect,
            uri: uri.into(),
        }
    }

    /// Check if this is an external link (contains "://")
    pub fn is_external(&self) -> bool {
        self.uri.contains("://")
    }

    /// Get the scheme of the URI if it's external
    pub fn scheme(&self) -> Option<&str> {
        if let Some(pos) = self.uri.find("://") {
            Some(&self.uri[..pos])
        } else {
            None
        }
    }

    /// Check if link points to a specific page
    pub fn is_page_link(&self) -> bool {
        self.uri.starts_with('#')
    }

    /// Extract page number from internal link
    pub fn page_number(&self) -> Option<i32> {
        if !self.is_page_link() {
            return None;
        }

        // Try "#page=N" format
        if self.uri.starts_with("#page=") {
            return self.uri[6..].parse().ok();
        }

        // Try "#N" format
        if self.uri.len() > 1 {
            return self.uri[1..].parse().ok();
        }

        None
    }
}

/// A linked list of links (as in MuPDF)
#[derive(Clone)]
pub struct LinkList {
    links: Vec<Link>,
}

impl LinkList {
    /// Create a new empty link list
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    /// Add a link to the list
    pub fn push(&mut self, link: Link) {
        self.links.push(link);
    }

    /// Get all links as a slice
    pub fn links(&self) -> &[Link] {
        &self.links
    }

    /// Get number of links
    pub fn len(&self) -> usize {
        self.links.len()
    }

    /// Check if list is empty
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    /// Get the first link
    pub fn first(&self) -> Option<&Link> {
        self.links.first()
    }

    /// Get a link by index
    pub fn get(&self, index: usize) -> Option<&Link> {
        self.links.get(index)
    }

    /// Clear all links
    pub fn clear(&mut self) {
        self.links.clear();
    }

    /// Find link at a given point
    pub fn link_at_point(&self, x: f32, y: f32) -> Option<&Link> {
        self.links.iter().find(|link| link.rect.contains(x, y))
    }
}

impl Default for LinkList {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for LinkList {
    type Item = Link;
    type IntoIter = std::vec::IntoIter<Link>;

    fn into_iter(self) -> Self::IntoIter {
        self.links.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_dest_none() {
        let dest = LinkDest::none();
        assert!(!dest.is_valid());
        assert_eq!(dest.location.page, -1);
    }

    #[test]
    fn test_link_dest_xyz() {
        let dest = LinkDest::xyz(0, 5, 100.0, 200.0, 1.5);
        assert!(dest.is_valid());
        assert_eq!(dest.location.chapter, 0);
        assert_eq!(dest.location.page, 5);
        assert_eq!(dest.x, 100.0);
        assert_eq!(dest.y, 200.0);
        assert_eq!(dest.zoom, 1.5);
        assert_eq!(dest.dest_type, LinkDestType::XYZ);
    }

    #[test]
    fn test_link_new() {
        let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
        let link = Link::new(rect, "https://example.com");
        assert_eq!(link.rect, rect);
        assert_eq!(link.uri, "https://example.com");
    }

    #[test]
    fn test_link_is_external() {
        let external = Link::new(Rect::new(0.0, 0.0, 10.0, 10.0), "https://example.com");
        assert!(external.is_external());

        let internal = Link::new(Rect::new(0.0, 0.0, 10.0, 10.0), "#page=5");
        assert!(!internal.is_external());
    }

    #[test]
    fn test_link_scheme() {
        let https = Link::new(Rect::new(0.0, 0.0, 10.0, 10.0), "https://example.com");
        assert_eq!(https.scheme(), Some("https"));

        let ftp = Link::new(Rect::new(0.0, 0.0, 10.0, 10.0), "ftp://files.example.com");
        assert_eq!(ftp.scheme(), Some("ftp"));

        let internal = Link::new(Rect::new(0.0, 0.0, 10.0, 10.0), "#page=1");
        assert_eq!(internal.scheme(), None);
    }

    #[test]
    fn test_link_is_page_link() {
        let page_link = Link::new(Rect::new(0.0, 0.0, 10.0, 10.0), "#page=5");
        assert!(page_link.is_page_link());

        let external = Link::new(Rect::new(0.0, 0.0, 10.0, 10.0), "https://example.com");
        assert!(!external.is_page_link());
    }

    #[test]
    fn test_link_page_number() {
        let link1 = Link::new(Rect::new(0.0, 0.0, 10.0, 10.0), "#page=5");
        assert_eq!(link1.page_number(), Some(5));

        let link2 = Link::new(Rect::new(0.0, 0.0, 10.0, 10.0), "#10");
        assert_eq!(link2.page_number(), Some(10));

        let link3 = Link::new(Rect::new(0.0, 0.0, 10.0, 10.0), "https://example.com");
        assert_eq!(link3.page_number(), None);
    }

    #[test]
    fn test_link_list_new() {
        let list = LinkList::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_link_list_push() {
        let mut list = LinkList::new();
        let link = Link::new(Rect::new(0.0, 0.0, 10.0, 10.0), "#page=1");
        list.push(link);

        assert_eq!(list.len(), 1);
        assert!(!list.is_empty());
    }

    #[test]
    fn test_link_list_link_at_point() {
        let mut list = LinkList::new();

        let link1 = Link::new(Rect::new(0.0, 0.0, 100.0, 100.0), "#page=1");
        let link2 = Link::new(Rect::new(200.0, 200.0, 300.0, 300.0), "#page=2");

        list.push(link1);
        list.push(link2);

        // Point in first link
        let found1 = list.link_at_point(50.0, 50.0);
        assert!(found1.is_some());
        assert_eq!(found1.unwrap().uri, "#page=1");

        // Point in second link
        let found2 = list.link_at_point(250.0, 250.0);
        assert!(found2.is_some());
        assert_eq!(found2.unwrap().uri, "#page=2");

        // Point not in any link
        let found3 = list.link_at_point(500.0, 500.0);
        assert!(found3.is_none());
    }

    #[test]
    fn test_link_list_into_iter() {
        let mut list = LinkList::new();
        list.push(Link::new(Rect::new(0.0, 0.0, 10.0, 10.0), "#1"));
        list.push(Link::new(Rect::new(20.0, 20.0, 30.0, 30.0), "#2"));

        let links: Vec<_> = list.into_iter().collect();
        assert_eq!(links.len(), 2);
    }

    #[test]
    fn test_location() {
        let loc = Location::new(0, 5);
        assert_eq!(loc.chapter, 0);
        assert_eq!(loc.page, 5);
    }

    #[test]
    fn test_link_dest_type() {
        assert_eq!(LinkDestType::Fit, LinkDestType::Fit);
        assert_ne!(LinkDestType::Fit, LinkDestType::XYZ);
    }
}
