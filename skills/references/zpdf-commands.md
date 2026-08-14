# zpdf Command Reference

Complete reference for all zpdf CLI commands — reading, analysis, conversion, editing, and optimization.

## Reading Commands

### info — PDF Metadata
```bash
zpdf info <file.pdf> [--password <pw>]
```

**Output**:
- Page count
- PDF version
- Encryption status
- Document metadata (title, author, etc.)
- Page dimensions

**Example**:
```bash
$ zpdf info report.pdf
Pages: 25
Version: PDF-1.7
Encrypted: No
Title: Annual Report 2024
Author: Example Corp
Page size: 8.5 × 11 inches (612 × 792 pt)
```

**Use when**: Need to know page count, encryption, or document structure before processing.

---

### text — Extract Text
```bash
zpdf text <file.pdf> [-p <page>] [--all] [--struct] [--password <pw>]
```

**Flags**:
- `-p <N>` — Extract single page (1-indexed)
- `--all` — Extract all pages (sequential output)
- `--struct` — Preserve structural markup (if available)
- `--password <pw>` — Decrypt with password

**Examples**:
```bash
# Single page
zpdf text doc.pdf -p 5

# All pages
zpdf text doc.pdf --all

# Password-protected
zpdf text secure.pdf --password secret123 --all

# Structured (accessibility tree)
zpdf text doc.pdf -p 1 --struct
```

**Output**: Plain text, UTF-8 encoded, preserves line breaks.

**Use when**: Text-heavy PDFs, search corpus building, simple analysis.

---

### render — Render to Image
```bash
zpdf render <file.pdf> -p <page> -o <output.png> [--dpi <dpi>] [--backend cpu|wgpu] [--stats] [--password <pw>]
```

**Flags**:
- `-p <N>` — Page number (required)
- `-o <path>` — Output PNG path (required)
- `--dpi <N>` — Resolution (default: 72)
  - 72: Quick preview
  - 150: **Recommended default**
  - 300: High quality for detail/OCR
- `--backend cpu|wgpu` — Rendering backend (default: cpu)
- `--stats` — Print rendering performance stats
- `--password <pw>` — Decrypt with password

**Examples**:
```bash
# Standard quality
zpdf render doc.pdf -p 1 -o page1.png --dpi 150

# High quality for OCR
zpdf render diagram.pdf -p 3 -o detail.png --dpi 300

# Password-protected
zpdf render secure.pdf -p 1 -o out.png --dpi 150 --password secret

# GPU rendering (2-3× faster for complex pages)
zpdf render complex.pdf -p 10 -o out.png --dpi 150 --backend wgpu
```

**Output**: PNG image, 24-bit RGB.

**Use when**: Layout matters, diagrams present, vision capabilities available.

---

### search — Full-Text Search
```bash
zpdf search <file.pdf> <query> [-p <page>] [--case-sensitive] [--password <pw>]
```

**Flags**:
- `-p <N>` — Search only specified page
- `--case-sensitive` — Match case exactly
- `--password <pw>` — Decrypt with password

**Output**: Page numbers and surrounding context for each match.

**Example**:
```bash
$ zpdf search report.pdf "quarterly revenue"
Page 3: ...showing quarterly revenue growth of 15%...
Page 12: ...projected quarterly revenue for Q3...
Page 18: ...quarterly revenue comparison chart...
```

**Use when**: Finding specific content before reading full pages.

---

### convert — Convert to Text/Markdown/HTML
```bash
zpdf convert <file.pdf> -o <output> [--mode text|rich] [--format txt|md|html] [-p <page>|--pages <list>|--all] [--struct] [--images-dir <dir>] [--password <pw>]
```

**Flags**:
- `-o <path>` — Output file path (required)
- `--mode text|rich` — Text only or rich with formatting (default: text)
- `--format txt|md|html` — Output format (auto-detected from extension)
- `-p <N>` — Convert single page
- `--pages 1,3-5` — Convert page list/ranges
- `--all` — Convert all pages
- `--struct` — Use structure tree if available
- `--images-dir <dir>` — Extract images to directory (HTML mode)
- `--password <pw>` — Decrypt with password

