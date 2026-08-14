# Reading Commands Reference

Commands for extracting content from PDFs.

## info — PDF Metadata
```bash
zpdf info <file.pdf> [--password <pw>]
```

**Output**: Page count, PDF version, encryption status, metadata, page dimensions

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

## text — Extract Text
```bash
zpdf text <file.pdf> [-p <page>] [--all] [--struct] [--password <pw>]
```

**Flags**:
- `-p <N>` — Extract single page (1-indexed)
- `--all` — Extract all pages
- `--struct` — Preserve structural markup
- `--password <pw>` — Decrypt with password

**Examples**:
```bash
zpdf text doc.pdf -p 5                        # Single page
zpdf text doc.pdf --all                       # All pages
zpdf text secure.pdf --password secret123 --all  # Encrypted PDF
```

**Output**: Plain text, UTF-8 encoded, preserves line breaks.

---

## render — Render to Image
```bash
zpdf render <file.pdf> -p <page> -o <output.png> [--dpi <dpi>] [--backend cpu|wgpu] [--stats] [--password <pw>]
```

**Flags**:
- `-p <N>` — Page number (required)
- `-o <path>` — Output PNG path (required)
- `--dpi <N>` — Resolution: 72 (preview), 150 (default), 300 (high quality)
- `--backend cpu|wgpu` — cpu (default) or wgpu (2-3× faster, needs GPU)
- `--stats` — Print rendering performance stats
- `--password <pw>` — Decrypt with password

**Examples**:
```bash
zpdf render doc.pdf -p 1 -o page1.png --dpi 150
zpdf render diagram.pdf -p 3 -o detail.png --dpi 300
zpdf render complex.pdf -p 10 -o out.png --dpi 150 --backend wgpu
```

**Output**: PNG image, 24-bit RGB.

---

## search — Full-Text Search
```bash
zpdf search <file.pdf> <query> [-p <page>] [--case-sensitive] [--password <pw>]
```

**Flags**:
- `-p <N>` — Search only specified page
- `--case-sensitive` — Match case exactly
- `--password <pw>` — Decrypt with password

**Example**:
```bash
$ zpdf search report.pdf "quarterly revenue"
Page 3: ...showing quarterly revenue growth of 15%...
Page 12: ...projected quarterly revenue for Q3...
Page 18: ...quarterly revenue comparison chart...
```

---

## convert — Convert to Text/Markdown/HTML
```bash
zpdf convert <file.pdf> -o <output> [--mode text|rich] [--format txt|md|html] [-p <page>|--pages <list>|--all] [--struct] [--images-dir <dir>] [--password <pw>]
```

**Flags**:
- `-o <path>` — Output file (required)
- `--mode text|rich` — Text only or rich with formatting
- `--format txt|md|html` — Output format (auto-detected from extension)
- `-p <N>` or `--pages 1,3-5` or `--all` — Page selection
- `--images-dir <dir>` — Extract images to directory (HTML mode)
- `--password <pw>` — Decrypt with password

**Examples**:
```bash
zpdf convert paper.pdf -o paper.md --format md --all
zpdf convert doc.pdf -o doc.html --format html --images-dir ./img --all
zpdf convert report.pdf -o summary.txt --pages 1,5-10 --format txt
```

---

## export-pptx — Export to PowerPoint
```bash
zpdf export-pptx <file.pdf> -o <output.pptx> [-p <page>|--pages <list>|--all] [--password <pw>]
```

**Flags**:
- `-o <path>` — Output PowerPoint file (required)
- `-p <N>` or `--pages 1,3-5` or `--all` — Page selection
- `--password <pw>` — Decrypt with password

**Examples**:
```bash
zpdf export-pptx slides.pdf -o presentation.pptx --all
zpdf export-pptx deck.pdf -o summary.pptx --pages 1,5-10,20
```

---

## Common Patterns

### Preview then Extract
```bash
zpdf info document.pdf
zpdf render document.pdf -p 1 -o preview.png --dpi 150
zpdf text document.pdf -p 3 > page3.txt
```

### Search then Read
```bash
zpdf search report.pdf "budget allocation" | grep "Page" | awk '{print $2}'
zpdf text report.pdf --pages 5,12,18 > relevant.txt
```

### Batch Image Extraction
```bash
pages=$(zpdf info doc.pdf | grep "Pages:" | awk '{print $2}')
for p in $(seq 1 $pages); do
  zpdf render doc.pdf -p $p -o page_$p.png --dpi 150
done
```
