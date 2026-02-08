# Test Fixtures

PDF test documents for MicroPDF integration testing.

## Files

### `minimal.pdf`
A minimal valid PDF document with:
- Single page
- Basic text content ("Hello, World!")
- Helvetica font (Type1)
- Standard US Letter size (612x792 points)

### `multipage.pdf`
A multi-page PDF for testing page handling:
- 5 pages
- Simple text content on each page
- Single font
- Standard page size

### `comprehensive_test.pdf`
A comprehensive PDF containing all major PDF features for testing:

#### Structure
- 3 pages
- PDF version 1.7
- Document catalog with all optional entries

#### Fonts
- Helvetica (Type1)
- Times-Roman (Type1)

#### Annotations
- Link annotation (external URL)
- Text annotation (sticky note)
- Highlight annotation
- Widget annotations (form fields)

#### Form Fields (AcroForm)
- Text field (`name_field`)
- Checkbox (`checkbox_field`)
- Dropdown/Choice field (`dropdown_field`)

#### Metadata
- XMP metadata stream
- Document info dictionary (Title, Author, Subject, Keywords, Creator, Producer, dates)

#### Outlines/Bookmarks
- Two chapters with destinations

#### Named Destinations
- Named destinations for each page

#### Graphics Features
- Inline image (8x8 RGB checkerboard)
- Pattern (tiling pattern)
- Extended Graphics State (transparency)
- ICC-based colorspace reference
- Compressed content stream (FlateDecode on page 2)

#### Page Features
- MediaBox and CropBox
- Resource dictionaries with Font, XObject, Pattern, ExtGState, ColorSpace

### `encrypted_empty_password.pdf`
A PDF encrypted with the Standard security handler:
- Standard encryption (V1, R2)
- Empty password
- For testing encryption detection

## Usage in Tests

```rust
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(name);
    path
}

fn read_fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixture_path(name))
        .expect(&format!("Failed to read fixture: {}", name))
}
```

## Adding New Fixtures

When adding new test PDFs:

1. Place them in this directory
2. Document the features they test above
3. Git LFS will automatically handle them (see `.gitattributes`)
4. Add corresponding tests in `../integration_tests.rs`

## PDF Specification Reference

These fixtures are designed to test conformance with:
- PDF 1.4 through 1.7 specification
- ISO 32000-1:2008

Key PDF Reference Manual sections covered:
- §7.3 - Objects (null, bool, int, real, string, name, array, dict, stream)
- §7.5 - File Structure (header, body, xref, trailer)
- §7.6 - Encryption
- §7.7 - Document Structure (catalog, pages tree)
- §8 - Graphics
- §9 - Text
- §10 - Rendering
- §12.3 - Document-level Navigation (outlines, named destinations)
- §12.4 - Page-level Navigation (links, annotations)
- §12.5 - Annotations
- §12.7 - Interactive Forms (AcroForm)