**Examples**:
```bash
# PDF → Markdown
zpdf convert paper.pdf -o paper.md --format md --all

# PDF → HTML with images
zpdf convert doc.pdf -o doc.html --format html --images-dir ./img --all

# Specific pages
zpdf convert report.pdf -o summary.txt --pages 1,5-10 --format txt
```

**Use when**: Need editable text format or web publishing.

---

### export-pptx — Export to PowerPoint
```bash
zpdf export-pptx <file.pdf> -o <output.pptx> [-p <page>|--pages <list>|--all] [--password <pw>]
```

**Flags**:
- `-o <path>` — Output PowerPoint file (required)
- `-p <N>` — Export single page
- `--pages 1,3-5` — Export page list/ranges
- `--all` — Export all pages
- `--password <pw>` — Decrypt with password

**Examples**:
```bash
# Export all slides
zpdf export-pptx slides.pdf -o presentation.pptx --all

# Selected pages
zpdf export-pptx deck.pdf -o summary.pptx --pages 1,5-10,20
```

**Use when**: Converting PDF presentations to editable PowerPoint format.

---

## Analysis Commands

### tables — Extract Tables
```bash
zpdf tables <file.pdf> [-p <page>] [--all] [--csv] [--password <pw>]
```

**Flags**:
- `-p <N>` — Extract tables from single page
- `--all` — Extract from all pages
- `--csv` — Output in CSV format
- `--password <pw>` — Decrypt with password

**Output**: Structured table data (text or CSV format).

**Example**:
```bash
$ zpdf tables financial.pdf -p 5
Table 1 (rows: 10, cols: 4):
Quarter | Revenue | Expenses | Profit
Q1 2024 | $1.2M   | $800K    | $400K
Q2 2024 | $1.5M   | $900K    | $600K
...

$ zpdf tables financial.pdf -p 5 --csv > data.csv
```

**Use when**: Extracting tabular data without rendering full page.

---

### forms — Extract Form Fields
```bash
zpdf forms <file.pdf> [--password <pw>]
```

**Output**: List of all interactive form fields with current values.

**Example**:
```bash
$ zpdf forms application.pdf
Field: name (Type: text) = "John Doe"
Field: email (Type: text) = "john@example.com"
Field: agree (Type: checkbox) = checked
Field: country (Type: dropdown) = "United States"
```

**Use when**: Reading filled forms or understanding form structure before filling.

---

### outline — Document Outline
```bash
zpdf outline <file.pdf> [--password <pw>]
```

**Output**: Hierarchical table of contents with page references.

**Example**:
```bash
$ zpdf outline manual.pdf
1. Introduction (page 1)
2. Getting Started (page 5)
   2.1. Installation (page 6)
   2.2. Configuration (page 10)
3. Advanced Topics (page 25)
```

**Use when**: Understanding document structure before deep reading.

---

### links — Extract Hyperlinks
```bash
zpdf links <file.pdf> [--password <pw>]
```

**Output**: All hyperlinks in the document.

**Example**:
```bash
$ zpdf links doc.pdf
Page 3: https://example.com/documentation
Page 3: Page 15 (internal link)
Page 5: mailto:support@example.com
```

**Use when**: Extracting references, checking external resources.

---

### struct — PDF Structure Tree
```bash
zpdf struct <file.pdf> [--password <pw>]
```

**Output**: Accessibility structure (tagged PDF markup).

**Use when**: Understanding document semantics, accessibility analysis.

---

### signatures — Verify Digital Signatures
```bash
zpdf signatures <file.pdf> [--trust <roots.pem>] [--password <pw>]
```

**Flags**:
- `--trust <path>` — PEM file with trusted root certificates
- `--password <pw>` — Decrypt with password

**Output**: Signature validity, signer identity, timestamp.

