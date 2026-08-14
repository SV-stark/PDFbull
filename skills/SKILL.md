---
name: zpdf
description: Complete PDF toolkit. Intelligently loads minimal reading reference for simple tasks, full references for editing/conversion. Use zpdf CLI for all PDF operations.
version: 3.1.0
references:
  - read-minimal.md
  - quick-reference.md
  - reading-commands.md
  - analysis-commands.md
  - editing-commands.md
---

# zpdf: Complete PDF Toolkit

Use zpdf CLI for all PDF operations. This skill intelligently loads references based on task type.

## Reference Loading Strategy

**For reading-only tasks** (90% of PDF work):
- Load **[read-minimal.md](references/read-minimal.md)**
- Covers: `info`, `text`, `render`, `search`
- Minimal context overhead

**For editing/conversion tasks**:
- Load additional references as needed:
  - **[quick-reference.md](references/quick-reference.md)** - Fast lookup
  - **[editing-commands.md](references/editing-commands.md)** - Form filling, merging, annotations, etc.
  - **[analysis-commands.md](references/analysis-commands.md)** - Tables, forms, signatures, etc.
  - **[reading-commands.md](references/reading-commands.md)** - Detailed reading command syntax

## Quick Start

### Reading PDFs (Load read-minimal.md)

```bash
# Check metadata
zpdf info <file.pdf>

# Extract text
zpdf text <file.pdf> -p <page>
zpdf text <file.pdf> --all

# Render to image
zpdf render <file.pdf> -p <page> -o output.png --dpi 150

# Search
zpdf search <file.pdf> "keyword"
```

### Editing PDFs (Load editing-commands.md)

```bash
# Fill form
zpdf fill form.pdf --set name="John" -o filled.pdf

# Merge/split
zpdf merge a.pdf b.pdf -o combined.pdf
zpdf split doc.pdf --pages 1-5 -o part.pdf

# Annotate
zpdf annotate doc.pdf -p 1 --kind highlight --rect 100,200,300,220 -o marked.pdf

# Sign
zpdf sign doc.pdf --key key.p8.der --cert cert.der -o signed.pdf

# Optimize
zpdf optimize large.pdf -o small.pdf --max-image-dim 1024

# Add accessibility tags (for untagged PDFs)
zpdf tag doc.pdf -o tagged.pdf
```

### Converting PDFs (Load reading-commands.md)

```bash
# Convert to Markdown/HTML
zpdf convert doc.pdf -o doc.md --all
zpdf convert doc.pdf -o doc.html --all

# Export to PowerPoint
zpdf export-pptx slides.pdf -o presentation.pptx --all
```

### Analysis (Load analysis-commands.md)

```bash
# Extract tables
zpdf tables financial.pdf -p 5 --csv

# Get form fields
zpdf forms application.pdf

# Extract attachments
zpdf attachments invoice.pdf --extract all

# Validate conformance (PDF/A or PDF/UA)
zpdf validate doc.pdf --profile pdfa-1b
zpdf validate doc.pdf --profile pdfua-1
```

## Decision Flow

1. **Determine task type**:
   - **Reading only?** → Load `read-minimal.md` (saves ~90% context)
   - **Editing needed?** → Load `editing-commands.md`
   - **Converting?** → Load `reading-commands.md`
   - **Structured data?** → Load `analysis-commands.md`

2. **For reading tasks**:
   - Text-heavy → `zpdf text`
   - Visual/layout → `zpdf render`
   - Search first on large docs

3. **Always start with**: `zpdf info <file.pdf>`

## Key Rules

1. **Load minimal reference by default** - most PDF tasks are reading
2. **Always run `zpdf info` first** - reveals pages/encryption
3. **Password handling**: Use `--password <pw>` flag
4. **DPI defaults**: 150 for readable, 300 for detail
5. **Search before bulk extraction** on large documents
6. **Page selection**: `-p 1` or `--pages 1,3-5` or `--all`

## All Available Commands

**Reading (6)**: info, text, render, search, convert, export-pptx  
**Analysis (9)**: tables, forms, outline, links, struct, signatures, attachments, validate, compare  
**Editing (10)**: fill, merge, split, optimize, annotate, redact, sign, pages, set-meta, stamp, tag  
**Debug (2)**: dump, debug-stream

**Total**: 27 commands

## Reference Files

Load these based on task type:

- **[read-minimal.md](references/read-minimal.md)** — Reading only (info, text, render, search) ~400 tokens ← **Load this first**
- **[quick-reference.md](references/quick-reference.md)** — Fast lookup for all commands ~1,500 tokens
- **[reading-commands.md](references/reading-commands.md)** — Detailed reading/conversion commands ~1,800 tokens
- **[analysis-commands.md](references/analysis-commands.md)** — Tables, forms, links, signatures ~1,500 tokens
- **[editing-commands.md](references/editing-commands.md)** — Fill, merge, annotate, sign, optimize ~2,500 tokens

**Token savings**: Using read-minimal.md for reading tasks saves ~8,000 tokens vs loading all references.