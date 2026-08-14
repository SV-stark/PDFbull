# Quick Reference Card

Fast lookup for common zpdf operations.

## Most Common Commands

```bash
# Check PDF info (ALWAYS run first)
zpdf info <file.pdf>

# Extract text
zpdf text <file.pdf> -p <page>
zpdf text <file.pdf> --all

# Render page to image
zpdf render <file.pdf> -p <page> -o output.png --dpi 150

# Search content
zpdf search <file.pdf> "keyword"

# Extract tables
zpdf tables <file.pdf> -p <page>

# Fill form
zpdf fill form.pdf --set field=value -o filled.pdf

# Merge PDFs
zpdf merge a.pdf b.pdf c.pdf -o combined.pdf

# Split pages
zpdf split doc.pdf --pages 1,3-5 -o selected.pdf

# Optimize/compress
zpdf optimize large.pdf -o small.pdf --max-image-dim 1024

# Encrypt
zpdf optimize doc.pdf -o secure.pdf --encrypt aes256 \
  --user-password "user123" --owner-password "admin456"
```

## By Task Type

### Reading PDFs
```bash
zpdf info doc.pdf                          # Metadata
zpdf text doc.pdf -p 1                     # Extract text
zpdf render doc.pdf -p 1 -o page1.png --dpi 150  # Render image
zpdf search doc.pdf "keyword"              # Search
zpdf convert doc.pdf -o doc.md --all       # Convert to Markdown
zpdf export-pptx slides.pdf -o out.pptx --all    # Export to PowerPoint
```

### Analyzing PDFs
```bash
zpdf tables doc.pdf -p 1                   # Extract tables
zpdf forms doc.pdf                         # List form fields
zpdf outline doc.pdf                       # Document TOC
zpdf links doc.pdf                         # Extract hyperlinks
zpdf signatures doc.pdf                    # Verify signatures
zpdf attachments doc.pdf                   # List embedded files
zpdf validate doc.pdf --profile pdfa-1b   # PDF/A archival check
zpdf validate doc.pdf --profile pdfua-1    # PDF/UA accessibility check
```

### Modifying PDFs
```bash
zpdf fill form.pdf --set name="John" -o filled.pdf       # Fill form
zpdf merge a.pdf b.pdf -o combined.pdf                   # Merge
zpdf split doc.pdf --pages 1-5 -o part1.pdf             # Split
zpdf pages doc.pdf --rotate 1-3:90 -o rotated.pdf      # Rotate pages
zpdf annotate doc.pdf -p 1 --kind highlight --rect 100,200,300,220 -o marked.pdf
zpdf redact doc.pdf -p 1 --rect 100,200,300,250 -o redacted.pdf
zpdf sign doc.pdf --key key.p8.der --cert cert.der -o signed.pdf
zpdf stamp doc.pdf -p 1 --text "DRAFT" --at 300,700 -o stamped.pdf
zpdf tag doc.pdf -o tagged.pdf                           # Add accessibility tags
```

### Optimizing PDFs
```bash
zpdf optimize doc.pdf -o small.pdf                      # Compress
zpdf optimize doc.pdf -o web.pdf --max-image-dim 1024  # Downsample images
zpdf optimize doc.pdf -o secure.pdf --encrypt aes256 \
  --user-password "read" --owner-password "admin"      # Encrypt
```

## Password Handling

All commands support `--password <pw>` for encrypted PDFs:

```bash
zpdf info secure.pdf --password secret123
zpdf text secure.pdf --password secret123 -p 1
zpdf render secure.pdf -p 1 -o out.png --dpi 150 --password secret123
zpdf split secure.pdf --pages 1-5 -o part.pdf --password secret123
```

## DPI Guidelines

- **72 DPI**: Quick preview (~20-40 KB/page, 0.5s)
- **150 DPI**: Default quality (~50-150 KB/page, 1-2s) ← **Recommended**
- **300 DPI**: High detail/OCR (~200-500 KB/page, 3-5s)

## Common Flags

- `-p <N>` — Page number (1-indexed)
- `--all` — All pages
- `--pages 1,3-5,10` — Page list/ranges
- `-o <path>` — Output file
- `--password <pw>` — Decrypt with password
- `--dpi <N>` — Resolution for rendering
- `--backend cpu|wgpu` — Rendering backend

## Typical Workflows

### Extract specific content
```bash
zpdf info doc.pdf                                    # Check pages
zpdf search doc.pdf "section 3"                      # Find pages
zpdf text doc.pdf --pages 5,7,9 > extracted.txt     # Extract those pages
```

### Process form
```bash
zpdf forms template.pdf                              # List fields
zpdf fill template.pdf --set name="John" -o filled.pdf
zpdf sign filled.pdf --key key.p8.der --cert cert.der -o final.pdf
```

### Optimize for web
```bash
zpdf optimize large.pdf -o compressed.pdf --max-image-dim 1024
zpdf stamp compressed.pdf -p 1 --text "CONFIDENTIAL" --at 300,50 -o final.pdf
```

### Merge and reorganize
```bash
zpdf split report1.pdf --pages 1-3 -o intro.pdf
zpdf split report2.pdf --pages 5-10 -o body.pdf
zpdf merge intro.pdf body.pdf -o combined.pdf
zpdf pages combined.pdf --rotate 1:90 --delete 5 -o final.pdf
```

## Error Messages

| Error | Solution |
|-------|----------|
| "File not found" | Check file path |
| "Invalid password" | Verify password |
| "Page out of range" | Run `zpdf info` first |
| "Corrupted PDF" | Try `zpdf info` to diagnose |
| "Permission denied" | Close PDF viewer |

## Performance Tips

1. Use `zpdf search` to find relevant pages before bulk extraction
2. Use `--backend wgpu` for 2-3× faster rendering (needs GPU)
3. Extract page ranges, not entire documents
4. Reuse extracted text/images when possible
5. Use `--max-image-dim` to reduce file size

## Reference Files

- **[reading-commands.md](reading-commands.md)** — info, text, render, search, convert, export-pptx
- **[analysis-commands.md](analysis-commands.md)** — tables, forms, outline, links, signatures, attachments
- **[editing-commands.md](editing-commands.md)** — fill, merge, split, optimize, annotate, redact, sign
- **[zpdf-commands.md](zpdf-commands.md)** — Complete reference (all commands)