**Example**:
```bash
$ zpdf signatures signed.pdf
Signature 1: Valid
  Signer: John Doe (john@example.com)
  Timestamp: 2024-07-26 10:30:45 UTC
  Certificate: CN=John Doe, O=Example Corp
  Valid: Yes
```

**Use when**: Verifying document authenticity.

---

### attachments — Embedded Files
```bash
zpdf attachments <file.pdf> [--extract <index|name|all>] [--out-dir <dir>] [--password <pw>]
```

**Flags**:
- `--extract <spec>` — Extract by index, name, or all
- `--out-dir <dir>` — Output directory (default: current dir)
- `--password <pw>` — Decrypt with password

**Output**: Names and sizes of embedded files (PDF portfolios, ZUGFeRD invoices).

**Example**:
```bash
$ zpdf attachments invoice.pdf
Attachment 0: invoice_data.xml (2.3 KB)
Attachment 1: logo.png (15.7 KB)

$ zpdf attachments invoice.pdf --extract all --out-dir ./extracted/
Extracted: invoice_data.xml → ./extracted/invoice_data.xml
Extracted: logo.png → ./extracted/logo.png
```

**Use when**: Finding or extracting embedded documents within PDFs.

---

### validate — PDF/A & PDF/UA Validation
```bash
zpdf validate <file.pdf> [--profile pdfa-1b|pdfa-2b|pdfua-1] [--password <pw>]
```

**Flags**:
- `--profile <spec>` — Conformance profile to validate against:
  - `pdfa-1b` / `pdfa-2b` — PDF/A archival conformance (encryption, `/ID`, header version, XMP `pdfaid`, `GTS_PDFA1` output intent + ICC, font embedding, forbidden features incl. JS/Launch actions, embedded files, transparency, forbidden annotation subtypes)
  - `pdfua-1` — PDF/UA-1 accessibility conformance (tagged, structure tree, `/Lang`, figure alt-text, headings, role mapping, table structure, annotation `/OBJR` coverage, `/StructParents`)
- `--password <pw>` — Decrypt with password

**Output**: Validation result with any violations (exit code 3 on FAIL for PDF/A).

**Use when**: Checking PDF/A compliance for archival, or PDF/UA compliance for accessibility (e.g. after `zpdf tag` adds a structure tree).

---

### compare — Visual Diff
```bash
zpdf compare <a.png> <b.png> [--out <diff.png>] [--threshold <0-255>]
```

**Flags**:
- `--out <path>` — Output difference image
- `--threshold <N>` — Color difference threshold (default: 10)

**Output**: Visual differences between two rendered pages.

**Use when**: Tracking document changes, regression testing.

---

## Editing Commands

### fill — Fill Form Fields
```bash
zpdf fill <file.pdf> --set NAME=VALUE [--set ...] [--list] -o <out.pdf>
```

**Flags**:
- `--set name=value` — Set field value (repeat for multiple fields)
- `--list` — List available field names
- `-o <path>` — Output file (required)

**Examples**:
```bash
# List fields first
zpdf fill form.pdf --list

# Fill form
zpdf fill application.pdf \
  --set name="John Doe" \
  --set email="john@example.com" \
  --set agree=true \
  -o filled.pdf
```

**Use when**: Programmatically filling PDF forms.

---

### merge — Merge PDFs
```bash
zpdf merge <a.pdf> <b.pdf> [more.pdf ...] -o <out.pdf>
```

**Flags**:
- `-o <path>` — Output file (required)

**Example**:
```bash
zpdf merge intro.pdf body.pdf appendix.pdf -o complete.pdf
```

**Use when**: Combining multiple PDFs into one document.

---

### split — Split Pages
```bash
zpdf split <file.pdf> [--pages 1,3-5] [-o <out.pdf|out-dir>] [--password <pw>]
```

**Flags**:
- `--pages <list>` — Page numbers/ranges to extract (comma-separated)
- `-o <path>` — Output file or directory
- `--password <pw>` — Decrypt with password

**Examples**:
```bash
# Extract specific pages
zpdf split document.pdf --pages 1,3-5,10 -o selected.pdf

# Split all pages to directory
zpdf split document.pdf -o pages/
```

