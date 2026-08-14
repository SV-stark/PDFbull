# zpdf Reading Reference (Minimal)

Ultra-lightweight reference for reading PDFs only. Use this when the task is just extracting content.

## Four Essential Commands

```bash
# 1. Check PDF metadata (ALWAYS first)
zpdf info <file.pdf> [--password <pw>]

# 2. Extract text
zpdf text <file.pdf> -p <page> [--password <pw>]
zpdf text <file.pdf> --all [--password <pw>]

# 3. Render to image
zpdf render <file.pdf> -p <page> -o output.png --dpi 150 [--password <pw>]

# 4. Search content
zpdf search <file.pdf> "keyword" [--password <pw>]
```

## Decision: Text vs Image

**Extract text** when:
- Text-heavy documents (reports, articles, contracts)
- Layout doesn't matter
- Want to summarize or search content

**Render to image** when:
- Visual elements (diagrams, charts, tables, forms)
- Layout/formatting is important
- Need vision analysis

## Quick Workflow

```bash
# Standard flow
zpdf info doc.pdf                        # Check pages (e.g., 25 pages)
zpdf search doc.pdf "keyword"            # Find relevant pages (e.g., p.5, 12)
zpdf text doc.pdf -p 5 -p 12             # Extract those pages

# Large document
zpdf info large.pdf                      # 150 pages
zpdf search large.pdf "section 3"        # Page 45
zpdf text large.pdf -p 45 > section3.txt

# Visual content
zpdf info slides.pdf                     # 10 slides
zpdf render slides.pdf -p 1 -o slide1.png --dpi 150
zpdf render slides.pdf -p 5 -o slide5.png --dpi 150
```

## Key Rules

- Always run `zpdf info` first
- DPI: 150 (default), 300 (high detail)
- Password: `--password <pw>` (required for encrypted PDFs)
- Save images to `/tmp/` for auto-cleanup
- Search before bulk extraction on large docs
- Don't render >10 pages without user confirmation

**For editing, conversion, or advanced features**, see other reference files.