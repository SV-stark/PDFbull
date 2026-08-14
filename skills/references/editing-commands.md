# Editing Commands Reference

Commands for modifying, combining, and optimizing PDFs.

## fill — Fill Form Fields
```bash
zpdf fill <file.pdf> --set NAME=VALUE [--set ...] [--list] -o <out.pdf>
```

**Flags**:
- `--set name=value` — Set field value (repeat for multiple fields)
- `--list` — List available field names
- `-o <path>` — Output file (required)

**Examples**:
```bash
zpdf fill form.pdf --list
zpdf fill application.pdf \
  --set name="John Doe" \
  --set email="john@example.com" \
  --set agree=true \
  -o filled.pdf
```

---

## merge — Merge PDFs
```bash
zpdf merge <a.pdf> <b.pdf> [more.pdf ...] -o <out.pdf>
```

**Example**:
```bash
zpdf merge intro.pdf body.pdf appendix.pdf -o complete.pdf
```

---

## split — Split Pages
```bash
zpdf split <file.pdf> [--pages 1,3-5] [-o <out.pdf|out-dir>] [--password <pw>]
```

**Flags**:
- `--pages <list>` — Page numbers/ranges to extract
- `-o <path>` — Output file or directory
- `--password <pw>` — Decrypt with password

**Examples**:
```bash
zpdf split document.pdf --pages 1,3-5,10 -o selected.pdf
zpdf split document.pdf -o pages/  # Split all pages to directory
```

---

## optimize — Optimize/Compress/Encrypt
```bash
zpdf optimize <file.pdf> -o <out.pdf> [--no-compress] [--password <pw>]
       [--max-image-dim N] [--encrypt aes256|rc4 --user-password <pw> --owner-password <pw>]
```

**Flags**:
- `-o <path>` — Output file (required)
- `--no-compress` — Skip compression
- `--max-image-dim <N>` — Downsample images to max dimension
- `--password <pw>` — Decrypt input with password
- `--encrypt aes256|rc4` — Encrypt output
- `--user-password <pw>` — User password for encrypted output
- `--owner-password <pw>` — Owner password for encrypted output

**Examples**:
```bash
# Compress
zpdf optimize large.pdf -o small.pdf

# Downsample images
zpdf optimize photos.pdf -o web.pdf --max-image-dim 1024

# Encrypt
zpdf optimize doc.pdf -o secure.pdf \
  --encrypt aes256 \
  --user-password "read123" \
  --owner-password "admin456"

# Decrypt and re-optimize
zpdf optimize encrypted.pdf -o clean.pdf --password "read123"
```

---

## annotate — Add Annotations
```bash
zpdf annotate <file.pdf> -p <page> --kind <type>
       [--rect X0,Y0,X1,Y1] [--at X,Y] [--to X,Y] [--text STR]
       [--color R,G,B] [--interior R,G,B] [--width W] [--size S] [--icon NAME]
       -o <out.pdf>
```

**Annotation types**: `highlight`, `underline`, `strikeout`, `squiggly`, `note`, `freetext`, `square`, `circle`, `line`

**Key flags**:
- `-p <N>` — Page number (required)
- `--kind <type>` — Annotation type (required)
- `--rect X0,Y0,X1,Y1` — Rectangle (for highlights, shapes, freetext)
- `--at X,Y` — Point (for notes, line start)
- `--to X,Y` — End point (for lines)
- `--text <str>` — Text content (for notes, freetext)
- `--color R,G,B` — Stroke/highlight color (0-255)
- `--icon <name>` — Icon for notes (Comment, Key, Note, Help)
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

# Draw shapes
zpdf annotate doc.pdf -p 1 --kind circle \
  --rect 200,300,250,350 --color 0,0,255 --width 2 -o circled.pdf
```

---

## redact — Redact Content
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

---

## sign — Digital Signature
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

---

## tag — Add Tag Structure (Accessibility)
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

## pages — Page Operations
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
zpdf pages doc.pdf --rotate 1-5:90 -o rotated.pdf
zpdf pages doc.pdf --delete 3,7,9 -o trimmed.pdf
zpdf pages doc.pdf --order 3,1,2,4-10 -o reordered.pdf

# Combine operations
zpdf pages doc.pdf --rotate 1-5:90 --delete 7 --order 1,3,2,4-10 -o modified.pdf
```

---

## set-meta — Set Metadata
```bash
zpdf set-meta <file.pdf> [--title S] [--author S] [--subject S] [--keywords S] -o <out.pdf>
```

**Example**:
```bash
zpdf set-meta doc.pdf \
  --title "Annual Report 2026" \
  --author "Example Corp" \
  --subject "Financial results" \
  --keywords "finance,2026,annual" \
  -o updated.pdf
```

---

## stamp — Add Text Stamp
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
# Watermark
zpdf stamp doc.pdf -p 1 --text "DRAFT" --at 300,700 \
  --size 48 --color 255,0,0 -o draft.pdf

# Confidential stamp
zpdf stamp report.pdf -p 1 --text "CONFIDENTIAL" --at 250,50 \
  --font Helvetica --size 24 --color 128,0,0 -o confidential.pdf
```

---

## Common Workflows

### Form Processing Pipeline
```bash
zpdf forms template.pdf                      # List fields
zpdf fill template.pdf --set name="John Doe" --set email="john@example.com" -o filled.pdf
zpdf sign filled.pdf --key key.p8.der --cert cert.der --name "John Doe" -o final.pdf
```

### Document Optimization Pipeline
```bash
zpdf optimize large.pdf -o compressed.pdf --max-image-dim 1024
zpdf optimize compressed.pdf -o secure.pdf --encrypt aes256 \
  --user-password "read123" --owner-password "admin456"
zpdf stamp secure.pdf -p 1 --text "CONFIDENTIAL" --at 300,50 \
  --size 36 --color 128,128,128 -o final.pdf
```

### Multi-Document Assembly
```bash
# Extract pages from source documents
zpdf split report1.pdf --pages 1-3 -o intro.pdf
zpdf split report2.pdf --pages 5-10 -o body.pdf
zpdf split report3.pdf --pages 1 -o conclusion.pdf

# Merge with custom order
zpdf merge intro.pdf body.pdf conclusion.pdf -o final_report.pdf

# Add metadata and stamp
zpdf set-meta final_report.pdf --title "Consolidated Report" \
  --author "Team" -o final_report_meta.pdf
zpdf stamp final_report_meta.pdf -p 1 --text "FINAL" --at 500,50 -o final.pdf
```

### Redaction Workflow
```bash
# Redact sensitive areas
zpdf redact personnel.pdf -p 1 \
  --rect 100,200,300,220 \
  --rect 100,400,300,420 \
  --fill 0,0,0 -o redacted.pdf

# Remove metadata
zpdf set-meta redacted.pdf --title "" --author "" --subject "" --keywords "" -o clean.pdf

# Flatten to prevent reverse engineering
zpdf optimize clean.pdf -o final_redacted.pdf
```