**Use when**: Extracting page ranges or splitting into individual pages.

---

### optimize — Optimize/Compress
```bash
zpdf optimize <file.pdf> -o <out.pdf> [--no-compress] [--password <pw>]
       [--max-image-dim N] [--encrypt aes256|rc4 --user-password <pw> --owner-password <pw>]
```

**Flags**:
- `-o <path>` — Output file (required)
- `--no-compress` — Skip compression (faster but larger)
- `--max-image-dim <N>` — Downsample images to max dimension
- `--password <pw>` — Decrypt input with password
- `--encrypt aes256|rc4` — Encrypt output
- `--user-password <pw>` — User password for encrypted output
- `--owner-password <pw>` — Owner password for encrypted output

**Examples**:
```bash
# Basic optimization (compress)
zpdf optimize large.pdf -o small.pdf

# Downsample images
zpdf optimize photos.pdf -o web.pdf --max-image-dim 1024

# Encrypt output
zpdf optimize doc.pdf -o secure.pdf \
  --encrypt aes256 \
  --user-password "read123" \
  --owner-password "admin456"

# Decrypt and re-optimize
zpdf optimize encrypted.pdf -o clean.pdf --password "read123"
```

**Use when**: Reducing file size, encrypting/decrypting documents.

---

### annotate — Add Annotations
```bash
zpdf annotate <file.pdf> -p <page> --kind <type>
       [--rect X0,Y0,X1,Y1] [--at X,Y] [--to X,Y] [--text STR]
       [--color R,G,B] [--interior R,G,B] [--width W] [--size S] [--icon NAME]
       -o <out.pdf>
```

**Annotation types**: `highlight`, `underline`, `strikeout`, `squiggly`, `note`, `freetext`, `square`, `circle`, `line`

**Flags**:
- `-p <N>` — Page number (required)
- `--kind <type>` — Annotation type (required)
- `--rect X0,Y0,X1,Y1` — Rectangle coordinates (for highlights, shapes, freetext)
- `--at X,Y` — Point coordinates (for notes, line start)
- `--to X,Y` — End point (for lines)
- `--text <str>` — Text content (for notes, freetext)
- `--color R,G,B` — Stroke/highlight color (0-255)
- `--interior R,G,B` — Fill color (for shapes)
- `--width <N>` — Line width (for shapes, lines)
- `--size <N>` — Font size (for freetext)
- `--icon <name>` — Icon name (for notes: Comment, Key, Note, Help, etc.)
- `-o <path>` — Output file (required)

**Examples**:
```bash
# Highlight text
zpdf annotate doc.pdf -p 1 --kind highlight \
  --rect 100,200,300,220 --color 255,255,0 -o highlighted.pdf

# Add sticky note
zpdf annotate doc.pdf -p 2 --kind note \
  --at 100,500 --text "Review this section" --icon Comment -o noted.pdf

# Add text box
zpdf annotate doc.pdf -p 3 --kind freetext \
  --rect 50,600,200,650 --text "Important!" \
  --size 14 --color 255,0,0 -o marked.pdf

# Draw circle
zpdf annotate doc.pdf -p 1 --kind circle \
  --rect 200,300,250,350 --color 0,0,255 --width 2 -o circled.pdf

# Draw line
zpdf annotate doc.pdf -p 1 --kind line \
  --at 100,100 --to 300,200 --color 255,0,0 --width 3 -o lined.pdf
```

**Use when**: Marking up PDFs with highlights, comments, shapes.

---

### redact — Redact Content
```bash
zpdf redact <file.pdf> -p <page> --rect X0,Y0,X1,Y1 [--rect ...]
       [--fill R,G,B | --no-fill] [--password <pw>] -o <out.pdf>
```

**Flags**:
- `-p <N>` — Page number (required)
- `--rect X0,Y0,X1,Y1` — Rectangle to redact (repeat for multiple)
- `--fill R,G,B` — Fill color (default: black)
- `--no-fill` — Remove content without fill
- `--password <pw>` — Decrypt with password
- `-o <path>` — Output file (required)

**Examples**:
```bash
# Redact single area
zpdf redact sensitive.pdf -p 1 --rect 100,200,300,250 -o redacted.pdf

# Redact multiple areas
zpdf redact doc.pdf -p 2 \
  --rect 50,100,200,120 \
  --rect 50,300,200,320 \
  --fill 0,0,0 -o redacted.pdf
```

**Use when**: Permanently removing sensitive information from PDFs.

---

### sign — Digital Signature
```bash
zpdf sign <file.pdf> --key <key.p8.der> --cert <cert.der>
       [--name S] [--reason S] [--location S]
       [--subfilter pkcs7|cades] [--appearance] [--appearance-rect X0,Y0,X1,Y1]
       [--tsa <url>] [--cert-chain <pem> ...] [--crl <der> ...]
       [--password <pw>] -o <out.pdf>
```

**Flags**:
- `--key <path>` — PKCS#8 DER private key (RSA or ECDSA P-256)
- `--cert <path>` — X.509 DER certificate (matching public key)
- `--name <str>` — Signer name
- `--reason <str>` — Signing reason
- `--location <str>` — Signing location
- `--subfilter pkcs7|cades` — Signature encoding: `pkcs7` (`adbe.pkcs7.detached`, default) or `cades` (`ETSI.CAdES.detached`, PAdES)
- `--appearance` — Emit a visible signature widget (use with `--appearance-rect`)
- `--appearance-rect X0,Y0,X1,Y1` — On-page rectangle for the visible signature (required with `--appearance`)
- `--tsa <url>` — RFC 3161 timestamp authority URL; embeds a timestamp token in the CMS `unsignedAttrs` (requires the `timestamp` feature at build time)
- `--cert-chain <pem>` — Extra certificate (PEM) to embed in the CMS / DSS (repeat for a chain)
- `--crl <der>` — CRL (DER) to embed for offline revocation info (repeat)
- `--password <pw>` — Decrypt input with password
- `-o <path>` — Output file (required)

**Examples**:
```bash
# Basic detached signature
zpdf sign contract.pdf \
  --key private_key.p8.der \
  --cert certificate.der \
  --name "John Doe" \
  --reason "Contract approval" \
  --location "New York" \
  -o signed_contract.pdf

# PAdES (ETSI.CAdES.detached) with a visible appearance
zpdf sign contract.pdf \
  --key key.p8.der --cert cert.der \
  --subfilter cades \
  --appearance --appearance-rect 100,600,300,680 \
  --name "John Doe" -o signed_pades.pdf

# Timestamped + LTV (DSS) with an embedded cert chain and CRL
zpdf sign contract.pdf \
  --key key.p8.der --cert cert.der \
  --tsa http://timestamp.digicert.com \
  --cert-chain chain.pem \
  --crl revoked.crl \
  -o signed_ltv.pdf
```

**Use when**: Adding digital signatures for authenticity and non-repudiation; PAdES/LTV for long-term-validatable signatures.

---

### pages — Page Operations
```bash
zpdf pages <file.pdf> [--rotate PAGES:DEG] [--delete LIST] [--order LIST] -o <out.pdf>
```

**Flags**:
- `--rotate <pages>:<deg>` — Rotate pages (90, 180, 270)
- `--delete <list>` — Delete page numbers
- `--order <list>` — Reorder pages
- `-o <path>` — Output file (required)

**Examples**:
```bash
# Rotate pages
zpdf pages doc.pdf --rotate 1-5:90 -o rotated.pdf

# Delete pages
zpdf pages doc.pdf --delete 3,7,9 -o trimmed.pdf

# Reorder pages
zpdf pages doc.pdf --order 3,1,2,4-10 -o reordered.pdf

# Combine operations
zpdf pages doc.pdf --rotate 1-5:90 --delete 7 --order 1,3,2,4-10 -o modified.pdf
```

**Use when**: Rotating, deleting, or reordering pages.

---

### set-meta — Set Metadata
```bash
zpdf set-meta <file.pdf> [--title S] [--author S] [--subject S] [--keywords S] -o <out.pdf>
```

**Flags**:
- `--title <str>` — Document title
- `--author <str>` — Author name
- `--subject <str>` — Subject/description
- `--keywords <str>` — Keywords (comma-separated)
- `-o <path>` — Output file (required)

**Example**:
```bash
zpdf set-meta doc.pdf \
  --title "Annual Report 2026" \
  --author "Example Corp" \
  --subject "Financial results" \
  --keywords "finance,2026,annual" \
  -o updated.pdf
```

**Use when**: Updating document metadata.

---

### stamp — Add Text Stamp
```bash
zpdf stamp <file.pdf> -p <page> --text STR --at X,Y
       [--font F] [--size N] [--color R,G,B] -o <out.pdf>
```

**Flags**:
- `-p <N>` — Page number (required)
- `--text <str>` — Stamp text (required)
- `--at X,Y` — Position coordinates (required)
- `--font <name>` — Font name (Helvetica, Times, Courier)
- `--size <N>` — Font size (default: 12)
- `--color R,G,B` — Text color (default: black)
- `-o <path>` — Output file (required)

**Examples**:
```bash
# Simple watermark
zpdf stamp doc.pdf -p 1 --text "DRAFT" --at 300,700 \
  --size 48 --color 255,0,0 -o draft.pdf

# Confidential stamp
zpdf stamp report.pdf -p 1 --text "CONFIDENTIAL" --at 250,50 \
  --font Helvetica --size 24 --color 128,0,0 -o confidential.pdf
```

**Use when**: Adding watermarks, stamps, or text overlays.

---

### tag — Add Tag Structure (Accessibility)
```bash
zpdf tag <file.pdf> [--password <pw>] -o <out.pdf>
```

Adds a coarse-grained tag structure to an **untagged** PDF for accessibility
(PDF/UA): each page's content is wrapped in a `/Part` marked-content sequence,
a `/StructTreeRoot` + `/ParentTree` + `/MarkInfo /Marked true` is emitted, and
one `/Part` element per page carries the page's extracted text as `/Alt`. No-op
when the document is already tagged.

**Flags**:
- `--password <pw>` — Decrypt input with password
- `-o <path>` — Output file (required)

**Example**:
```bash
# Tag an untagged PDF for screen-reader accessibility
zpdf tag legacy_scan.pdf -o legacy_tagged.pdf

# Verify with the PDF/UA validator
zpdf validate legacy_tagged.pdf --profile pdfua-1
```

**Use when**: Making an existing untagged PDF accessible. The tags are
page-level only (`/Part`); for fine-grained paragraph/heading/table semantics,
author the PDF with a tagged-aware tool instead.

---

## Debugging Commands

### dump — Dump Raw Object
```bash
zpdf dump <file.pdf> <obj_num> <gen_num> [--password <pw>]
```

**Output**: Raw PDF object data.

**Use when**: Debugging PDF structure, investigating object contents.

---

### debug-stream — Debug Content Stream
```bash
zpdf debug-stream <file.pdf> <obj_num> <gen_num>
```

**Output**: Decoded and formatted content stream operators.

**Use when**: Debugging rendering issues, understanding content streams.

---

## Password Handling

Most commands support `--password <pw>` for encrypted PDFs:

```bash
# Reading encrypted PDFs
zpdf info secure.pdf --password secret123
zpdf text secure.pdf --password secret123 -p 1
zpdf render secure.pdf -p 1 -o out.png --dpi 150 --password secret123
zpdf search secure.pdf "keyword" --password secret123

# Modifying encrypted PDFs
zpdf split secure.pdf --pages 1-5 -o extracted.pdf --password secret123
zpdf redact secure.pdf -p 1 --rect 100,200,300,250 -o redacted.pdf --password secret123
```

**Important**: 
- Always provide password via `--password` flag in automation
- Interactive password prompts don't work in scripts
- Wrong password returns error immediately
- Use `optimize --encrypt` to create encrypted PDFs

---

## Common Patterns

### Pattern: Preview then Extract
```bash
# Step 1: Check document
zpdf info document.pdf

# Step 2: Preview first page
zpdf render document.pdf -p 1 -o preview.png --dpi 150

# Step 3: Extract needed pages
zpdf text document.pdf -p 3 > page3.txt
```

### Pattern: Search then Read
```bash
# Step 1: Find relevant pages
zpdf search report.pdf "budget allocation" | grep "Page" | awk '{print $2}'

# Step 2: Extract those pages
zpdf text report.pdf --pages 5,12,18 > relevant.txt
```

### Pattern: Batch Image Extraction
```bash
# Get page count
pages=$(zpdf info doc.pdf | grep "Pages:" | awk '{print $2}')

# Render all pages
for p in $(seq 1 $pages); do
  zpdf render doc.pdf -p $p -o page_$p.png --dpi 150
done
```

### Pattern: Table Extraction
```bash
# Extract tables from multiple pages
for p in 5 7 9 11; do
  echo "=== Page $p ===" >> tables.txt
  zpdf tables financial.pdf -p $p >> tables.txt
done
```

### Pattern: Form Processing Pipeline
```bash
# 1. List fields
zpdf forms template.pdf

# 2. Fill form
zpdf fill template.pdf --set name="John Doe" --set email="john@example.com" -o filled.pdf

# 3. Sign
zpdf sign filled.pdf --key key.p8.der --cert cert.der --name "John Doe" -o final.pdf
```

### Pattern: Document Optimization Pipeline
```bash
# 1. Optimize and compress
zpdf optimize large.pdf -o compressed.pdf --max-image-dim 1024

# 2. Encrypt
zpdf optimize compressed.pdf -o secure.pdf --encrypt aes256 \
  --user-password "read123" --owner-password "admin456"

# 3. Add watermark
zpdf stamp secure.pdf -p 1 --text "CONFIDENTIAL" --at 300,50 \
  --size 36 --color 128,128,128 -o final.pdf
```

---

## Error Handling

Common errors and solutions:

| Error | Cause | Solution |
|-------|-------|----------|
| "File not found" | Path incorrect | Check file exists |
| "Invalid password" | Wrong password | Verify password |
| "Page out of range" | Page number > total | Check `zpdf info` first |
| "Corrupted PDF" | Malformed file | Try `zpdf info` to diagnose |
| "Permission denied" | File locked | Close PDF viewer/reader |
| "Missing required flag" | Required arg omitted | Check command syntax |

---

## Output Formats

| Command | Output Format | Encoding |
|---------|---------------|----------|
| text | Plain text | UTF-8 |
| render | PNG image | 24-bit RGB |
| convert | Text/MD/HTML | UTF-8 |
| export-pptx | PowerPoint | OOXML |
| tables | Structured text/CSV | UTF-8 |
| forms | Key-value pairs | UTF-8 |
| outline | Hierarchical text | UTF-8 |
| links | URL list | UTF-8 |
| signatures | Status report | UTF-8 |

---

## Performance Tips

1. **DPI Choice**:
   - 72 DPI: ~20-40 KB per page, 0.5s render
   - 150 DPI: ~50-150 KB per page, 1-2s render
   - 300 DPI: ~200-500 KB per page, 3-5s render

2. **Backend Selection**:
   - `cpu`: Default, works everywhere
   - `wgpu`: 2-3× faster for complex pages, requires GPU

3. **Selective Extraction**:
   - Use `zpdf search` to find pages before extracting
   - Use `zpdf outline` to understand structure
   - Extract page ranges, not entire documents

4. **Caching**:
   - zpdf parses on every invocation
   - Reuse extracted text/images when possible
   - Don't re-render same page multiple times

5. **Batch Operations**:
   - Use shell loops for multiple files
   - Parallelize with `xargs -P` or GNU parallel
   - Extract attachments once, not per-file
